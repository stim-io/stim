use axum::http::StatusCode;
use stim_proto::DiscoveryRecord;

pub(crate) fn seed_stim_server_registry(
    base_url: &str,
    self_discovery: &DiscoveryRecord,
    peer_discovery: &DiscoveryRecord,
) -> Result<(), String> {
    register_endpoint_via_server(base_url, self_discovery).map_err(|(_, error)| error)?;
    register_endpoint_via_server(base_url, peer_discovery).map_err(|(_, error)| error)?;
    Ok(())
}

pub(crate) fn discover_endpoint_via_server(
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

pub(crate) fn register_endpoint_via_server(
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
