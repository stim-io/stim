use super::support::*;

#[test]
fn snapshot_reports_env_targets() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };

    let handle = spawn_local_controller(Some("test-detail")).unwrap();
    let snapshot = handle.snapshot();
    let detail = snapshot.detail.unwrap_or_default();

    assert!(detail.contains("stim-server env-override via STIM_SERVER_BASE_URL ->"));
    assert!(detail.contains(&stim_server_base_url));
    assert!(detail.contains("target santi env-override via SANTI_BASE_URL ->"));
    assert!(detail.contains(&santi_base_url));

    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

#[test]
fn exposes_discovery_registry() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };
    let handle = spawn_local_controller(Some("test-registry")).unwrap();
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
                    "GET /api/v1/debug/registry HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(request.as_bytes()).unwrap();
                stream.read_to_string(&mut response).unwrap();
                break;
            }
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    assert!(response.contains("200 OK"));
    assert!(response.contains("endpoint-a"));
    assert!(response.contains("endpoint-b"));
    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}
