use axum::{extract::State, http::StatusCode, Json};
use stim_proto::DiscoveryRecord;

use crate::{
    client::{discover_endpoint_via_server, register_endpoint_via_server},
    model::{ControllerHttpState, RegistrySnapshotResponse},
};

pub(super) async fn discover_endpoint(
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

pub(super) async fn register_endpoint(
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

pub(super) async fn registry_snapshot(
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
