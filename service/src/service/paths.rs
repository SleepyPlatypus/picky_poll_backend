use actix_web::Result;
use actix_web::web::{Data, HttpResponse, Path, Json};

use crate::{
    model::*,
    operations::{GetPollError, PostCandidateError, PollOperationsT, PostPollError, PutBallotError}
};

pub const POST_POLL_PATH: &str = "/polls";
pub const POST_CANDIDATE_PATH: &str = "/polls/{poll_id}/candidates";
pub const GET_POLL_PATH: &str = "/polls/{poll_id}";
pub const PUT_BALLOT_PATH: &str = "/polls/{poll_id}/ballots/{ballot_id}";

pub async fn get_poll_handler<A: 'static + PollOperationsT> (
    ops: Data<A>,
    path: Path<String>) -> Result<Json<GetPollResponse>>
{
    let poll = ops.get_poll(&path)
        .await
        .map_err(|e| match e {
            GetPollError::NotFound =>
                HttpResponse::NotFound().finish(),
            GetPollError::Unexpected =>
                HttpResponse::InternalServerError().finish(),
        })?;
    Ok(Json(poll))
}

pub async fn post_candidate_handler<A: 'static + PollOperationsT>(
    ops: Data<A>,
    Path(poll_id): Path<String>,
    body: Json<Candidate>,
) -> Result<HttpResponse> {
    let Json(candidate) = body;
    ops.post_candidate(&poll_id, &candidate)
    .await
    .map_err(|e| match e {
        PostCandidateError::PollNotFound => HttpResponse::NotFound().finish(),
        PostCandidateError::NoWriteIns => HttpResponse::BadRequest().body("Write-ins not allowed for this poll."),
        PostCandidateError::DuplicateCandidate(_) => HttpResponse::Conflict().finish(),
        PostCandidateError::Unexpected => HttpResponse::InternalServerError().finish(),
    })?;

    Ok(HttpResponse::NoContent().finish())
}

pub async fn post_poll_handler<A: 'static + PollOperationsT>(
    ops: Data<A>,
    body: Json<PostPollRequest>,
    id: Identity) -> Result<Json<PostPollResponse>>
{
    let Json(request_body) = body;
    let ok = ops.post_poll(&id, &request_body)
        .await
        .map_err(|e| match e {
            PostPollError::Unexpected => HttpResponse::InternalServerError().finish(),
            PostPollError::DuplicateCandidate(dupe_name) =>
                HttpResponse::BadRequest().body(
                    format!("Duplicate candidate name: [{}]", dupe_name)
                )
        })?;
    Ok(Json(ok))
}

pub async fn put_ballot_handler<A: 'static + PollOperationsT>(
    ops: Data<A>,
    Path((poll_id, ballot_id)): Path<(String, String)>,
    body: Json<PutBallotRequest>,
    user_id: Identity) -> Result<HttpResponse> {
        let Json(request_body) = body;
        ops.put_ballot(&poll_id, &user_id, &ballot_id, &request_body)
            .await
            .map_err(|e| match e {
                PutBallotError::PollNotFound => HttpResponse::NotFound().finish(),
                PutBallotError::Unexpected => HttpResponse::InternalServerError().finish(),
                PutBallotError::NotOwner => HttpResponse::Forbidden().finish(),
                PutBallotError::NotSameName => HttpResponse::BadRequest().finish(),
                PutBallotError::DuplicateRanking(candidate) => {
                    let message = format!("Duplicate ranking: [{}]", candidate);
                    HttpResponse::BadRequest().body(message)
                }
                PutBallotError::CandidateNotFound(name) => {
                    let message = format!("Invalid candidate: [{}]", name);
                    HttpResponse::BadRequest().body(message)
                }
            })?;
        Ok(HttpResponse::NoContent().finish())
    }
