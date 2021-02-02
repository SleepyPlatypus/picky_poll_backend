use super::*;
use super::operations::*;
use async_trait::async_trait;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Resource};
use tokio::prelude;
use actix_web::http::StatusCode;

const POLLS: &str = "/polls";

async fn post_poll_handler<A: 'static + PollOperations>(ops: web::Data<A>,
                            body: web::Json<PostPollRequest>,
                            request: web::HttpRequest) -> impl Responder
{
    let key = request
        .headers()
        .get("SECRET_KEY")
        .unwrap()
        .to_str()
        .unwrap();
    let id = Identity::SecretKey(key.to_string());
    ops.post_poll(&id, body.0).await
        .map_err(|e|{
            HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).body(format!("{:?}", e))
        })
        .map(|r| web::Json(r))
}

pub fn post_poll<A: 'static + PollOperations>() -> Resource {
    web::resource(POLLS)
        .route(web::post().to(post_poll_handler::<A>))
}

#[cfg(test)]
mod tests {
    use actix_web::test;
    use Clone;
    use super::*;
    use actix_web::http::Method;

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
                .service(post_poll::<operations::MockPollOperations>())
        ).await;

        let requestBody = PostPollRequest{ name: "test name".to_string(), description: "test description".to_string() };
        let request = test::TestRequest::with_header("SECRET_KEY", "my_secret")
            .uri("/polls")
            .set_json(&requestBody)
            .method(Method::POST)
            .to_request();
        let mut response = test::call_service(&mut app, request).await;

        assert_eq!(response.status(), StatusCode::OK);
        let responseBody: PostPollResponse = test::read_body_json(response).await;
        assert_eq!(mock_poll_id, responseBody.id);
    }
}