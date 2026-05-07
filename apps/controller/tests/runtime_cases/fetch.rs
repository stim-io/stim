use super::support::*;

#[test]
fn fetch_returns_retry_meta() {
    let santi_base_url = spawn_santi_fail_server(1);

    let result = fetch_santi_conversation_messages(&santi_base_url, "conv-1").unwrap();

    assert_eq!(result.metadata.attempts, 2);
    assert_eq!(result.metadata.retries, 1);
    assert_eq!(result.metadata.last_status, Some(200));
    assert_eq!(result.payload.messages.len(), 2);
}

#[test]
fn retries_initial_projection_gap() {
    let santi_base_url = spawn_santi_flaky_server(1, StatusCode::NOT_FOUND);

    let result = fetch_santi_conversation_messages(&santi_base_url, "conv-1").unwrap();

    assert_eq!(result.metadata.attempts, 2);
    assert_eq!(result.metadata.retries, 1);
    assert_eq!(result.metadata.last_status, Some(200));
    assert_eq!(result.payload.messages.len(), 2);
}

#[test]
fn retry_disabled_by_default() {
    let santi_base_url = spawn_santi_fail_server(1);
    let result = fetch::FetchClient::new(&santi_base_url).get_json::<SantiSessionMessagesResponse>(
        "/api/v1/sessions/conv-1/messages",
        fetch::FetchRequestOptions::default(),
    );

    let error = result.unwrap_err();

    assert_eq!(error.metadata.attempts, 1);
    assert_eq!(error.metadata.retries, 0);
    assert_eq!(error.metadata.last_status, Some(502));
}

#[test]
fn retry_supports_decision_closure() {
    let santi_base_url = spawn_santi_fail_server(1);
    let result = fetch::FetchClient::new(&santi_base_url)
        .get_json::<SantiSessionMessagesResponse>(
            "/api/v1/sessions/conv-1/messages",
            fetch::FetchRequestOptions::default().with_retry(fetch::FetchRetry::custom(
                fetch::FetchRetryPolicy::new(2, 0, 0),
                |context| {
                    if context.attempt == 1
                        && context.method == reqwest::Method::GET
                        && context.path.ends_with("/messages")
                        && context.status == Some(502)
                    {
                        fetch::FetchRetryDecision::RetryAfter(Duration::from_millis(0))
                    } else {
                        fetch::FetchRetryDecision::Fail
                    }
                },
            )),
        )
        .unwrap();

    assert_eq!(result.metadata.attempts, 2);
    assert_eq!(result.metadata.retries, 1);
    assert_eq!(result.payload.messages.len(), 2);
}

#[test]
fn not_found_payload_explicit() {
    let santi_base_url = spawn_test_santi_server();
    let result = fetch::FetchClient::new(&santi_base_url)
        .get_json::<serde_json::Value>(
            "/api/v1/missing",
            fetch::FetchRequestOptions::default()
                .with_not_found_payload(|| serde_json::json!({ "state": "missing" })),
        )
        .unwrap();

    assert_eq!(result.metadata.attempts, 1);
    assert_eq!(result.metadata.retries, 0);
    assert_eq!(result.metadata.last_status, Some(404));
    assert_eq!(result.payload, serde_json::json!({ "state": "missing" }));
}

#[test]
fn not_found_after_retry() {
    let santi_base_url = spawn_santi_flaky_server(1, StatusCode::NOT_FOUND);
    let result = fetch::FetchClient::new(&santi_base_url)
        .get_json::<serde_json::Value>(
            "/api/v1/sessions/conv-1/messages",
            fetch::FetchRequestOptions::default()
                .with_retry(fetch::FetchRetry::custom(
                    fetch::FetchRetryPolicy::new(2, 0, 0),
                    |context| {
                        if context.status == Some(404) && context.path.ends_with("/messages") {
                            fetch::FetchRetryDecision::Retry
                        } else {
                            fetch::FetchRetryDecision::Fail
                        }
                    },
                ))
                .with_not_found_payload(|| serde_json::json!({ "state": "missing" })),
        )
        .unwrap();

    assert_eq!(result.metadata.attempts, 2);
    assert_eq!(result.metadata.retries, 1);
    assert_eq!(result.metadata.last_status, Some(200));
    assert!(result.payload.get("messages").is_some());
}

#[test]
fn options_are_request_local() {
    let santi_base_url = spawn_test_santi_server();
    let result = fetch::FetchClient::new(&santi_base_url)
        .get_json::<String>(
            "/api/v1/health",
            fetch::FetchRequestOptions::default()
                .with_timeout(Duration::from_secs(5))
                .with_header(
                    reqwest::header::HeaderName::from_static("x-stim-fetch-test"),
                    reqwest::header::HeaderValue::from_static("1"),
                )
                .with_query_param("probe", "1")
                .with_status_policy(fetch::FetchStatusPolicy::custom(|status| {
                    status == reqwest::StatusCode::OK
                })),
        )
        .unwrap();

    assert_eq!(result.payload, "ok");
    assert_eq!(result.metadata.attempts, 1);
    assert_eq!(result.metadata.last_status, Some(200));
}

#[test]
fn retry_skips_accepted_status() {
    let santi_base_url = spawn_test_santi_server();
    let result = fetch::FetchClient::new(&santi_base_url)
        .get_json::<String>(
            "/api/v1/health",
            fetch::FetchRequestOptions::default().with_retry(fetch::FetchRetry::custom(
                fetch::FetchRetryPolicy::new(2, 0, 0),
                |_| panic!("retry decision should not run for accepted status"),
            )),
        )
        .unwrap();

    assert_eq!(result.payload, "ok");
    assert_eq!(result.metadata.attempts, 1);
    assert_eq!(result.metadata.retries, 0);
}
