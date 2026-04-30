use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use stim_shared::control_plane::{
    namespace_or_default, ControllerRuntimeHeartbeat, ControllerRuntimeSnapshot,
    ControllerRuntimeState,
};

use crate::controller;

use super::{
    clock::timestamp_now,
    routes::build_router,
    stim_server::seed_stim_server_registry,
    targets::{resolve_santi_base_url, resolve_stim_server_base_url},
    types::{ControllerHttpState, ControllerServiceHandle},
};

pub fn spawn_local_controller(namespace: Option<&str>) -> Result<ControllerServiceHandle, String> {
    let namespace = namespace_or_default(namespace).to_string();
    let (stim_server_base_url, stim_server_target) = resolve_stim_server_base_url()?;
    let (santi_base_url, santi_target) = resolve_santi_base_url()?;

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
            "controller ready with stim-server {} ; target santi {}",
            stim_server_target.describe(&stim_server_base_url),
            santi_target.describe(&santi_base_url)
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
    let app = build_router(app_state);

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
