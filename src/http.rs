use crate::global::GlobalNodeHandle;
use crate::study::{self, StudyDirection, StudyName, StudyNameAndId, StudySummary, SubscribeId};
use crate::trial::{Trial, TrialId, TrialParamValue, TrialState};
use crate::{Error, ErrorKind, Result};
use bytecodec::json_codec::{JsonDecoder, JsonEncoder};
use bytecodec::marker::Never;
use bytecodec::null::NullDecoder;
use fibers_http_server::{HandleRequest, Reply, Req, Res, Status};
use futures::future::{done, ok};
use futures::Future;
use httpcodec::{BodyDecoder, BodyEncoder};
use serde_json::Value as JsonValue;
use std;
use std::time::Duration;
use trackable::error::ErrorKindExt;
use url::{self, Url};

macro_rules! http_try {
    ($x:expr) => {
        match track!($x) {
            Ok(v) => v,
            Err(e) => {
                return Box::new(done(into_http_response(Err(e))));
            }
        }
    };
}

pub struct PostStudy(pub GlobalNodeHandle);
impl HandleRequest for PostStudy {
    const METHOD: &'static str = "POST";
    const PATH: &'static str = "/studies";

    type ReqBody = PostStudyReq;
    type ResBody = HttpResult<PostStudyRes>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let wait_time = Duration::from_secs(1); // TODO
        let future = self
            .0
            .create_study(req.into_body().study_name, wait_time)
            .map(|study_id| PostStudyRes { study_id });
        Box::new(track_err!(future).then(into_http_response))
    }
}

pub struct GetStudyByName(pub GlobalNodeHandle);
impl HandleRequest for GetStudyByName {
    const METHOD: &'static str = "GET";
    const PATH: &'static str = "/study_names/*";

    type ReqBody = ();
    type ResBody = HttpResult<StudyNameAndId>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let wait_time = Duration::from_millis(1500); // TODO
        let study_name = http_try!(get_study_name(req.url()));
        let future = track_err!(self.0.join_study(study_name.clone(), wait_time));
        Box::new(
            future
                .map(move |study_id| StudyNameAndId {
                    study_name,
                    study_id,
                })
                .then(into_http_response),
        )
    }
}

pub struct GetStudies(pub GlobalNodeHandle);
impl HandleRequest for GetStudies {
    const METHOD: &'static str = "GET";
    const PATH: &'static str = "/studies";

    type ReqBody = ();
    type ResBody = HttpResult<Vec<StudyNameAndId>>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, _req: Req<Self::ReqBody>) -> Self::Reply {
        let future = track_err!(self.0.get_studies());
        Box::new(future.then(into_http_response))
    }
}

pub struct GetStudy(pub GlobalNodeHandle);
impl HandleRequest for GetStudy {
    const METHOD: &'static str = "GET";
    const PATH: &'static str = "/studies/*";

    type ReqBody = ();
    type ResBody = HttpResult<StudySummary>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_id = http_try!(get_study_id(req.url()));
        let study_node = http_try!(self.0.get_study_node(&study_id));
        let future = track_err!(study_node.get_summary());
        Box::new(future.then(into_http_response))
    }
}

pub struct PutStudyDirection(pub GlobalNodeHandle);
impl HandleRequest for PutStudyDirection {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/studies/*/direction";

    type ReqBody = StudyDirection;
    type ResBody = HttpResult<()>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_id = http_try!(get_study_id(req.url()));
        let study_node = http_try!(self.0.get_study_node(&study_id));
        let direction = req.into_body();
        study_node.set_study_direction(direction);
        Box::new(ok(http_ok(())))
    }
}

pub struct PutStudyUserAttr(pub GlobalNodeHandle);
impl HandleRequest for PutStudyUserAttr {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/studies/*/user_attrs/*";

    type ReqBody = JsonValue;
    type ResBody = HttpResult<()>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_id = http_try!(get_study_id(req.url()));
        let study_node = http_try!(self.0.get_study_node(&study_id));
        let key = http_try!(get_attr_key(req.url()));
        let value = req.into_body();
        study_node.set_study_user_attr(key, value);
        Box::new(ok(http_ok(())))
    }
}

pub struct PutStudySystemAttr(pub GlobalNodeHandle);
impl HandleRequest for PutStudySystemAttr {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/studies/*/system_attrs/*";

    type ReqBody = JsonValue;
    type ResBody = HttpResult<()>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_id = http_try!(get_study_id(req.url()));
        let study_node = http_try!(self.0.get_study_node(&study_id));
        let key = http_try!(get_attr_key(req.url()));
        let value = req.into_body();
        study_node.set_study_system_attr(key, value);
        Box::new(ok(http_ok(())))
    }
}

pub struct PostTrial(pub GlobalNodeHandle);
impl HandleRequest for PostTrial {
    const METHOD: &'static str = "POST";
    const PATH: &'static str = "/studies/*/trials";

    type ReqBody = ();
    type ResBody = HttpResult<TrialId>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_id = http_try!(get_study_id(req.url()));
        let study_node = http_try!(self.0.get_study_node(&study_id));
        let trial_id = TrialId::new(&study_id);
        study_node.create_trial(trial_id.clone());
        Box::new(ok(http_ok(trial_id)))
    }
}

pub struct PutTrialState(pub GlobalNodeHandle);
impl HandleRequest for PutTrialState {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/trials/*/state";

    type ReqBody = TrialState;
    type ResBody = HttpResult<()>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let trial_id = http_try!(get_trial_id(req.url()));
        let state = req.into_body();
        let study_id = http_try!(trial_id.get_study_id());
        let study_node = http_try!(self.0.get_study_node(&study_id));
        study_node.set_trial_state(trial_id.clone(), state);
        Box::new(ok(http_ok(())))
    }
}

pub struct PutTrialParam(pub GlobalNodeHandle);
impl HandleRequest for PutTrialParam {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/trials/*/params/*";

    type ReqBody = TrialParamValue;
    type ResBody = HttpResult<()>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let trial_id = http_try!(get_trial_id(req.url()));
        let study_id = http_try!(trial_id.get_study_id());
        let study_node = http_try!(self.0.get_study_node(&study_id));

        let key = http_try!(get_attr_key(req.url()));
        let value = req.into_body();
        study_node.set_trial_param(trial_id, key, value);
        Box::new(ok(http_ok(())))
    }
}

pub struct PutTrialValue(pub GlobalNodeHandle);
impl HandleRequest for PutTrialValue {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/trials/*/value";

    type ReqBody = f64;
    type ResBody = HttpResult<()>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let trial_id = http_try!(get_trial_id(req.url()));
        let study_id = http_try!(trial_id.get_study_id());
        let study_node = http_try!(self.0.get_study_node(&study_id));

        let value = req.into_body();
        study_node.set_trial_value(trial_id, value);
        Box::new(ok(http_ok(())))
    }
}

pub struct PutTrialIntermediateValue(pub GlobalNodeHandle);
impl HandleRequest for PutTrialIntermediateValue {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/trials/*/intermediate_values/*";

    type ReqBody = f64;
    type ResBody = HttpResult<()>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let trial_id = http_try!(get_trial_id(req.url()));
        let study_id = http_try!(trial_id.get_study_id());
        let study_node = http_try!(self.0.get_study_node(&study_id));

        let step = http_try!(get_step(req.url()));
        let value = req.into_body();
        study_node.set_trial_intermediate_value(trial_id, step, value);
        Box::new(ok(http_ok(())))
    }
}

pub struct PutTrialSystemAttr(pub GlobalNodeHandle);
impl HandleRequest for PutTrialSystemAttr {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/trials/*/system_attrs/*";

    type ReqBody = JsonValue;
    type ResBody = HttpResult<()>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let trial_id = http_try!(get_trial_id(req.url()));
        let study_id = http_try!(trial_id.get_study_id());
        let study_node = http_try!(self.0.get_study_node(&study_id));

        let key = http_try!(get_attr_key(req.url()));
        let value = req.into_body();
        study_node.set_trial_system_attr(trial_id, key, value);
        Box::new(ok(http_ok(())))
    }
}

pub struct PutTrialUserAttr(pub GlobalNodeHandle);
impl HandleRequest for PutTrialUserAttr {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/trials/*/user_attrs/*";

    type ReqBody = JsonValue;
    type ResBody = HttpResult<()>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let trial_id = http_try!(get_trial_id(req.url()));
        let study_id = http_try!(trial_id.get_study_id());
        let study_node = http_try!(self.0.get_study_node(&study_id));

        let key = http_try!(get_attr_key(req.url()));
        let value = req.into_body();
        study_node.set_trial_user_attr(trial_id, key, value);
        Box::new(ok(http_ok(())))
    }
}

pub struct GetTrial(pub GlobalNodeHandle);
impl HandleRequest for GetTrial {
    const METHOD: &'static str = "GET";
    const PATH: &'static str = "/trials/*";

    type ReqBody = ();
    type ResBody = HttpResult<Trial>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let trial_id = http_try!(get_trial_id(req.url()));
        let study_id = http_try!(trial_id.get_study_id());
        let study_node = http_try!(self.0.get_study_node(&study_id));

        let future = study_node.get_trial(trial_id);
        Box::new(track_err!(future).then(into_http_response))
    }
}

pub struct GetTrials(pub GlobalNodeHandle);
impl HandleRequest for GetTrials {
    const METHOD: &'static str = "GET";
    const PATH: &'static str = "/studies/*/trials";

    type ReqBody = ();
    type ResBody = HttpResult<Vec<Trial>>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_id = http_try!(get_study_id(req.url()));
        let study_node = http_try!(self.0.get_study_node(&study_id));
        let future = track_err!(study_node.get_trials());
        Box::new(future.then(into_http_response))
    }
}

fn get_trial_id(url: &Url) -> Result<TrialId> {
    let id = url
        .path_segments()
        .expect("never fails")
        .nth(1)
        .expect("never fails");
    Ok(TrialId::from(id.to_owned()))
}

fn get_attr_key(url: &Url) -> Result<String> {
    let key = url
        .path_segments()
        .expect("never fails")
        .nth(3)
        .expect("never fails");
    track!(percent_decode(key).map_err(Error::from))
}

fn get_subscribe_id(url: &Url) -> Result<SubscribeId> {
    let id = url
        .path_segments()
        .expect("never fails")
        .nth(3)
        .expect("never fails");
    let id: u32 = track!(id.parse().map_err(Error::from))?;
    Ok(SubscribeId::from(id))
}

fn get_step(url: &Url) -> Result<u32> {
    let step = url
        .path_segments()
        .expect("never fails")
        .nth(3)
        .expect("never fails");
    track!(step.parse().map_err(Error::from))
}

fn get_study_id(url: &Url) -> Result<crate::study::StudyId> {
    use uuid::Uuid;

    let id = url
        .path_segments()
        .expect("never fails")
        .nth(1)
        .expect("never fails");
    let id: Uuid = track!(id.parse().map_err(Error::from); url)?;
    Ok(crate::study::StudyId::from(id))
}

fn get_study_name(url: &Url) -> Result<StudyName> {
    let name = url
        .path_segments()
        .expect("never fails")
        .nth(1)
        .expect("never fails");
    Ok(StudyName::new(name.to_owned()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostStudyReq {
    study_name: study::StudyName,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostStudyRes {
    study_id: study::StudyId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HttpResult<T> {
    Ok(T),
    Err { reason: String },
}

fn http_ok<T>(body: T) -> Res<HttpResult<T>> {
    Res::new(Status::Ok, HttpResult::Ok(body))
}

fn into_http_response<T>(
    result: std::result::Result<T, Error>,
) -> std::result::Result<Res<HttpResult<T>>, Never> {
    Ok(match result {
        Ok(v) => Res::new(Status::Ok, HttpResult::Ok(v)),
        Err(e) => {
            let status = match *e.kind() {
                ErrorKind::AlreadyExists => Status::Conflict,
                ErrorKind::NotFound => Status::NotFound,
                ErrorKind::Other => Status::InternalServerError,
            };
            Res::new(
                status,
                HttpResult::Err {
                    reason: e.to_string(),
                },
            )
        }
    })
}

pub struct PostStudySubscribe(pub GlobalNodeHandle);
impl HandleRequest for PostStudySubscribe {
    const METHOD: &'static str = "POST";
    const PATH: &'static str = "/studies/*/subscribe";

    type ReqBody = ();
    type ResBody = HttpResult<SubscribeId>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_id = http_try!(get_study_id(req.url()));
        let study_node = http_try!(self.0.get_study_node(&study_id));
        let future = track_err!(study_node.subscribe());
        Box::new(future.then(into_http_response))
    }
}

pub struct GetNewEvents(pub GlobalNodeHandle);
impl HandleRequest for GetNewEvents {
    const METHOD: &'static str = "GET";
    const PATH: &'static str = "/studies/*/subscribe/*";

    type ReqBody = ();
    type ResBody = HttpResult<Vec<crate::study::Message>>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_id = http_try!(get_study_id(req.url()));
        let study_node = http_try!(self.0.get_study_node(&study_id));
        let subscribe_id = http_try!(get_subscribe_id(req.url()));
        let future = track_err!(study_node.poll_events(subscribe_id));
        Box::new(future.then(into_http_response))
    }
}

fn percent_decode(s: &str) -> Result<String> {
    let mut chars = s.chars();
    let mut decoded = String::new();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h0 = track!(chars
                .next()
                .ok_or_else(|| ErrorKind::Other.cause("Invalid escaped char")))?;
            let h1 = track!(chars
                .next()
                .ok_or_else(|| ErrorKind::Other.cause("Invalid escaped char")))?;
            let code = track!(u32::from_str_radix(&format!("{h0}{h1}"), 16)
                .map_err(|e| track!(ErrorKind::Other.cause(e))))?;
            let c =
                track!(char::from_u32(code)
                    .ok_or_else(|| ErrorKind::Other.cause("Invalid escaped char")))?;
            decoded.push(c);
            continue;
        }
        decoded.push(c);
    }
    Ok(decoded)
}
