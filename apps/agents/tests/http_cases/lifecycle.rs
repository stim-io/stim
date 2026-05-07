use super::support::*;

#[tokio::test]
async fn launch_stop_manage_process() {
    let gateway = start_mock_santi().await;
    let app = build_router(
        AppState::santi_instances(
            "test",
            vec![SantiInstanceConfig {
                agent_id: Some("santi-managed".into()),
                participant_id: Some("participant-santi".into()),
                delivery_endpoint_id: Some("endpoint-managed".into()),
                id: "managed".into(),
                label: "Managed".into(),
                endpoint: gateway,
                profile: Some("profile-managed".into()),
                managed: true,
                launch: Some(SantiLaunchConfig {
                    command: "sleep".into(),
                    args: vec!["30".into()],
                    cwd: None,
                    env: BTreeMap::new(),
                }),
            }],
        )
        .unwrap(),
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/agents/instances/managed/launch")
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
        json.pointer("/action").and_then(serde_json::Value::as_str),
        Some("launch")
    );
    assert_eq!(
        json.pointer("/snapshot/process/launched_by_agents")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert!(json
        .pointer("/snapshot/process/pid")
        .and_then(serde_json::Value::as_u64)
        .is_some());

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/agents/instances/managed/stop")
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
        json.pointer("/action").and_then(serde_json::Value::as_str),
        Some("stop")
    );
    assert_eq!(
        json.pointer("/process_result/remaining_pids")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(0)
    );
    assert!(json
        .pointer("/snapshot/process")
        .is_some_and(serde_json::Value::is_null));
}

#[tokio::test]
async fn launch_rejects_attached() {
    let gateway = start_mock_santi().await;

    let app = build_router(AppState::single_santi("test", gateway));
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/agents/instances/local-santi/launch")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn instances_support_multiple_santi() {
    let alpha = start_mock_santi().await;
    let beta = start_mock_santi().await;

    let app = build_router(
        AppState::santi_instances(
            "test",
            vec![
                SantiInstanceConfig {
                    agent_id: Some("santi-alpha".into()),
                    participant_id: Some("participant-santi".into()),
                    delivery_endpoint_id: Some("endpoint-alpha".into()),
                    id: "alpha".into(),
                    label: "Alpha".into(),
                    endpoint: alpha,
                    profile: Some("profile-a".into()),
                    managed: false,
                    launch: None,
                },
                SantiInstanceConfig {
                    agent_id: Some("santi-beta".into()),
                    participant_id: Some("participant-santi".into()),
                    delivery_endpoint_id: Some("endpoint-beta".into()),
                    id: "beta".into(),
                    label: "Beta".into(),
                    endpoint: beta,
                    profile: Some("profile-b".into()),
                    managed: false,
                    launch: None,
                },
            ],
        )
        .unwrap(),
    );
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
        Some("alpha")
    );
    assert_eq!(
        json.pointer("/instances/0/agent_id")
            .and_then(serde_json::Value::as_str),
        Some("santi-alpha")
    );
    assert_eq!(
        json.pointer("/instances/0/delivery_endpoint_id")
            .and_then(serde_json::Value::as_str),
        Some("endpoint-alpha")
    );
    assert_eq!(
        json.pointer("/instances/0/active")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        json.pointer("/instances/1/id")
            .and_then(serde_json::Value::as_str),
        Some("beta")
    );
    assert_eq!(
        json.pointer("/instances/1/active")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        json.pointer("/instances/1/profile")
            .and_then(serde_json::Value::as_str),
        Some("profile-b")
    );
}

#[tokio::test]
async fn selection_updates_active_santi() {
    let alpha = start_mock_santi().await;
    let beta = start_mock_santi().await;

    let app = build_router(
        AppState::santi_instances(
            "test",
            vec![
                SantiInstanceConfig {
                    agent_id: Some("santi-alpha".into()),
                    participant_id: Some("participant-santi".into()),
                    delivery_endpoint_id: Some("endpoint-alpha".into()),
                    id: "alpha".into(),
                    label: "Alpha".into(),
                    endpoint: alpha,
                    profile: Some("profile-a".into()),
                    managed: false,
                    launch: None,
                },
                SantiInstanceConfig {
                    agent_id: Some("santi-beta".into()),
                    participant_id: Some("participant-santi".into()),
                    delivery_endpoint_id: Some("endpoint-beta".into()),
                    id: "beta".into(),
                    label: "Beta".into(),
                    endpoint: beta,
                    profile: Some("profile-b".into()),
                    managed: false,
                    launch: None,
                },
            ],
        )
        .unwrap(),
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/v1/agents/selection")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"instance_id":"beta"}"#))
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
        Some("beta")
    );

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
        json.pointer("/instances/0/active")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        json.pointer("/instances/1/active")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
}
