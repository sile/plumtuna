use crate::study_list::{StudyId, StudyListNodeHandle, StudyName};
use bytecodec::json_codec::{JsonDecoder, JsonEncoder};
use bytecodec::marker::Never;
use fibers_http_server::{HandleRequest, Reply, Req, Res, Status};
use futures::Future;
use httpcodec::{BodyDecoder, BodyEncoder};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct PostStudyReq {
    study_name: StudyName,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostStudyRes {
    study_id: StudyId,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HttpResult<T> {
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
