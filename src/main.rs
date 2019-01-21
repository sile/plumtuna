use bytecodec::bincode_codec::{BincodeDecoder, BincodeEncoder};
use futures::Future;
use plumcast::message::MessagePayload;
use plumcast::node::{LocalNodeId, Node, NodeBuilder, NodeId, SerialLocalNodeIdGenerator};
use plumcast::service::ServiceBuilder;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
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
        .finish::<_, RpcMessage, _>(fibers_global::handle(), SerialLocalNodeIdGenerator::new());
    let mut node = NodeBuilder::new().logger(logger).finish(service.handle());
    fibers_global::spawn(service.map_err(|e| panic!("{}", e)));

    if let Some(contact) = opt.contact_server {
        node.join(NodeId::new(contact, LocalNodeId::new(0)));
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum RpcMessage {
    CreateNewStudyId {
        study_name: String,
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
