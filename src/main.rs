use fibers_http_server::ServerBuilder;
use futures::Future;
use plumcast::node::{LocalNodeId, NodeBuilder, NodeId, SerialLocalNodeIdGenerator};
use plumcast::service::ServiceBuilder;
use sloggers::terminal::TerminalLoggerBuilder;
use sloggers::types::Severity;
use sloggers::Build;
use std::net::SocketAddr;
use structopt::StructOpt;
use trackable::result::MainResult;
use trackable::track;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long)]
    contact_server: Option<SocketAddr>,

    #[structopt(long, default_value = "7363")]
    http_port: u16,

    #[structopt(long, default_value = "7364")]
    rpc_port: u16,

    #[structopt(long, default_value = "info")]
    loglevel: Severity,
}

fn main() -> MainResult {
    let opt = Opt::from_args();
    let logger = track!(TerminalLoggerBuilder::new().level(opt.loglevel).build())?;

    let service = ServiceBuilder::new(([0, 0, 0, 0], opt.rpc_port).into())
        .logger(logger.clone())
        .finish(fibers_global::handle(), SerialLocalNodeIdGenerator::new());
    let mut node = NodeBuilder::new()
        .logger(logger.clone())
        .finish(service.handle());
    fibers_global::spawn(service.map_err(|e| panic!("{}", e)));

    if let Some(contact) = opt.contact_server {
        node.join(NodeId::new(contact, LocalNodeId::new(0)));
    }
    let node = plumtuna::study_list::StudyListNode::new(logger.clone(), node);
    let handle = node.handle();

    let mut builder = ServerBuilder::new(([0, 0, 0, 0], opt.http_port).into());
    builder.logger(logger);

    track!(builder.add_handler(plumtuna::http::GetStudies(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PostStudy(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::HeadStudy(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::GetStudy(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::GetStudyByName(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutStudyDirection(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PostTrial(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialState(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialParam(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialValue(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialIntermediateValue(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialSystemAttr(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialUserAttr(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::GetTrials(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::GetTrial(handle.clone())))?;

    let server = builder.finish(fibers_global::handle());
    fibers_global::spawn(server.map_err(|e| panic!("{}", e)));

    track!(fibers_global::execute(node))?;

    Ok(())
}
