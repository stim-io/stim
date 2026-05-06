use stim_shared::inspection::RendererProbeResult;

pub(super) fn process_list_json(
    processes: &[stim_platform::process::ProcessSnapshot],
) -> serde_json::Value {
    serde_json::Value::Array(
        processes
            .iter()
            .map(|process| {
                serde_json::json!({
                    "pid": process.pid,
                    "ppid": process.ppid,
                    "command": process.command,
                })
            })
            .collect(),
    )
}

pub(super) fn bridge_result_json<T: serde::Serialize>(
    result: Result<T, String>,
) -> serde_json::Value {
    match result {
        Ok(value) => serde_json::json!({ "state": "available", "value": value }),
        Err(error) => serde_json::json!({ "state": "unavailable", "detail": error }),
    }
}

pub(super) fn renderer_probe_result_json(
    result: Result<RendererProbeResult, String>,
) -> serde_json::Value {
    match result {
        Ok(RendererProbeResult::Success { snapshot }) => {
            serde_json::json!({ "state": "available", "value": snapshot })
        }
        Ok(RendererProbeResult::Failure { reason }) => serde_json::json!({
            "state": "unavailable",
            "detail": format!("renderer probe failed: {:?}", reason),
        }),
        Err(error) => serde_json::json!({ "state": "unavailable", "detail": error }),
    }
}
