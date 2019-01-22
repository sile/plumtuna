#[macro_use]
extern crate slog;

use bytecodec::bincode_codec::{BincodeDecoder, BincodeEncoder};
use bytecodec::json_codec::{JsonDecoder, JsonEncoder};
use bytecodec::marker::Never;
use fibers::sync::{mpsc, oneshot};
use fibers_http_server::{HandleRequest, Reply, Req, Res, ServerBuilder, Status};
use futures::{Async, Future, Poll, Stream};
use httpcodec::{BodyDecoder, BodyEncoder};
use plumcast::message::MessagePayload;
use plumcast::node::{LocalNodeId, Node, NodeBuilder, NodeId, SerialLocalNodeIdGenerator};
use plumcast::service::ServiceBuilder;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use slog::Logger;
use sloggers::terminal::TerminalLoggerBuilder;
use sloggers::types::Severity;
use sloggers::Build;
use std::collections::HashMap;
use std::net::SocketAddr;
use structopt::StructOpt;
use trackable::error::{ErrorKindExt, Failed, Failure};
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
        .finish::<_, RpcMessage, _>(fibers_global::handle(), SerialLocalNodeIdGenerator::new());
    let mut node = NodeBuilder::new()
        .logger(logger.clone())
        .finish(service.handle());
    fibers_global::spawn(service.map_err(|e| panic!("{}", e)));

    if let Some(contact) = opt.contact_server {
        node.join(NodeId::new(contact, LocalNodeId::new(0)));
    }

    let (command_tx, command_rx) = mpsc::channel();
    let agent = Agent {
        logger: logger.clone(),
        node,
        command_rx,
        study_seqno: 0,
        studies: HashMap::new(),
    };
    let agent_handle = AgentHandle { command_tx };

    let mut builder = ServerBuilder::new(([0, 0, 0, 0], opt.http_port).into());
    builder.logger(logger);
    track!(builder.add_handler(CreateNewStudyIdApi(agent_handle.clone())))?;
    let server = builder.finish(fibers_global::handle());
    fibers_global::spawn(server.map_err(|e| panic!("{}", e)));

    track!(fibers_global::execute(agent))?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum RpcMessage {
    CreateNewStudyId {
        study_name: String,
        study_id: u32,
    },
    SetStudyUserAttr {
        study_id: u32,
        attr: Attr,
    },
    SetStudyDirection {
        study_id: u32,
        direction: String,
    },
    SetStudySystemAttr {
        study_id: u32,
        attr: Attr,
    },
    CreateNewTrialId {
        study_id: u32,
    },
    SetTrialState {
        trial_id: u32,
        state: TrialState,
    },
    SetTrialParam {
        trial_id: u32,
        param_name: String,
        param_value_internal: f64,
        distribution: Distribution,
    },
    SetTrialValue {
        trial_id: u32,
        value: f64,
    },
    SetTrialIntermediateValue {
        trial_id: u32,
        step: u32,
        intermediate_value: f64,
    },
    SetTrialUserAttr {
        trial_id: u32,
        attr: Attr,
    },
    SetTrialSystemAttr {
        trial_id: u32,
        attr: Attr,
    },
}
impl MessagePayload for RpcMessage {
    type Encoder = BincodeEncoder<RpcMessage>;
    type Decoder = BincodeDecoder<RpcMessage>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attr {
    key: String,
    value: JsonValue,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum StudyDirection {
    NotSet,
    Minimize,
    Maximize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum TrialState {
    Running,
    Complete,
    Pruned,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Distribution {
    Uniform { low: f64, high: f64 },
    LogUniform { low: f64, high: f64 },
    DiscreteUniform { low: f64, high: f64, q: f64 },
    IntUniform { low: i64, high: i64 },
    Categorical { choices: Vec<JsonValue> },
}

#[derive(Debug)]
struct Agent {
    logger: Logger,
    node: Node<RpcMessage>,
    command_rx: mpsc::Receiver<Command>,
    study_seqno: u32,
    studies: HashMap<String, u32>,
}
impl Agent {
    fn handle_rpc_message(&mut self, m: RpcMessage) {
        match m {
            RpcMessage::CreateNewStudyId {
                study_name,
                study_id,
            } => {
                assert!(!self.studies.contains_key(&study_name)); // TODO:
                info!(
                    self.logger,
                    "New study is added: {:?}({})", study_name, study_id
                );
                self.studies.insert(study_name, study_id);
            }
            RpcMessage::SetStudyUserAttr { .. } => panic!(),
            RpcMessage::SetStudyDirection { .. } => panic!(),
            RpcMessage::SetStudySystemAttr { .. } => panic!(),
            RpcMessage::CreateNewTrialId { .. } => panic!(),
            RpcMessage::SetTrialState { .. } => panic!(),
            RpcMessage::SetTrialParam { .. } => panic!(),
            RpcMessage::SetTrialValue { .. } => panic!(),
            RpcMessage::SetTrialIntermediateValue { .. } => panic!(),
            RpcMessage::SetTrialUserAttr { .. } => panic!(),
            RpcMessage::SetTrialSystemAttr { .. } => panic!(),
        }
    }

    fn handle_command(&mut self, c: Command) {
        match c {
            Command::CreateNewStudyId {
                study_name,
                reply_tx,
            } => {
                if let Some(study_id) = self.studies.get(&study_name) {
                    reply_tx.exit(Err(track!(Failed.cause(format!(
                        "Study {:?}({}) already exists",
                        study_name, study_id
                    )))
                    .into()));
                } else {
                    let study_id = self.study_seqno;
                    info!(
                        self.logger,
                        "New study is created: {} => {}", study_name, study_id
                    );

                    let m = RpcMessage::CreateNewStudyId {
                        study_name,
                        study_id,
                    };
                    self.node.broadcast(m);

                    self.study_seqno += 1;
                    reply_tx.exit(Ok(study_id));
                }
            }
        }
    }
}
impl Future for Agent {
    type Item = ();
    type Error = Failure;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut did_something = true;
        while did_something {
            did_something = false;

            while let Async::Ready(Some(m)) =
                track!(self.node.poll().map_err(|e| Failed.takes_over(e)))?
            {
                self.handle_rpc_message(m.into_payload());
                did_something = true;
            }
            while let Async::Ready(Some(c)) = self.command_rx.poll().expect("never fails") {
                self.handle_command(c);
                did_something = true;
            }
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug, Clone)]
struct AgentHandle {
    command_tx: mpsc::Sender<Command>,
}

#[derive(Debug)]
enum Command {
    CreateNewStudyId {
        study_name: String,
        reply_tx: oneshot::Monitored<u32, Failure>,
    },
}

struct CreateNewStudyIdApi(AgentHandle);
impl HandleRequest for CreateNewStudyIdApi {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/optuna/study/create_new_study_id";

    type ReqBody = CreateNewStudyIdReq;
    type ResBody = HttpResult<CreateNewStudyIdRes>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let (reply_tx, reply_rx) = oneshot::monitor();
        let command = Command::CreateNewStudyId {
            study_name: req.into_body().study_name,
            reply_tx,
        };
        let _ = self.0.command_tx.send(command);

        let future = reply_rx
            .map(|study_id| CreateNewStudyIdRes { study_id })
            .then(into_http_response);
        Box::new(future)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateNewStudyIdReq {
    study_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateNewStudyIdRes {
    study_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum HttpResult<T> {
    Ok(T),
    Err { reason: String },
}

fn into_http_response<T, E>(result: Result<T, E>) -> Result<Res<HttpResult<T>>, Never>
where
    E: std::fmt::Display,
{
    Ok(match result {
        Ok(v) => Res::new(Status::Ok, HttpResult::Ok(v)),
        Err(e) => Res::new(
            Status::InternalServerError,
            HttpResult::Err {
                reason: e.to_string(),
            },
        ),
    })
}
