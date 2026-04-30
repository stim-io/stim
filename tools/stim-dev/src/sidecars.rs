use std::{
    fs,
    process::{Command, Stdio},
};

use stim_shared::{
    control_plane::RendererDeliveryLaunchBridge,
    paths::{renderer_delivery_launch_bridge_path, tauri_app_dir},
};
use stim_sidecar::{
    identity::{
        SidecarMode, SidecarStamp, SIDECAR_MODE_ENV, SIDECAR_NAMESPACE_ENV, SOURCE_TOOL_STIM_DEV,
    },
    ready::{wait_for_ready_line, SidecarReadyLine},
    stamp::create_stamp_args,
};

use crate::{clock::timestamp_now, runtime_control::current_namespace};

pub(crate) fn start_tauri() -> Result<(), String> {
    let namespace = current_namespace();
    write_renderer_delivery_bridge(
        &namespace,
        SidecarMode::Dev,
        stim_shared::RENDERER_DEV_URL,
        SOURCE_TOOL_STIM_DEV,
    )?;
    run_tauri_foreground(&namespace, &[])
}

pub(crate) fn start_renderer_foreground(force: bool) -> Result<(), String> {
    let namespace = current_namespace();
    let mut args = vec![
        "run".to_string(),
        "-p".into(),
        "stim-renderer".into(),
        "--".into(),
        "serve".into(),
        "--dev".into(),
    ];
    if force {
        args.push("--force".into());
    }
    args.extend(create_stamp_args(&renderer_stamp(&namespace)));

    run_cargo_owned(&stim_platform::paths::workspace_root(), &args)
}

pub(crate) fn start_controller() -> Result<(), String> {
    let namespace = current_namespace();
    let stamp = controller_stamp(&namespace);
    let mut args = vec![
        "run".to_string(),
        "-p".into(),
        "stim-controller".into(),
        "--".into(),
        "serve".into(),
    ];

    args.extend(create_stamp_args(&stamp));

    run_cargo_owned(&stim_platform::paths::workspace_root(), &args)
}

pub(crate) fn spawn_renderer_ready(
    namespace: &str,
    force: bool,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_renderer_ready_with_stdio(namespace, force, false)
}

pub(crate) fn spawn_renderer_ready_detached(
    namespace: &str,
    force: bool,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_renderer_ready_with_stdio(namespace, force, true)
}

pub(crate) fn spawn_controller_ready(
    namespace: &str,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_controller_ready_with_stdio(namespace, false)
}

pub(crate) fn spawn_controller_ready_detached(
    namespace: &str,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_controller_ready_with_stdio(namespace, true)
}

pub(crate) fn spawn_tauri(
    namespace: &str,
    envs: &[(&str, &str)],
) -> Result<std::process::Child, String> {
    spawn_tauri_with_stdio(namespace, envs, false)
}

pub(crate) fn spawn_tauri_detached(
    namespace: &str,
    envs: &[(&str, &str)],
) -> Result<std::process::Child, String> {
    spawn_tauri_with_stdio(namespace, envs, true)
}

pub(crate) fn write_renderer_delivery_bridge(
    namespace: &str,
    mode: SidecarMode,
    renderer_url: &str,
    source: &str,
) -> Result<(), String> {
    let bridge = RendererDeliveryLaunchBridge {
        namespace: namespace.into(),
        renderer_url: renderer_url.into(),
        source: source.into(),
        published_at: timestamp_now(),
    };
    let path = renderer_delivery_launch_bridge_path(mode.as_str(), namespace);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create renderer delivery bridge dir: {error}"))?;
    }
    let body = serde_json::to_string_pretty(&bridge)
        .map_err(|error| format!("failed to serialize renderer delivery bridge: {error}"))?;
    fs::write(&path, format!("{body}\n"))
        .map_err(|error| format!("failed to write renderer delivery bridge: {error}"))
}

pub(crate) fn wait_children(children: Vec<(String, std::process::Child)>) -> Result<(), String> {
    let mut first_error = None;
    for (name, mut child) in children {
        match child.wait() {
            Ok(status) if status.success() => {}
            Ok(status) => {
                first_error.get_or_insert_with(|| format!("{name} exited with status {status}"));
            }
            Err(error) => {
                first_error.get_or_insert_with(|| format!("failed waiting for {name}: {error}"));
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn renderer_stamp(namespace: &str) -> SidecarStamp {
    SidecarStamp {
        app: "renderer".into(),
        namespace: namespace.into(),
        mode: SidecarMode::Dev,
        source: SOURCE_TOOL_STIM_DEV.into(),
    }
}

fn controller_stamp(namespace: &str) -> SidecarStamp {
    SidecarStamp {
        app: "controller".into(),
        namespace: namespace.into(),
        mode: SidecarMode::Dev,
        source: SOURCE_TOOL_STIM_DEV.into(),
    }
}

fn tauri_stamp(namespace: &str) -> SidecarStamp {
    SidecarStamp {
        app: "tauri".into(),
        namespace: namespace.into(),
        mode: SidecarMode::Dev,
        source: SOURCE_TOOL_STIM_DEV.into(),
    }
}

fn spawn_renderer_ready_with_stdio(
    namespace: &str,
    force: bool,
    detached_stdio: bool,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    let mut args = vec!["run", "-p", "stim-renderer", "--", "serve", "--dev"];
    if force {
        args.push("--force");
    }
    let stamp = renderer_stamp(namespace);
    let stamp_args = create_stamp_args(&stamp);
    let mut command = Command::new("cargo");
    command
        .args(args)
        .args(&stamp_args)
        .current_dir(stim_platform::paths::workspace_root())
        .stdin(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stdout(Stdio::piped())
        .stderr(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        });
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn renderer delivery: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "renderer delivery stdout was not piped".to_string())?;
    let ready = wait_for_ready_line(stdout, std::time::Duration::from_secs(120))
        .map_err(|error| format!("renderer delivery ready failed: {error}"))?;
    validate_ready(&stamp, "renderer-delivery", &ready)?;
    Ok((child, ready))
}

fn spawn_controller_ready_with_stdio(
    namespace: &str,
    detached_stdio: bool,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    let stamp = controller_stamp(namespace);
    let stamp_args = create_stamp_args(&stamp);
    let mut command = Command::new("cargo");
    command
        .args(["run", "-p", "stim-controller", "--", "serve"])
        .args(&stamp_args)
        .current_dir(stim_platform::paths::workspace_root())
        .stdin(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stdout(Stdio::piped())
        .stderr(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        });
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn controller: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "controller stdout was not piped".to_string())?;
    let ready = wait_for_ready_line(stdout, std::time::Duration::from_secs(120))
        .map_err(|error| format!("controller ready failed: {error}"))?;
    validate_ready(&stamp, "controller-runtime", &ready)?;
    Ok((child, ready))
}

fn validate_ready(
    stamp: &SidecarStamp,
    role: &str,
    ready: &SidecarReadyLine,
) -> Result<(), String> {
    if !ready.is_ready_line() || &ready.stamp != stamp || ready.role != role {
        return Err(format!("unexpected {role} ready line"));
    }
    Ok(())
}

fn run_tauri_foreground(namespace: &str, envs: &[(&str, &str)]) -> Result<(), String> {
    let mut child = spawn_tauri(namespace, envs)?;
    let status = child
        .wait()
        .map_err(|error| format!("failed waiting for Tauri host: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Tauri host exited with status {status}"))
    }
}

fn spawn_tauri_with_stdio(
    namespace: &str,
    envs: &[(&str, &str)],
    detached_stdio: bool,
) -> Result<std::process::Child, String> {
    let mut command = Command::new("cargo");
    let stamp_args = create_stamp_args(&tauri_stamp(namespace));
    command
        .args(["run", "--no-default-features", "--"])
        .args(&stamp_args)
        .current_dir(tauri_app_dir().join("src-tauri"))
        .env(SIDECAR_NAMESPACE_ENV, namespace)
        .env(SIDECAR_MODE_ENV, SidecarMode::Dev.as_str())
        .stdin(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stdout(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stderr(if detached_stdio {
            Stdio::null()
        } else {
            Stdio::inherit()
        });
    for (key, value) in envs {
        command.env(key, value);
    }
    command
        .spawn()
        .map_err(|error| format!("failed to spawn Tauri host: {error}"))
}

fn run_cargo_owned(workdir: &std::path::Path, args: &[String]) -> Result<(), String> {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(workdir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("failed to run cargo {:?}: {error}", args))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {:?} exited with status {status}", args))
    }
}
