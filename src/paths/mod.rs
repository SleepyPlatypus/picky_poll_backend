use actix_web::{HttpRequest, HttpResponse, Result, web, FromRequest, Error};
use actix_web::web::{Data, Json, Path, ServiceConfig};

use crate::model::*;
use crate::operations::*;
use actix_web::dev::{PayloadStream, Payload};
use std::future::{Ready, ready};

const POST_POLLS_PATH: &str = "/polls";
const GET_POLLS_PATH: &str = "/polls/{poll_id}";
const SECRET_KEY: &str = "SECRET-KEY";

impl FromRequest for Identity {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload<PayloadStream>) -> Self::Future {
        let id = req.headers()
            .get("SECRET-KEY")
            .ok_or_else(||{
                let msg = format!("Missing header: {}", SECRET_KEY);
                Error::from(HttpResponse::BadRequest().body(msg))
            }).and_then(|header_value|
                header_value
                    .to_str()
                    .map_err(|_| {
                        let msg = format!("Failed to handle header value for {}", SECRET_KEY);
                        Error::from(HttpResponse::InternalServerError().body(msg))
                    })
            ).map(|secret_key| {
                Identity::SecretKey(secret_key.to_string())
            });

        ready(id)
    }
}

async fn get_poll_handler<A: 'static + PollOperations>(
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

async fn post_poll_handler<A: 'static + PollOperations>(
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

pub fn config<A: 'static + PollOperations>(cfg: &mut ServiceConfig) {
    cfg.route(POST_POLLS_PATH, web::post().to(post_poll_handler::<A>))
        .route(GET_POLLS_PATH, web::get().to(get_poll_handler::<A>))
    ;
}

#[cfg(test)]
mod tests {
    use actix_web::App;
    use actix_web::http::Method;
    use actix_web::http::StatusCode;
    use actix_web::test;

    use crate::operations;

    use super::*;

    #[tokio::test]
    async fn test_post_poll() {
        let mut mock_ops = operations::MockPollOperations::new();

        let mock_poll_id = "mock poll id";
        let mock_response = Ok(PostPollResponse{id: mock_poll_id.to_string()});
        mock_ops.expect_post_poll()
            .return_once(move |_, _| mock_response);

        let mut app = test::init_service(
            App::new()
                .data(mock_ops)
                .configure(config::<MockPollOperations>)
        ).await;

        let request_body = PostPollRequest{
            name: "test name".to_string(),
            description: "test description".to_string()
        };
        let request = test::TestRequest::with_header(SECRET_KEY, "my_secret")
            .uri(POST_POLLS_PATH)
            .set_json(&request_body)
            .method(Method::POST)
            .to_request();
        let response = test::call_service(&mut app, request).await;

        assert_eq!(StatusCode::OK, response.status());
        let response_body: PostPollResponse = test::read_body_json(response).await;
        assert_eq!(mock_poll_id, response_body.id);
    }
}