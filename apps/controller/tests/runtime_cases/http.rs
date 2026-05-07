use super::support::*;

#[test]
fn serves_first_http_roundtrip() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-http")).unwrap();
    let snapshot = handle.snapshot();
    let address = snapshot
        .http_base_url
        .unwrap()
        .trim_start_matches("http://")
        .to_string();

    let mut response = String::new();

    for _ in 0..20 {
        match TcpStream::connect(&address) {
            Ok(mut stream) => {
                let body = r#"{"text":"hello over http","target_endpoint_id":"endpoint-b"}"#;
                let request = format!(
                    "POST /api/v1/messages/roundtrip HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(request.as_bytes()).unwrap();
                stream.read_to_string(&mut response).unwrap();
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    assert!(response.contains("200 OK"));
    assert!(response.contains("hello from mock santi"));
    assert!(response.contains("accepted"));
    assert!(response.contains("endpoint-b"));
    let response_json = http_response_json(&response);
    let product_messages = fetch_product_chat_messages(
        &stim_server_base_url,
        response_json
            .pointer("/conversation_id")
            .and_then(serde_json::Value::as_str)
            .unwrap(),
    );
    assert_eq!(
        product_messages
            .pointer("/messages/0/text")
            .and_then(serde_json::Value::as_str),
        Some("hello over http")
    );
    assert_eq!(
        product_messages
            .pointer("/messages/1/chunks/0/text")
            .and_then(serde_json::Value::as_str),
        Some("hello from mock santi")
    );
    let snapshot_detail = handle.snapshot().detail.unwrap_or_default();
    assert!(snapshot_detail.contains("stim-server env-override via STIM_SERVER_BASE_URL ->"));
    assert!(snapshot_detail.contains("target santi env-override via SANTI_BASE_URL ->"));
    assert!(snapshot_detail.contains("last roundtrip ok for endpoint endpoint-b envelope"));
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

#[test]
fn resolves_http_delivery_target() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    register_test_agent_participant(&stim_server_base_url, "santi");
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-http-participant")).unwrap();
    let snapshot = handle.snapshot();
    let address = snapshot
        .http_base_url
        .unwrap()
        .trim_start_matches("http://")
        .to_string();

    let mut response = String::new();

    for _ in 0..20 {
        match TcpStream::connect(&address) {
            Ok(mut stream) => {
                let body = r#"{"text":"hello by participant","target_endpoint_id":"not-used","participant_id":"santi"}"#;
                let request = format!(
                    "POST /api/v1/messages/roundtrip HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(request.as_bytes()).unwrap();
                stream.read_to_string(&mut response).unwrap();
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    assert!(response.contains("200 OK"));
    assert!(response.contains("hello from mock santi"));
    assert!(response.contains(r#""target_endpoint_id":"endpoint-b""#));
    assert!(response.contains(r#""participant_id":"santi""#));
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

#[test]
fn serves_http_transcript() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-transcript")).unwrap();
    let snapshot = handle.snapshot();
    let address = snapshot
        .http_base_url
        .unwrap()
        .trim_start_matches("http://")
        .to_string();

    let mut response = String::new();

    for _ in 0..20 {
        match TcpStream::connect(&address) {
            Ok(mut stream) => {
                let request = format!(
                    "GET /api/v1/conversations/conv-1/messages HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(request.as_bytes()).unwrap();
                stream.read_to_string(&mut response).unwrap();
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    assert!(response.contains("200 OK"));
    assert!(response.contains("hello from persisted transcript"));
    assert!(response.contains("hello from mock santi"));
    assert!(response.contains("\"role\":\"user\""));
    assert!(response.contains("\"role\":\"assistant\""));
    assert!(response.contains("\"tool_activities\""));
    assert!(response.contains("bash exit 0; stdout 5 chars; stderr 0 chars"));
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

#[test]
fn recovers_transient_transcript() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_santi_fail_server(1);
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-transcript-retry")).unwrap();
    let snapshot = handle.snapshot();
    let address = snapshot
        .http_base_url
        .unwrap()
        .trim_start_matches("http://")
        .to_string();

    let mut response = String::new();

    for _ in 0..20 {
        match TcpStream::connect(&address) {
            Ok(mut stream) => {
                let request = format!(
                    "GET /api/v1/conversations/conv-1/messages HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(request.as_bytes()).unwrap();
                stream.read_to_string(&mut response).unwrap();
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    assert!(response.contains("200 OK"));
    assert!(response.contains("hello from persisted transcript"));
    assert!(response.contains("hello from mock santi"));
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

#[test]
fn maps_transcript_not_found() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_santi_flaky_server(100, StatusCode::NOT_FOUND);
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-transcript-not-found")).unwrap();
    let snapshot = handle.snapshot();
    let address = snapshot
        .http_base_url
        .unwrap()
        .trim_start_matches("http://")
        .to_string();

    let mut response = String::new();

    for _ in 0..20 {
        match TcpStream::connect(&address) {
            Ok(mut stream) => {
                let request = format!(
                    "GET /api/v1/conversations/missing/messages HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(request.as_bytes()).unwrap();
                stream.read_to_string(&mut response).unwrap();
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    assert!(response.contains("404 Not Found"));
    assert!(response.contains("fetch status failed: HTTP 404 Not Found"));
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}
