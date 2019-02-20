use fibers_http_server::ServerBuilder;
use futures::Future;
use plumcast::node::{NodeBuilder, UnixtimeLocalNodeIdGenerator};
use plumcast::service::ServiceBuilder;
use plumtuna::contact::{ContactService, ContactServiceClient};
use plumtuna::global::GlobalNodeBuilder;
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use sloggers::Build;
use std::net::{SocketAddr, ToSocketAddrs};
use structopt::StructOpt;
use trackable::result::MainResult;
use trackable::{track, track_any_err};

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct Opt {
    #[structopt(long)]
    contact_server: Option<String>,

    #[structopt(long, default_value = "7363")]
    http_port: u16,

    #[structopt(long, default_value = "127.0.0.1:7364")]
    rpc_addr: SocketAddr,

    #[structopt(long, default_value = "info")]
    loglevel: Severity,

    #[structopt(long)]
    exit_if_stdin_close: bool,

    #[structopt(long, default_value = "1")]
    threads: usize,
}

fn main() -> MainResult {
    let opt = Opt::from_args();
    fibers_global::set_thread_count(opt.threads);
    let logger = track!(TerminalLoggerBuilder::new()
        .level(opt.loglevel)
        .destination(Destination::Stderr)
        .build())?;

    let mut service_builder = ServiceBuilder::new(opt.rpc_addr);
    let contact_service = ContactService::new(service_builder.rpc_server_builder_mut());

    let global_node_builder = GlobalNodeBuilder::new(service_builder.rpc_server_builder_mut());

    let service = service_builder
        .logger(logger.clone())
        .finish(fibers_global::handle(), UnixtimeLocalNodeIdGenerator::new());
    let mut node = NodeBuilder::new()
        .logger(logger.clone())
        .finish(service.handle());
    let plumcast_service_handle = service.handle();
    contact_service.handle().set_contact_node_id(node.id());

    let rpc_client_service_handle = service.rpc_client_service().handle();
    let contact_service_client = ContactServiceClient::new(service.rpc_client_service().handle());
    fibers_global::spawn(service.map_err(|e| panic!("{}", e)));
    fibers_global::spawn(contact_service.map_err(|e| panic!("{}", e)));

    if let Some(host) = opt.contact_server {
        let mut last_error = None;
        for addr in track_any_err!(host.to_socket_addrs())? {
            let result = fibers_global::execute(contact_service_client.get_contact_node_id(addr));
            match track!(result) {
                Err(e) => {
                    last_error = Some(e);
                }
                Ok(contact_node_id) => {
                    node.join(contact_node_id);
                    last_error = None;
                    break;
                }
            }
        }
        if let Some(e) = last_error.take() {
            Err(e)?;
        }
    }

    let global_node = global_node_builder.finish(
        logger.clone(),
        node,
        rpc_client_service_handle.clone(),
        plumcast_service_handle,
    );
    let handle = global_node.handle();

    let mut builder = ServerBuilder::new(([0, 0, 0, 0], opt.http_port).into());
    builder.logger(logger);

    track!(builder.add_handler(plumtuna::http::PostStudy(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::GetStudyByName(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::GetStudies(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::GetStudy(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutStudyDirection(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutStudySystemAttr(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutStudyUserAttr(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PostTrial(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialState(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialParam(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialValue(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialIntermediateValue(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialSystemAttr(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::PutTrialUserAttr(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::GetTrial(handle.clone())))?;
    track!(builder.add_handler(plumtuna::http::GetTrials(handle.clone())))?;

    let server = builder.finish(fibers_global::handle());
    fibers_global::spawn(server.map_err(|e| panic!("{}", e)));

    if opt.exit_if_stdin_close {
        std::thread::spawn(|| {
            use std::io::Read;
            let mut buf = [0; 1024];
            while let Ok(size) = std::io::stdin().lock().read(&mut buf) {
                if size == 0 {
                    std::process::exit(0);
                }
            }
            std::process::exit(1);
        });
    }
    track!(fibers_global::execute(global_node))?;

    Ok(())
}
