use actix_web::{HttpRequest, HttpResponse, Resource, web};
use actix_web::web::{Data, Json};

use crate::model::*;
use crate::operations::*;

const POLLS: &str = "/polls";
const SECRET_KEY: &str = "SECRET-KEY";

async fn post_poll_handler<A: 'static + PollOperations>(ops: Data<A>,
                            body: Json<PostPollRequest>,
                            request: HttpRequest) -> Result<Json<PostPollResponse>, actix_web::Error>
{
    let key = request
        .headers()
        .get("SECRET-KEY")
        .ok_or_else(|| {
            let msg = format!("Missing header: {}", SECRET_KEY);
            actix_web::Error::from(HttpResponse::BadRequest().body(msg))
        })?;
    let id = Identity::SecretKey(
        key.to_str()
            .map_err(|_| {
                let msg = format!("Failed to handle header value for {}", SECRET_KEY);
                actix_web::Error::from(HttpResponse::InternalServerError().body(msg))
            })?
            .to_string());
    let Json(requestBody) = body;
    let ok = ops.post_poll(&id, requestBody)
        .await
        .map_err(|_|{
            actix_web::Error::from(HttpResponse::InternalServerError())
        })?;
    Ok(Json(ok))
}

pub fn post_poll<A: 'static + PollOperations>(ops: A) -> Resource {
    web::resource(POLLS)
        .app_data(Data::new(ops))
        .route(web::post().to(post_poll_handler::<A>))
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
                .service(post_poll::<operations::MockPollOperations>(mock_ops))
        ).await;

        let request_body = PostPollRequest{
            name: "test name".to_string(),
            description: "test description".to_string()
        };
        let request = test::TestRequest::with_header(SECRET_KEY, "my_secret")
            .uri(POLLS)
            .set_json(&request_body)
            .method(Method::POST)
            .to_request();
        let response = test::call_service(&mut app, request).await;

        assert_eq!(StatusCode::OK, response.status());
        let response_body: PostPollResponse = test::read_body_json(response).await;
        assert_eq!(mock_poll_id, response_body.id);
    }
}