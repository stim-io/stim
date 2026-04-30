use crate::{
    bridge::write_renderer_delivery_bridge,
    cli::{RUN_RENDERER_COMMAND, RUN_TAURI_COMMAND},
    plan::{PackagedSidecarEntry, PackagedSidecarPlan},
    runner::{
        spawn_controller_ready, spawn_runner_ready, spawn_runner_ready_with_env,
        CONTROLLER_ENDPOINT_ENV, CONTROLLER_INSTANCE_ENV,
    },
};

pub(crate) fn launch_packaged_sidecar(plan: &PackagedSidecarPlan, app: &str) -> Result<(), String> {
    if app == "all" {
        return launch_all(plan);
    }

    let sidecar = plan
        .sidecars
        .iter()
        .find(|sidecar| sidecar.stamp.app == app)
        .ok_or_else(|| format!("unknown packaged sidecar app: {app}"))?;

    match app {
        "controller" => launch_controller(sidecar),
        "renderer" => launch_runner(RUN_RENDERER_COMMAND, sidecar),
        "tauri" => launch_runner(RUN_TAURI_COMMAND, sidecar),
        _ => Err(format!("unknown packaged sidecar app: {app}")),
    }
}

fn launch_all(plan: &PackagedSidecarPlan) -> Result<(), String> {
    let renderer = plan
        .sidecars
        .iter()
        .find(|sidecar| sidecar.stamp.app == "renderer")
        .ok_or_else(|| "packaged renderer sidecar plan is missing".to_string())?;
    let tauri = plan
        .sidecars
        .iter()
        .find(|sidecar| sidecar.stamp.app == "tauri")
        .ok_or_else(|| "packaged tauri sidecar plan is missing".to_string())?;
    let controller = plan
        .sidecars
        .iter()
        .find(|sidecar| sidecar.stamp.app == "controller")
        .ok_or_else(|| "packaged controller sidecar plan is missing".to_string())?;
    let mut children = Vec::new();
    let mut ready_lines = Vec::new();

    let (renderer_child, renderer_ready) = spawn_runner_ready(RUN_RENDERER_COMMAND, renderer)?;
    children.push((renderer.stamp.app.clone(), renderer_child));
    let renderer_url = renderer_ready
        .endpoint
        .clone()
        .ok_or_else(|| "packaged renderer ready line did not include endpoint".to_string())?;
    write_renderer_delivery_bridge(
        &renderer.stamp.namespace,
        renderer.stamp.mode,
        &renderer_url,
        &renderer.stamp.source,
    )?;
    ready_lines.push(renderer_ready);

    let (controller_child, controller_ready) = spawn_controller_ready(controller)?;
    children.push((controller.stamp.app.clone(), controller_child));
    let controller_endpoint = controller_ready
        .endpoint
        .clone()
        .ok_or_else(|| "packaged controller ready line did not include endpoint".to_string())?;
    let controller_instance_id = controller_ready.instance_id.clone();
    ready_lines.push(controller_ready);

    let (tauri_child, tauri_ready) = spawn_runner_ready_with_env(
        RUN_TAURI_COMMAND,
        tauri,
        &[
            (CONTROLLER_ENDPOINT_ENV, controller_endpoint.as_str()),
            (CONTROLLER_INSTANCE_ENV, controller_instance_id.as_str()),
        ],
    )?;
    children.push((tauri.stamp.app.clone(), tauri_child));
    ready_lines.push(tauri_ready);

    let output = serde_json::to_string(&serde_json::json!({
        "kind": "stim-packaged-ready",
        "sidecars": ready_lines,
    }))
    .map_err(|error| format!("failed to serialize packaged ready lines: {error}"))?;
    println!("{output}");

    wait_for_children(children)
}

fn launch_controller(sidecar: &PackagedSidecarEntry) -> Result<(), String> {
    let (mut child, ready) = spawn_controller_ready(sidecar)?;

    let output = serde_json::to_string(&ready)
        .map_err(|error| format!("failed to serialize packaged controller ready line: {error}"))?;
    println!("{output}");

    let status = child
        .wait()
        .map_err(|error| format!("failed waiting for packaged controller sidecar: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "packaged controller sidecar exited with status {status}"
        ))
    }
}

fn launch_runner(command_name: &str, sidecar: &PackagedSidecarEntry) -> Result<(), String> {
    let (mut child, ready) = spawn_runner_ready(command_name, sidecar)?;

    let output = serde_json::to_string(&ready).map_err(|error| {
        format!(
            "failed to serialize packaged {} ready line: {error}",
            sidecar.stamp.app
        )
    })?;
    println!("{output}");

    let status = child.wait().map_err(|error| {
        format!(
            "failed waiting for packaged {} runner: {error}",
            sidecar.stamp.app
        )
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "packaged {} runner exited with status {status}",
            sidecar.stamp.app
        ))
    }
}

fn wait_for_children(children: Vec<(String, std::process::Child)>) -> Result<(), String> {
    let mut first_error: Option<String> = None;

    for (app, mut child) in children {
        match child.wait() {
            Ok(status) if status.success() => {}
            Ok(status) => {
                first_error.get_or_insert_with(|| {
                    format!("packaged {app} runner exited with status {status}")
                });
            }
            Err(error) => {
                first_error
                    .get_or_insert_with(|| format!("failed waiting for packaged {app}: {error}"));
            }
        }
    }

    first_error.map_or(Ok(()), Err)
}
