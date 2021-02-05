use actix_web::{Error, Result};
use actix_web::web::{Data, HttpResponse, Path, Json};
use crate::model::*;
use crate::operations::{PollOperations, GetPollError};

pub const POST_POLLS_PATH: &str = "/polls";
pub const GET_POLLS_PATH: &str = "/polls/{poll_id}";

pub async fn get_poll_handler<A: 'static + PollOperations>(
    ops: Data<A>,
    path: Path<String>) -> Result<Json<GetPollResponse>>
{
    let poll = ops.get_poll(&path)
        .await
        .map_err(|e| {
            match e {
                GetPollError::NotFound =>
                    Error::from(HttpResponse::NotFound()),
                GetPollError::Error(_) =>
                    Error::from(HttpResponse::InternalServerError()),
            }
        })?;
    Ok(Json(poll))
}

pub async fn post_poll_handler<A: 'static + PollOperations>(
    ops: Data<A>,
    body: Json<PostPollRequest>,
    id: Identity) -> Result<Json<PostPollResponse>>
{
    let Json(request_body) = body;
    let ok = ops.post_poll(&id, request_body)
        .await
        .map_err(|_|{
            Error::from(HttpResponse::InternalServerError())
        })?;
    Ok(Json(ok))
}