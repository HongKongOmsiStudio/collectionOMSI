mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_register_and_login_flow() {
    let (app, _pool) = common::setup_test_app().await;

    let test_username = format!("testuser_{}", uuid::Uuid::new_v4());
    let test_email = format!("{}@example.com", test_username);
    let test_password = "SecurePassword123!";

    let register_payload = json!({
        "username": test_username,
        "email": test_email,
        "password": test_password
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(register_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let login_payload = json!({
        "email": test_email,
        "password": test_password
    });

    let login_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(login_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_response.status(), StatusCode::OK);
}
