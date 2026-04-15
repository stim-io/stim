use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use axum::{extract::State, http::StatusCode, routing::get, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use stim_proto::DiscoveryRecord;
use stim_shared::control_plane::{
    namespace_or_default, ControllerRuntimeHeartbeat, ControllerRuntimeSnapshot,
    ControllerRuntimeState,
};

use crate::controller;

const DEFAULT_COMPOSE_STIM_SERVER_BASE_URL: &str = "http://127.0.0.1:18083";
const DEFAULT_COMPOSE_SANTI_BASE_URL: &str = "http://127.0.0.1:18081";

#[derive(Debug, Clone)]
pub struct ControllerServiceHandle {
    snapshot: Arc<Mutex<ControllerRuntimeSnapshot>>,
    heartbeat: Arc<Mutex<ControllerRuntimeHeartbeat>>,
}

impl ControllerServiceHandle {
    pub fn snapshot(&self) -> ControllerRuntimeSnapshot {
        self.snapshot.lock().expect("snapshot poisoned").clone()
    }

    pub fn heartbeat(&self) -> ControllerRuntimeHeartbeat {
        self.heartbeat.lock().expect("heartbeat poisoned").clone()
    }
}

#[derive(Debug, Clone)]
pub struct ControllerHttpState {
    snapshot: Arc<Mutex<ControllerRuntimeSnapshot>>,
    stim_server_base_url: String,
    registered_endpoint_ids: Arc<Mutex<Vec<String>>>,
    self_discovery: DiscoveryRecord,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FirstMessageRequest {
    pub text: String,
    pub target_endpoint_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FirstMessageResponse {
    pub target_endpoint_id: String,
    pub sent_text: String,
    pub response_text: String,
    pub sent_envelope_id: String,
    pub response_envelope_id: String,
    pub receipt_result: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegistrySnapshotResponse {
    pub endpoints: Vec<String>,
}

pub fn spawn_local_controller(namespace: Option<&str>) -> Result<ControllerServiceHandle, String> {
    let namespace = namespace_or_default(namespace).to_string();
    let (stim_server_base_url, stim_server_mode) = resolve_stim_server_base_url()?;
    let (santi_base_url, santi_mode) = resolve_santi_base_url()?;

    let std_listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("failed to bind controller listener: {error}"))?;
    let local_addr = std_listener
        .local_addr()
        .map_err(|error| format!("failed to read controller listener addr: {error}"))?;
    std_listener
        .set_nonblocking(true)
        .map_err(|error| format!("failed to set controller listener nonblocking: {error}"))?;
    let discovery_fixture = controller::http_santi_discovery_fixture(
        &format!("controller-{}", local_addr.port()),
        &santi_base_url,
    );
    seed_stim_server_registry(
        &stim_server_base_url,
        &discovery_fixture.self_discovery,
        &discovery_fixture.peer_discovery,
    )?;

    let snapshot = Arc::new(Mutex::new(ControllerRuntimeSnapshot {
        namespace: namespace.clone(),
        instance_id: format!("controller-{}", local_addr.port()),
        published_at: timestamp_now(),
        state: ControllerRuntimeState::Ready,
        http_base_url: Some(format!("http://{local_addr}")),
        detail: Some(format!(
            "controller ready via {stim_server_mode} at {stim_server_base_url}; target santi via {santi_mode} at {santi_base_url}"
        )),
    }));
    let heartbeat = Arc::new(Mutex::new(ControllerRuntimeHeartbeat {
        namespace: namespace.clone(),
        instance_id: format!("controller-{}", local_addr.port()),
        published_at: timestamp_now(),
        sequence: 0,
        state: ControllerRuntimeState::Ready,
    }));
    let registered_endpoint_ids = Arc::new(Mutex::new(vec![
        "endpoint-a".to_string(),
        "endpoint-b".to_string(),
    ]));

    let app_state = ControllerHttpState {
        snapshot: snapshot.clone(),
        stim_server_base_url: stim_server_base_url.clone(),
        registered_endpoint_ids: registered_endpoint_ids.clone(),
        self_discovery: discovery_fixture.self_discovery.clone(),
    };
    let app = Router::new()
        .route("/api/v1/health", get(health))
        .route(
            "/api/v1/discovery/endpoints/{endpoint_id}",
            get(discover_endpoint).put(register_endpoint),
        )
        .route("/api/v1/debug/registry", get(registry_snapshot))
        .route("/api/v1/messages/roundtrip", post(first_message_roundtrip))
        .with_state(app_state);

    let snapshot_for_thread = snapshot.clone();
    let heartbeat_for_thread = heartbeat.clone();
    thread::spawn(move || {
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(runtime) => runtime,
            Err(error) => {
                if let Ok(mut snapshot) = snapshot_for_thread.lock() {
                    snapshot.state = ControllerRuntimeState::Degraded;
                    snapshot.published_at = timestamp_now();
                    snapshot.detail = Some(format!("failed to create runtime: {error}"));
                }
                return;
            }
        };

        let heartbeat_state = heartbeat_for_thread.clone();
        runtime.spawn(async move {
            loop {
                if let Ok(mut heartbeat) = heartbeat_state.lock() {
                    heartbeat.sequence += 1;
                    heartbeat.published_at = timestamp_now();
                    heartbeat.state = ControllerRuntimeState::Ready;
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        runtime.block_on(async move {
            let listener = match tokio::net::TcpListener::from_std(std_listener) {
                Ok(listener) => listener,
                Err(error) => {
                    if let Ok(mut snapshot) = snapshot_for_thread.lock() {
                        snapshot.state = ControllerRuntimeState::Degraded;
                        snapshot.published_at = timestamp_now();
                        snapshot.detail = Some(format!("failed to convert listener: {error}"));
                    }
                    return;
                }
            };

            if let Err(error) = axum::serve(listener, app).await {
                if let Ok(mut snapshot) = snapshot_for_thread.lock() {
                    snapshot.state = ControllerRuntimeState::Degraded;
                    snapshot.published_at = timestamp_now();
                    snapshot.detail = Some(format!("controller HTTP server stopped: {error}"));
                }
            }
        });
    });

    Ok(ControllerServiceHandle {
        snapshot,
        heartbeat,
    })
}

fn resolve_stim_server_base_url() -> Result<(String, &'static str), String> {
    if let Ok(base_url) = std::env::var("STIM_SERVER_BASE_URL") {
        wait_for_health(&base_url)?;
        return Ok((base_url, "env-configured external"));
    }

    if wait_for_health(DEFAULT_COMPOSE_STIM_SERVER_BASE_URL).is_ok() {
        return Ok((
            DEFAULT_COMPOSE_STIM_SERVER_BASE_URL.into(),
            "compose-managed external",
        ));
    }

    Err(format!(
        "stim-server unavailable: set STIM_SERVER_BASE_URL or start docker-compose service at {}",
        DEFAULT_COMPOSE_STIM_SERVER_BASE_URL
    ))
}

fn resolve_santi_base_url() -> Result<(String, &'static str), String> {
    if let Ok(base_url) = std::env::var("SANTI_BASE_URL") {
        wait_for_health(&base_url)?;
        return Ok((base_url, "env-configured external"));
    }

    if wait_for_health(DEFAULT_COMPOSE_SANTI_BASE_URL).is_ok() {
        return Ok((
            DEFAULT_COMPOSE_SANTI_BASE_URL.into(),
            "compose-managed external",
        ));
    }

    Err(format!(
        "santi unavailable: set SANTI_BASE_URL or start docker-compose service at {}",
        DEFAULT_COMPOSE_SANTI_BASE_URL
    ))
}

async fn health() -> Json<&'static str> {
    Json("ok")
}

async fn first_message_roundtrip(
    State(state): State<ControllerHttpState>,
    Json(request): Json<FirstMessageRequest>,
) -> Result<Json<FirstMessageResponse>, (StatusCode, String)> {
    let stim_server_base_url = state.stim_server_base_url.clone();
    let target_endpoint_id = request.target_endpoint_id.clone();
    let text = request.text.clone();
    let self_discovery = state.self_discovery.clone();
    let summary = tokio::task::spawn_blocking(move || {
        controller::first_message_roundtrip_via_server(
            &stim_server_base_url,
            &target_endpoint_id,
            &text,
            self_discovery,
        )
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("controller blocking roundtrip join failed: {error}"),
        )
    })?
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error:?}")))?;

    if let Ok(mut snapshot) = state.snapshot.lock() {
        snapshot.published_at = timestamp_now();
        snapshot.detail = Some(format!(
            "last roundtrip ok for endpoint {} envelope {}",
            summary.endpoint_id, summary.envelope_id
        ));
    }

    Ok(Json(FirstMessageResponse {
        target_endpoint_id: request.target_endpoint_id,
        sent_text: request.text,
        response_text: summary.response_text,
        sent_envelope_id: summary.envelope_id,
        response_envelope_id: summary.response_envelope_id,
        receipt_result: format!("{:?}", summary.receipt_result).to_lowercase(),
    }))
}

async fn discover_endpoint(
    State(state): State<ControllerHttpState>,
    axum::extract::Path(endpoint_id): axum::extract::Path<String>,
) -> Result<Json<DiscoveryRecord>, (StatusCode, String)> {
    let stim_server_base_url = state.stim_server_base_url.clone();
    tokio::task::spawn_blocking(move || {
        discover_endpoint_via_server(&stim_server_base_url, &endpoint_id)
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("controller blocking discover join failed: {error}"),
        )
    })?
    .map(Json)
}

async fn register_endpoint(
    State(state): State<ControllerHttpState>,
    axum::extract::Path(endpoint_id): axum::extract::Path<String>,
    Json(record): Json<DiscoveryRecord>,
) -> Result<Json<DiscoveryRecord>, (StatusCode, String)> {
    if record.endpoint_declaration.endpoint_id != endpoint_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "endpoint_id path must match endpoint_declaration.endpoint_id".to_string(),
        ));
    }

    let stim_server_base_url = state.stim_server_base_url.clone();
    let record_for_register = record.clone();
    tokio::task::spawn_blocking(move || {
        register_endpoint_via_server(&stim_server_base_url, &record_for_register)
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("controller blocking register join failed: {error}"),
        )
    })??;

    let mut registered = state.registered_endpoint_ids.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "controller registry list poisoned".to_string(),
        )
    })?;
    if !registered.iter().any(|existing| existing == &endpoint_id) {
        registered.push(endpoint_id.clone());
        registered.sort();
    }

    Ok(Json(record))
}

async fn registry_snapshot(
    State(state): State<ControllerHttpState>,
) -> Result<Json<RegistrySnapshotResponse>, (StatusCode, String)> {
    let registered = state.registered_endpoint_ids.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "controller registry list poisoned".to_string(),
        )
    })?;

    Ok(Json(RegistrySnapshotResponse {
        endpoints: registered.clone(),
    }))
}

fn wait_for_health(base_url: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();

    for _ in 0..20 {
        match client.get(format!("{base_url}/api/v1/health")).send() {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(_) | Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }

    Err("stim-server health never became ready".into())
}

fn seed_stim_server_registry(
    base_url: &str,
    self_discovery: &DiscoveryRecord,
    peer_discovery: &DiscoveryRecord,
) -> Result<(), String> {
    register_endpoint_via_server(base_url, self_discovery).map_err(|(_, error)| error)?;
    register_endpoint_via_server(base_url, peer_discovery).map_err(|(_, error)| error)?;
    Ok(())
}

fn discover_endpoint_via_server(
    base_url: &str,
    endpoint_id: &str,
) -> Result<DiscoveryRecord, (StatusCode, String)> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(format!(
            "{base_url}/api/v1/discovery/endpoints/{endpoint_id}"
        ))
        .send()
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("stim-server discovery request failed: {error}"),
            )
        })?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err((StatusCode::NOT_FOUND, "endpoint not registered".into()));
    }

    response
        .error_for_status()
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("stim-server discovery status failed: {error}"),
            )
        })?
        .json::<DiscoveryRecord>()
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("stim-server discovery decode failed: {error}"),
            )
        })
}

fn register_endpoint_via_server(
    base_url: &str,
    record: &DiscoveryRecord,
) -> Result<(), (StatusCode, String)> {
    let client = reqwest::blocking::Client::new();
    client
        .put(format!(
            "{base_url}/api/v1/discovery/endpoints/{}",
            record.endpoint_declaration.endpoint_id
        ))
        .json(record)
        .send()
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("stim-server register request failed: {error}"),
            )
        })?
        .error_for_status()
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("stim-server register status failed: {error}"),
            )
        })?;
    Ok(())
}

fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}-{:03}", now.as_secs(), now.subsec_millis())
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::Mutex,
        thread,
        time::Duration,
    };

    use axum::{routing::get, routing::post, Json, Router};
    use stim_proto::{AcknowledgementResult, MessageEnvelope, ProtocolAcknowledgement};
    use stim_server::{
        app::build_router as build_stim_server_router, state::AppState as StimServerState,
    };

    use super::spawn_local_controller;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn spawn_test_stim_server() -> String {
        let std_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let local_addr = std_listener.local_addr().unwrap();
        std_listener.set_nonblocking(true).unwrap();

        thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
                let app = build_stim_server_router(StimServerState::in_memory());
                axum::serve(listener, app).await.unwrap();
            });
        });

        format!("http://{local_addr}")
    }

    fn spawn_test_santi_server() -> String {
        let std_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let local_addr = std_listener.local_addr().unwrap();
        std_listener.set_nonblocking(true).unwrap();

        thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
                let app = Router::new()
                    .route("/api/v1/health", get(|| async { Json("ok") }))
                    .route(
                        "/api/v1/stim/envelopes",
                        post(|Json(envelope): Json<MessageEnvelope>| async move {
                            Json(ProtocolAcknowledgement {
                                ack_envelope_id: format!("ack-{}", envelope.envelope_id),
                                ack_message_id: envelope.message_id,
                                ack_version: envelope.new_version,
                                ack_result: AcknowledgementResult::Applied,
                                detail: Some(format!(
                                    "santi session {} applied; output=hello from mock santi",
                                    envelope.conversation_id
                                )),
                            })
                        }),
                    );

                axum::serve(listener, app).await.unwrap();
            });
        });

        format!("http://{local_addr}")
    }

    #[test]
    fn spawned_controller_serves_first_message_roundtrip_over_http() {
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
        unsafe { std::env::remove_var("STIM_SERVER_BASE_URL") };
        unsafe { std::env::remove_var("SANTI_BASE_URL") };
    }

    #[test]
    fn spawned_controller_exposes_discovery_registry() {
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
}
