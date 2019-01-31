use crate::study_list::{StudyId, StudyListNodeHandle, StudyName};
use crate::{Error, Result};
use bytecodec::json_codec::{JsonDecoder, JsonEncoder};
use bytecodec::marker::Never;
use bytecodec::null::NullDecoder;
use fibers_http_server::{HandleRequest, Reply, Req, Res, Status};
use futures::future::{done, ok};
use futures::Future;
use httpcodec::{BodyDecoder, BodyEncoder, HeadBodyEncoder};
use std;
use url::Url;

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

fn get_study_id(url: &Url) -> Result<StudyId> {
    let id = url
        .path_segments()
        .expect("never fails")
        .nth(1)
        .expect("never fails");
    let id = track!(id.parse().map_err(Error::from); url)?;
    Ok(StudyId::new(id))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostStudyReq {
    study_name: StudyName,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostStudyRes {
    study_id: StudyId,
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
