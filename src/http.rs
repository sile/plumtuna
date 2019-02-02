use crate::study::StudyDirection;
use crate::study_list::{StudyId, StudyListNodeHandle, StudyName};
use crate::trial::{Trial, TrialId, TrialParamValue, TrialState};
use crate::{Error, Result};
use bytecodec::json_codec::{JsonDecoder, JsonEncoder};
use bytecodec::marker::Never;
use bytecodec::null::NullDecoder;
use fibers_http_server::{HandleRequest, Reply, Req, Res, Status};
use futures::future::{done, ok};
use futures::Future;
use httpcodec::{BodyDecoder, BodyEncoder, HeadBodyEncoder};
use serde_json::Value as JsonValue;
use std;
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

pub struct PostStudy(pub StudyListNodeHandle);
impl HandleRequest for PostStudy {
    const METHOD: &'static str = "POST";
    const PATH: &'static str = "/studies";

    type ReqBody = PostStudyReq;
    type ResBody = HttpResult<PostStudyRes>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let future = self
            .0
            .create_study(req.into_body().study_name)
            .map(|study_id| PostStudyRes { study_id });
        Box::new(track_err!(future).then(into_http_response))
    }
}

pub struct PostTrial(pub StudyListNodeHandle);
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
        let trial_id = http_try!(self.0.create_trial(study_id));
        Box::new(ok(http_ok(trial_id)))
    }
}

pub struct GetStudies(pub StudyListNodeHandle);
impl HandleRequest for GetStudies {
    const METHOD: &'static str = "GET";
    const PATH: &'static str = "/studies";

    type ReqBody = ();
    type ResBody = HttpResult<Vec<Study>>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, _req: Req<Self::ReqBody>) -> Self::Reply {
        let studies = self
            .0
            .studies()
            .iter()
            .map(|(&study_id, s)| Study {
                study_id,
                study_name: s.name.clone(),
            })
            .collect();
        Box::new(ok(Res::new(Status::Ok, HttpResult::Ok(studies))))
    }
}

pub struct HeadStudy(pub StudyListNodeHandle);
impl HandleRequest for HeadStudy {
    const METHOD: &'static str = "HEAD";
    const PATH: &'static str = "/studies/*";

    type ReqBody = ();
    type ResBody = HttpResult<Study>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = HeadBodyEncoder<BodyEncoder<JsonEncoder<Self::ResBody>>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_id = http_try!(get_study_id(req.url()));
        let study = http_try!(self.0.fetch_study(study_id));
        Box::new(ok(http_ok(Study {
            study_id: study.id,
            study_name: study.name,
        })))
    }
}

pub struct GetStudy(pub StudyListNodeHandle);
impl HandleRequest for GetStudy {
    const METHOD: &'static str = "GET";
    const PATH: &'static str = "/studies/*";

    type ReqBody = ();
    type ResBody = HttpResult<Study>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_id = http_try!(get_study_id(req.url()));
        let study = http_try!(self.0.fetch_study(study_id));
        Box::new(ok(http_ok(Study {
            study_id: study.id,
            study_name: study.name,
        })))
    }
}

pub struct GetStudyByName(pub StudyListNodeHandle);
impl HandleRequest for GetStudyByName {
    const METHOD: &'static str = "GET";
    const PATH: &'static str = "/study_names/*";

    type ReqBody = ();
    type ResBody = HttpResult<Study>;
    type Decoder = BodyDecoder<NullDecoder>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_name = http_try!(get_study_name(req.url()));
        let study = http_try!(self.0.fetch_study_by_name(&study_name));
        Box::new(ok(http_ok(Study {
            study_id: study.id,
            study_name: study.name,
        })))
    }
}

pub struct PutStudyDirection(pub StudyListNodeHandle);
impl HandleRequest for PutStudyDirection {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/studies/*/direction";

    type ReqBody = StudyDirection;
    type ResBody = HttpResult<StudyDirection>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let study_id = http_try!(get_study_id(req.url()));
        let direction = req.into_body();
        http_try!(self.0.set_study_direction(study_id, direction));
        Box::new(ok(http_ok(direction)))
    }
}

pub struct PutTrialState(pub StudyListNodeHandle);
impl HandleRequest for PutTrialState {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/trials/*/state";

    type ReqBody = TrialState;
    type ResBody = HttpResult<TrialState>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let trial_id = http_try!(get_trial_id(req.url()));
        let state = req.into_body();
        http_try!(self.0.set_trial_state(trial_id, state));
        Box::new(ok(http_ok(state)))
    }
}

pub struct PutTrialValue(pub StudyListNodeHandle);
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
        let value = req.into_body();
        http_try!(self.0.set_trial_value(trial_id, value));
        Box::new(ok(http_ok(())))
    }
}

pub struct PutTrialIntermediateValue(pub StudyListNodeHandle);
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
        let step = http_try!(get_step(req.url()));
        let value = req.into_body();
        http_try!(self.0.set_trial_intermediate_value(trial_id, step, value));
        Box::new(ok(http_ok(())))
    }
}

pub struct PutTrialParam(pub StudyListNodeHandle);
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
        let key = http_try!(get_attr_key(req.url()));
        let value = req.into_body();
        http_try!(self.0.set_trial_param(trial_id, key, value));
        Box::new(ok(http_ok(())))
    }
}

pub struct PutTrialSystemAttr(pub StudyListNodeHandle);
impl HandleRequest for PutTrialSystemAttr {
    const METHOD: &'static str = "PUT";
    const PATH: &'static str = "/trials/*/system_attrs/*";

    type ReqBody = JsonValue;
    type ResBody = HttpResult<JsonValue>;
    type Decoder = BodyDecoder<JsonDecoder<Self::ReqBody>>;
    type Encoder = BodyEncoder<JsonEncoder<Self::ResBody>>;
    type Reply = Reply<Self::ResBody>;

    fn handle_request(&self, req: Req<Self::ReqBody>) -> Self::Reply {
        let trial_id = http_try!(get_trial_id(req.url()));
        let key = http_try!(get_attr_key(req.url()));
        let value = req.into_body();
        let future = self.0.set_trial_system_attr(trial_id, key, value.clone());
        Box::new(track_err!(future.map(move |_| value)).then(into_http_response))
    }
}

pub struct PutTrialUserAttr(pub StudyListNodeHandle);
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
        let key = http_try!(get_attr_key(req.url()));
        let value = req.into_body();
        http_try!(self.0.set_trial_user_attr(trial_id, key, value.clone()));
        Box::new(ok(http_ok(())))
    }
}

pub struct GetTrials(pub StudyListNodeHandle);
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
        let future = self.0.get_trials(study_id);
        Box::new(track_err!(future).then(into_http_response))
    }
}

pub struct GetTrial(pub StudyListNodeHandle);
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
        let future = self.0.get_trial(trial_id);
        Box::new(track_err!(future).then(into_http_response))
    }
}

fn get_trial_id(url: &Url) -> Result<TrialId> {
    let id = url
        .path_segments()
        .expect("never fails")
        .nth(1)
        .expect("never fails");
    let id = track!(id.parse().map_err(Error::from); url)?;
    Ok(TrialId::new(id))
}

fn get_attr_key(url: &Url) -> Result<String> {
    let key = url
        .path_segments()
        .expect("never fails")
        .nth(3)
        .expect("never fails");
    track!(url::percent_encoding::percent_decode(key.as_bytes())
        .decode_utf8()
        .map(|s| s.into_owned())
        .map_err(Error::from))
}

fn get_step(url: &Url) -> Result<u32> {
    let step = url
        .path_segments()
        .expect("never fails")
        .nth(3)
        .expect("never fails");
    track!(step.parse().map_err(Error::from))
}

fn get_study_id(url: &Url) -> Result<StudyId> {
    let id = url
        .path_segments()
        .expect("never fails")
        .nth(1)
        .expect("never fails");
    let id = track!(id.parse().map_err(Error::from); url)?;
    Ok(StudyId::new(id))
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
    study_name: StudyName,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostStudyRes {
    study_id: Option<StudyId>, // `None` means the study already exists
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Study {
    study_id: StudyId,
    study_name: StudyName,
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

fn into_http_response<T, E>(
    result: std::result::Result<T, E>,
) -> std::result::Result<Res<HttpResult<T>>, Never>
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
