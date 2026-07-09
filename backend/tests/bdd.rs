use axum::body::Body;
use axum::http::Request;
use cucumber::{World as _, given, then, when};
use tower::ServiceExt;

#[derive(Debug, Default, cucumber::World)]
struct HealthWorld {
    response_status: Option<u16>,
    response_body: Option<String>,
}

#[given("the backend service is running")]
async fn given_backend_running(_world: &mut HealthWorld) {}

#[when("the operator checks the service health")]
async fn when_check_health(world: &mut HealthWorld) {
    let router = backend::router();
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.response_status = Some(response.status().as_u16());
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.response_body = Some(String::from_utf8(body_bytes.to_vec()).unwrap());
}

#[then("the service reports it is ok")]
async fn then_service_ok(world: &mut HealthWorld) {
    assert_eq!(world.response_status, Some(200));
    assert_eq!(world.response_body.as_deref(), Some(r#"{"status":"ok"}"#));
}

#[tokio::main]
async fn main() {
    HealthWorld::cucumber()
        .fail_on_skipped()
        .run_and_exit(concat!(env!("CARGO_MANIFEST_DIR"), "/../features"))
        .await;
}
