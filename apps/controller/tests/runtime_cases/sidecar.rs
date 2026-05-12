use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    process::{Child, Command, Stdio},
    time::Duration,
};

#[cfg(unix)]
use std::os::unix::net::UnixStream;

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
    let capabilities = runtime_event(&endpoint, "capabilities", Value::Null);
    assert!(capabilities
        .get("events")
        .and_then(Value::as_array)
        .is_some_and(|events| events
            .iter()
            .any(|event| event.as_str() == Some("runtime.snapshot"))));
    assert!(capabilities
        .get("events")
        .and_then(Value::as_array)
        .is_some_and(|events| events
            .iter()
            .any(|event| event.as_str() == Some("accept.messaging"))));

    let snapshot = runtime_event(&endpoint, "runtime.snapshot", Value::Null);
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

    let heartbeat = runtime_event(&endpoint, "runtime.heartbeat", Value::Null);
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

#[test]
fn controller_accept_messaging() {
    let _guard = ENV_LOCK.lock().unwrap();
    let first_text = "controller accept first marker";
    let followup_text = "quote the controller accept first marker";
    let stim_server_base_url = spawn_test_stim_server();
    let santi_base_url = spawn_acceptance_santi_server(first_text, followup_text);
    unsafe { std::env::set_var("STIM_SERVER_BASE_URL", &stim_server_base_url) };
    unsafe { std::env::set_var("SANTI_BASE_URL", &santi_base_url) };

    let (_child, endpoint, _namespace) = spawn_controller_sidecar();
    let result = runtime_event(
        &endpoint,
        "accept.messaging",
        json!({
            "text": first_text,
            "followupText": followup_text,
        }),
    );

    assert_eq!(result.get("state").and_then(Value::as_str), Some("passed"));
    assert_eq!(
        result.get("submitted_text").and_then(Value::as_str),
        Some(first_text)
    );
    assert_eq!(
        result.get("followup_text").and_then(Value::as_str),
        Some(followup_text)
    );
    assert_eq!(result.get("turn_count").and_then(Value::as_u64), Some(2));
    assert!(result
        .pointer("/final_reload/snapshot/last_assistant_text")
        .and_then(Value::as_str)
        .is_some_and(|text| text.contains(first_text)));

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
    assert_runtime_endpoint_shape(&endpoint);
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
    let mut reader = BufReader::new(stream.try_clone_reader());
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

enum RuntimeStream {
    Tcp(TcpStream),
    #[cfg(unix)]
    Unix(UnixStream),
}

impl RuntimeStream {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            Self::Tcp(stream) => stream.set_read_timeout(timeout),
            #[cfg(unix)]
            Self::Unix(stream) => stream.set_read_timeout(timeout),
        }
    }

    fn set_write_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            Self::Tcp(stream) => stream.set_write_timeout(timeout),
            #[cfg(unix)]
            Self::Unix(stream) => stream.set_write_timeout(timeout),
        }
    }

    fn try_clone_reader(&self) -> Box<dyn std::io::Read> {
        match self {
            Self::Tcp(stream) => Box::new(stream.try_clone().expect("clone tcp stream")),
            #[cfg(unix)]
            Self::Unix(stream) => Box::new(stream.try_clone().expect("clone unix stream")),
        }
    }
}

impl Write for RuntimeStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.write(buf),
            #[cfg(unix)]
            Self::Unix(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Tcp(stream) => stream.flush(),
            #[cfg(unix)]
            Self::Unix(stream) => stream.flush(),
        }
    }
}

fn connect_runtime(endpoint: &str) -> RuntimeStream {
    #[cfg(unix)]
    if let Some(path) = endpoint.strip_prefix("unix://") {
        for _ in 0..20 {
            match UnixStream::connect(path) {
                Ok(stream) => return RuntimeStream::Unix(stream),
                Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }

        return RuntimeStream::Unix(
            UnixStream::connect(path).expect("connect unix runtime endpoint"),
        );
    }

    let address = endpoint.strip_prefix("tcp://").unwrap_or(endpoint);
    for _ in 0..20 {
        match TcpStream::connect(address) {
            Ok(stream) => return RuntimeStream::Tcp(stream),
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }

    RuntimeStream::Tcp(TcpStream::connect(address).expect("connect tcp runtime endpoint"))
}

#[cfg(unix)]
fn assert_runtime_endpoint_shape(endpoint: &str) {
    assert!(endpoint.starts_with("unix:///"), "{endpoint}");
}

#[cfg(not(unix))]
fn assert_runtime_endpoint_shape(endpoint: &str) {
    assert!(endpoint.starts_with("tcp://"), "{endpoint}");
}
