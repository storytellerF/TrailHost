use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use trailhost::{build_router, ws, AppState};

const TEST_SECRET: &str = "test_jwt_secret_at_least_32_chars!!";

fn make_state(pool: PgPool) -> AppState {
    AppState {
        db: pool,
        jwt_secret: TEST_SECRET.to_string(),
        ws_hub: ws::new_hub(),
    }
}

async fn read_json(body: Body) -> Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn post_json(app: axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let json = read_json(res.into_body()).await;
    (status, json)
}

// ── Auth ──────────────────────────────────────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn register_success(pool: PgPool) {
    let app = build_router(make_state(pool));
    let (status, body) = post_json(
        app,
        "/api/auth/register",
        json!({ "email": "alice@example.com", "password": "password123" }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
    assert!(body["user_id"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn register_duplicate_email(pool: PgPool) {
    let app = build_router(make_state(pool));
    let payload = json!({ "email": "bob@example.com", "password": "password123" });

    let (s1, _) = post_json(app.clone(), "/api/auth/register", payload.clone()).await;
    let (s2, _) = post_json(app, "/api/auth/register", payload).await;

    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "./migrations")]
async fn login_success(pool: PgPool) {
    let app = build_router(make_state(pool));
    let creds = json!({ "email": "carol@example.com", "password": "password123" });

    post_json(app.clone(), "/api/auth/register", creds.clone()).await;
    let (status, body) = post_json(app, "/api/auth/login", creds).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["access_token"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn login_wrong_password(pool: PgPool) {
    let app = build_router(make_state(pool));
    post_json(
        app.clone(),
        "/api/auth/register",
        json!({ "email": "dave@example.com", "password": "correct_pass" }),
    )
    .await;

    let (status, _) = post_json(
        app,
        "/api/auth/login",
        json!({ "email": "dave@example.com", "password": "wrong_pass" }),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn login_unknown_email(pool: PgPool) {
    let app = build_router(make_state(pool));
    let (status, _) = post_json(
        app,
        "/api/auth/login",
        json!({ "email": "nobody@example.com", "password": "pass" }),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_token_works(pool: PgPool) {
    let app = build_router(make_state(pool));
    let (_, reg) = post_json(
        app.clone(),
        "/api/auth/register",
        json!({ "email": "eve@example.com", "password": "password123" }),
    )
    .await;

    let refresh_token = reg["refresh_token"].as_str().unwrap().to_string();
    let (status, body) = post_json(
        app,
        "/api/auth/refresh",
        json!({ "refresh_token": refresh_token }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["access_token"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_with_invalid_token(pool: PgPool) {
    let app = build_router(make_state(pool));
    let (status, _) = post_json(
        app,
        "/api/auth/refresh",
        json!({ "refresh_token": "not.a.valid.token" }),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ── History ───────────────────────────────────────────────────────────────────

async fn register_and_login(app: axum::Router, email: &str) -> String {
    let creds = json!({ "email": email, "password": "password123" });
    let (_, body) = post_json(app.clone(), "/api/auth/register", creds.clone()).await;
    body["access_token"].as_str().unwrap().to_string()
}

async fn authed_post(app: axum::Router, uri: &str, token: &str, body: Value) -> StatusCode {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
    .status()
}

async fn authed_get(app: axum::Router, uri: &str, token: &str) -> (StatusCode, Value) {
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let json = read_json(res.into_body()).await;
    (status, json)
}

#[sqlx::test(migrations = "./migrations")]
async fn history_batch_upsert_and_list(pool: PgPool) {
    let app = build_router(make_state(pool));
    let token = register_and_login(app.clone(), "frank@example.com").await;

    let status = authed_post(
        app.clone(),
        "/api/history/batch",
        &token,
        json!({
            "device_id": "00000000-0000-0000-0000-000000000001",
            "device_name": "Test Browser",
            "entries": [
                { "url": "https://example.com", "title": "Example", "visit_time": "2024-01-01T10:00:00Z" },
                { "url": "https://rust-lang.org", "title": "Rust", "visit_time": "2024-01-01T11:00:00Z" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = authed_get(app, "/api/history", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn history_batch_idempotent(pool: PgPool) {
    let app = build_router(make_state(pool));
    let token = register_and_login(app.clone(), "grace@example.com").await;

    let batch = json!({
        "device_id": "00000000-0000-0000-0000-000000000002",
        "device_name": "Test Browser",
        "entries": [
            { "url": "https://example.com", "title": "Example", "visit_time": "2024-01-01T10:00:00Z" }
        ]
    });

    authed_post(app.clone(), "/api/history/batch", &token, batch.clone()).await;
    authed_post(app.clone(), "/api/history/batch", &token, batch).await;

    let (_, body) = authed_get(app, "/api/history", &token).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn history_search(pool: PgPool) {
    let app = build_router(make_state(pool));
    let token = register_and_login(app.clone(), "heidi@example.com").await;

    authed_post(
        app.clone(),
        "/api/history/batch",
        &token,
        json!({
            "device_id": "00000000-0000-0000-0000-000000000003",
            "device_name": "Test",
            "entries": [
                { "url": "https://rust-lang.org", "title": "Rust Language", "visit_time": "2024-01-01T10:00:00Z" },
                { "url": "https://python.org", "title": "Python", "visit_time": "2024-01-01T11:00:00Z" }
            ]
        }),
    )
    .await;

    let (_, body) = authed_get(app, "/api/history?q=rust", &token).await;
    let entries = body.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["url"], "https://rust-lang.org");
}

#[sqlx::test(migrations = "./migrations")]
async fn history_delete(pool: PgPool) {
    let app = build_router(make_state(pool));
    let token = register_and_login(app.clone(), "ivan@example.com").await;

    authed_post(
        app.clone(),
        "/api/history/batch",
        &token,
        json!({
            "device_id": "00000000-0000-0000-0000-000000000004",
            "device_name": "Test",
            "entries": [
                { "url": "https://delete-me.com", "title": "Delete me", "visit_time": "2024-01-01T10:00:00Z" }
            ]
        }),
    )
    .await;

    let (_, body) = authed_get(app.clone(), "/api/history", &token).await;
    let id = body.as_array().unwrap()[0]["id"].as_str().unwrap().to_string();

    let status = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/history/{id}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status();

    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "./migrations")]
async fn history_delete_other_users_entry(pool: PgPool) {
    let app = build_router(make_state(pool));
    let token_a = register_and_login(app.clone(), "judy@example.com").await;
    let token_b = register_and_login(app.clone(), "karl@example.com").await;

    authed_post(
        app.clone(),
        "/api/history/batch",
        &token_a,
        json!({
            "device_id": "00000000-0000-0000-0000-000000000005",
            "device_name": "Test",
            "entries": [
                { "url": "https://judy-only.com", "title": "Private", "visit_time": "2024-01-01T10:00:00Z" }
            ]
        }),
    )
    .await;

    let (_, body) = authed_get(app.clone(), "/api/history", &token_a).await;
    let id = body.as_array().unwrap()[0]["id"].as_str().unwrap().to_string();

    let status = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/history/{id}"))
                .header("authorization", format!("Bearer {token_b}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status();

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn unauthenticated_history_request(pool: PgPool) {
    let app = build_router(make_state(pool));
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/history")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn health_endpoint(pool: PgPool) {
    let app = build_router(make_state(pool));
    let res = app
        .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}
