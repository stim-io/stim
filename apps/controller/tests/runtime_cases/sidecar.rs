use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    process::{Child, Command, Stdio},
    time::Duration,
};

use serde_json::{json, Value};
use stim_sidecar::{
    identity::{SidecarMode, SidecarStamp},
    ready::wait_for_ready_line,
    stamp::create_stamp_args,
};

use super::support::*;

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn controller_sidecar_runtime_smoke() {
    let _guard = ENV_LOCK.lock().unwrap();
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_test_santi_server();
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };

    let (_child, endpoint, namespace) = spawn_controller_sidecar();
    let snapshot = runtime_event(&endpoint, "controller-runtime", Value::Null);
    assert_eq!(
        snapshot.get("namespace").and_then(Value::as_str),
        Some(namespace.as_str())
    );
    assert_eq!(snapshot.get("state").and_then(Value::as_str), Some("ready"));
    assert_eq!(
        snapshot
            .get("http_base_url")
            .and_then(Value::as_str)
            .map(|value| value.starts_with("http://127.0.0.1:")),
        Some(true)
    );

    let heartbeat = runtime_event(&endpoint, "controller-heartbeat", Value::Null);
    assert_eq!(
        heartbeat.get("namespace").and_then(Value::as_str),
        Some(namespace.as_str())
    );
    assert_eq!(
        heartbeat.get("state").and_then(Value::as_str),
        Some("ready")
    );

    unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
    unsafe { std::env::remove_var("SANTI_BASE_URL") };
}

fn spawn_controller_sidecar() -> (ChildGuard, String, String) {
    let namespace = format!("runtime-socket-{}", std::process::id());
    let stamp = SidecarStamp {
        app: "controller".into(),
        namespace: namespace.clone(),
        mode: SidecarMode::Dev,
        source: "test:runtime-socket".into(),
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_stim-controller"))
        .arg("serve")
        .args(create_stamp_args(&stamp))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn controller sidecar");
    let stdout = child.stdout.take().expect("controller stdout");
    let ready = wait_for_ready_line(stdout, Duration::from_secs(20)).expect("controller ready");

    assert_eq!(ready.stamp, stamp);
    assert_eq!(ready.role, "controller-runtime");
    let endpoint = ready
        .runtime_endpoint
        .expect("controller ready line should include runtime endpoint");
    (ChildGuard(child), endpoint, namespace)
}

fn runtime_event(endpoint: &str, verb: &str, payload: Value) -> Value {
    let mut stream = connect_runtime(endpoint);
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("set read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .expect("set write timeout");
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let frame = json!({
        "kind": "event",
        "id": format!("event-{verb}"),
        "verb": verb,
        "payload": payload,
    });
    let mut body = serde_json::to_vec(&frame).expect("encode frame");
    body.push(b'\n');
    stream.write_all(&body).expect("write frame");
    stream.flush().expect("flush frame");

    let mut line = String::new();
    reader.read_line(&mut line).expect("read frame");
    let response: Value = serde_json::from_str(line.trim()).expect("decode frame");
    assert_eq!(
        response.get("kind").and_then(Value::as_str),
        Some("event_response")
    );
    response.get("payload").cloned().unwrap_or(Value::Null)
}

fn connect_runtime(endpoint: &str) -> TcpStream {
    for _ in 0..20 {
        match TcpStream::connect(endpoint) {
            Ok(stream) => return stream,
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }

    TcpStream::connect(endpoint).expect("connect runtime endpoint")
}
