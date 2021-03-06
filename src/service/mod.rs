use actix_web::{HttpRequest, HttpResponse, Result, web, FromRequest, Error};
use actix_web::dev::{Payload, PayloadStream};
use actix_web::web::ServiceConfig;
use std::future::{Ready, ready};

use crate::model::*;
use crate::operations::*;

mod paths;

const SECRET_KEY: &str = "X-VOTE-SECRET";

impl FromRequest for Identity {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload<PayloadStream>) -> Self::Future {
        let id = req.headers()
            .get(SECRET_KEY)
            .ok_or_else(||{
                let msg = format!("Missing header: {}", SECRET_KEY);
                Error::from(HttpResponse::BadRequest().body(msg))
            }).and_then(|header_value|
                header_value
                    .to_str()
                    .map_err(|_| {
                        let msg = format!("Non-ascii header value: {}", SECRET_KEY);
                        Error::from(HttpResponse::InternalServerError().body(msg))
                    })
            ).map(|secret_key| {
                Identity::SecretKey(secret_key.to_owned())
            });

        ready(id)
    }
}

pub fn config<A: 'static + PollOperationsT>(cfg: &mut ServiceConfig) {
    cfg.route(paths::POST_POLL_PATH,
              web::post().to(paths::post_poll_handler::<A>))
        .route(paths::GET_POLL_PATH,
               web::get().to(paths::get_poll_handler::<A>))
        .route(paths::PUT_BALLOT_PATH,
               web::put().to(paths::put_ballot_handler::<A>))
        .route(paths::POST_CANDIDATE_PATH,
            web::post().to(paths::post_candidate_handler::<A>))
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
        let mut mock_ops = operations::MockPollOperationsT::new();

        let mock_poll_id = "mock poll id";
        let mock_response = Ok(PostPollResponse{id: mock_poll_id.to_string()});
        
        mock_ops.expect_post_poll()
            .return_once(move |_, _| mock_response);

        let mut app = test::init_service(
            App::new()
                .data(mock_ops)
                .configure(config::<MockPollOperationsT>)
        ).await;

        let request_body = PostPollRequest {
            name: "test name".to_string(),
            description: Some("test description".to_string()),
            candidates: Vec::new(),
            configuration: Configuration {
                write_ins: false,
            },
        };
        let request = test::TestRequest::with_header(SECRET_KEY, "my_secret")
            .uri(paths::POST_POLL_PATH)
            .set_json(&request_body)
            .method(Method::POST)
            .to_request();
        let response = test::call_service(&mut app, request).await;

        assert_eq!(StatusCode::OK, response.status());
        let response_body: PostPollResponse = test::read_body_json(response).await;
        assert_eq!(mock_poll_id, response_body.id);
    }
}