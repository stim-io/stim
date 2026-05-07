use super::support::*;

#[tokio::test]
async fn profiles_list_safe_profiles() {
    let app = build_router(AppState::single_santi(
        "test",
        "http://127.0.0.1:18081".into(),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents/profiles")
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
        json.pointer("/profiles/0/id")
            .and_then(serde_json::Value::as_str),
        Some("local")
    );
    assert_eq!(
        json.pointer("/profiles/0/secret_state")
            .and_then(serde_json::Value::as_str),
        Some("available")
    );
    assert!(!serde_json::to_string(&json)
        .unwrap()
        .contains("codex-local-dev"));
}

#[tokio::test]
async fn apply_profile_calls_santi() {
    let gateway = start_mock_santi().await;
    let app = build_router(
        AppState::santi_instances_with_profiles(
            "test",
            vec![SantiInstanceConfig {
                agent_id: Some("santi".into()),
                participant_id: Some("participant-santi".into()),
                delivery_endpoint_id: Some("endpoint-b".into()),
                id: "local-santi".into(),
                label: "Local Santi".into(),
                endpoint: gateway,
                profile: Some("local".into()),
                managed: false,
                launch: None,
            }],
            vec![SantiProfileConfig {
                id: "test-profile".into(),
                label: "Test Profile".into(),
                launch_profile: "test-launch".into(),
                provider: SantiProfileProviderConfig {
                    api: "responses".into(),
                    model: "gpt-test".into(),
                    gateway_base_url: "http://127.0.0.1:18082/openai/v1".into(),
                    api_key: SantiProfileSecretConfig::Value("secret-test-key".into()),
                },
            }],
        )
        .unwrap(),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/agents/instances/local-santi/profiles/apply")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"profile_id":"test-profile"}"#))
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
        json.pointer("/profile_id")
            .and_then(serde_json::Value::as_str),
        Some("test-profile")
    );
    assert_eq!(
        json.pointer("/status").and_then(serde_json::Value::as_str),
        Some("applied")
    );
    assert_eq!(
        json.pointer("/config_version")
            .and_then(serde_json::Value::as_u64),
        Some(3)
    );
    assert_eq!(
        json.pointer("/snapshot/config/config_version")
            .and_then(serde_json::Value::as_u64),
        Some(2)
    );
    assert!(!serde_json::to_string(&json)
        .unwrap()
        .contains("secret-test-key"));
}
