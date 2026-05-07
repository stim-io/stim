use super::support::*;

#[tokio::test]
async fn health_uses_api_prefix() {
    let app = build_router(AppState::single_santi(
        "test",
        "http://127.0.0.1:18081".into(),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn instances_project_santi_meta() {
    let gateway = start_mock_santi().await;

    let app = build_router(AppState::single_santi("test", gateway));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents/instances")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json.pointer("/active_instance_id")
            .and_then(serde_json::Value::as_str),
        Some("local-santi")
    );
    assert_eq!(
        json.pointer("/instances/0/active")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        json.pointer("/instances/0/state")
            .and_then(serde_json::Value::as_str),
        Some("ready")
    );
    assert_eq!(
        json.pointer("/instances/0/provider/api")
            .and_then(serde_json::Value::as_str),
        Some("responses")
    );
    assert_eq!(
        json.pointer("/instances/0/config/config_version")
            .and_then(serde_json::Value::as_u64),
        Some(2)
    );
    assert_eq!(
        json.pointer("/instances/0/config/source")
            .and_then(serde_json::Value::as_str),
        Some("admin-apply")
    );
    assert_eq!(
        json.pointer("/instances/0/service/launch_profile")
            .and_then(serde_json::Value::as_str),
        Some("local-foreground")
    );
    assert_eq!(
        json.pointer("/instances/0/service/capabilities/admin_hooks")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        json.pointer("/instances/0/provider_probe/state")
            .and_then(serde_json::Value::as_str),
        Some("ready")
    );
}

#[tokio::test]
async fn probe_returns_santi_snapshot() {
    let gateway = start_mock_santi().await;

    let app = build_router(AppState::single_santi("test", gateway));
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/agents/instances/local-santi/probe")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json.pointer("/state").and_then(serde_json::Value::as_str),
        Some("ready")
    );
    assert_eq!(
        json.pointer("/runtime/runtime_root")
            .and_then(serde_json::Value::as_str),
        Some("/runtime")
    );
    assert_eq!(
        json.pointer("/config/last_event_id")
            .and_then(serde_json::Value::as_str),
        Some("config.applied-test")
    );
    assert_eq!(
        json.pointer("/provider_probe/http_status")
            .and_then(serde_json::Value::as_u64),
        Some(200)
    );
}

#[tokio::test]
async fn provider_probe_returns_facts() {
    let gateway = start_mock_santi().await;

    let app = build_router(AppState::single_santi("test", gateway));
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/agents/instances/local-santi/provider/probe")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        json.pointer("/instance_id")
            .and_then(serde_json::Value::as_str),
        Some("local-santi")
    );
    assert_eq!(
        json.pointer("/provider_probe/state")
            .and_then(serde_json::Value::as_str),
        Some("ready")
    );
}
