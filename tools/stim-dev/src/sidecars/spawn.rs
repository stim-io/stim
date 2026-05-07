use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use stim_shared::paths::tauri_app_dir;
use stim_sidecar::{
    identity::{
        SidecarMode, SidecarStamp, SIDECAR_MODE_ENV, SIDECAR_NAMESPACE_ENV, SOURCE_TOOL_STIM_DEV,
    },
    ready::{wait_for_ready_line, SidecarReadyLine},
    stamp::create_stamp_args,
};

use crate::control::current_namespace;

use super::{
    bridge_file::write_renderer_delivery_bridge,
    stamp::{agents_stamp, controller_stamp, renderer_stamp, tauri_stamp},
};

const READY_TIMEOUT: Duration = Duration::from_secs(120);

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

pub(crate) fn start_agents() -> Result<(), String> {
    let namespace = current_namespace();
    let stamp = agents_stamp(&namespace);
    let mut args = vec![
        "run".to_string(),
        "-p".into(),
        "stim-agents".into(),
        "--".into(),
        "serve".into(),
    ];

    args.extend(create_stamp_args(&stamp));

    run_cargo_owned(&stim_platform::paths::workspace_root(), &args)
}

pub(crate) fn spawn_agents_ready(
    namespace: &str,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_agents_stdio(namespace, false)
}

pub(crate) fn spawn_agents_ready_detached(
    namespace: &str,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_agents_stdio(namespace, true)
}

pub(crate) fn spawn_renderer_ready(
    namespace: &str,
    force: bool,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_renderer_stdio(namespace, force, false)
}

pub(crate) fn spawn_renderer_ready_detached(
    namespace: &str,
    force: bool,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_renderer_stdio(namespace, force, true)
}

pub(crate) fn spawn_controller_ready(
    namespace: &str,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_controller_stdio(namespace, false)
}

pub(crate) fn spawn_controller_ready_detached(
    namespace: &str,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    spawn_controller_stdio(namespace, true)
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

fn spawn_renderer_stdio(
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
    let log_path = if detached_stdio {
        detach_process_group(&mut command);
        redirect_detached_output(&mut command, namespace, &stamp.app)?
    } else {
        None
    };
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn renderer delivery: {error}"))?;
    let stdout = child.stdout.take();
    let ready = wait_for_child_ready(&mut child, stdout, log_path.as_deref(), "renderer delivery")?;
    validate_ready(&stamp, "renderer-delivery", &ready)?;
    Ok((child, ready))
}

fn spawn_controller_stdio(
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
    let log_path = if detached_stdio {
        detach_process_group(&mut command);
        redirect_detached_output(&mut command, namespace, &stamp.app)?
    } else {
        None
    };
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn controller: {error}"))?;
    let stdout = child.stdout.take();
    let ready = wait_for_child_ready(&mut child, stdout, log_path.as_deref(), "controller")?;
    validate_ready(&stamp, "controller-runtime", &ready)?;
    Ok((child, ready))
}

fn spawn_agents_stdio(
    namespace: &str,
    detached_stdio: bool,
) -> Result<(std::process::Child, SidecarReadyLine), String> {
    let stamp = agents_stamp(namespace);
    let stamp_args = create_stamp_args(&stamp);
    let mut command = Command::new("cargo");
    command
        .args(["run", "-p", "stim-agents", "--", "serve"])
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
    let log_path = if detached_stdio {
        detach_process_group(&mut command);
        redirect_detached_output(&mut command, namespace, &stamp.app)?
    } else {
        None
    };
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn agents: {error}"))?;
    let stdout = child.stdout.take();
    let ready = wait_for_child_ready(&mut child, stdout, log_path.as_deref(), "agents")?;
    validate_ready(&stamp, "agents-runtime", &ready)?;
    Ok((child, ready))
}

fn redirect_detached_output(
    command: &mut Command,
    namespace: &str,
    app: &str,
) -> Result<Option<PathBuf>, String> {
    let layout =
        stim_sidecar::layout::SidecarLayout::new(stim_platform::paths::dev_root(), Some(namespace));
    let log_path = layout.app_log_path(app);
    let parent = log_path
        .parent()
        .ok_or_else(|| format!("log path has no parent: {}", log_path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .map_err(|error| format!("failed to open {}: {error}", log_path.display()))?;
    let stderr = log
        .try_clone()
        .map_err(|error| format!("failed to clone {}: {error}", log_path.display()))?;
    command.stdout(Stdio::from(log)).stderr(Stdio::from(stderr));
    Ok(Some(log_path))
}

fn wait_for_child_ready(
    child: &mut Child,
    stdout: Option<std::process::ChildStdout>,
    log_path: Option<&Path>,
    name: &str,
) -> Result<SidecarReadyLine, String> {
    if let Some(log_path) = log_path {
        return wait_for_ready_log(child, log_path, name);
    }

    let stdout = stdout.ok_or_else(|| format!("{name} stdout was not piped"))?;
    wait_for_ready_line(stdout, READY_TIMEOUT)
        .map_err(|error| format!("{name} ready failed: {error}"))
}

fn wait_for_ready_log(
    child: &mut Child,
    log_path: &Path,
    name: &str,
) -> Result<SidecarReadyLine, String> {
    let started = Instant::now();
    loop {
        if let Some(ready) = read_ready_line(log_path)? {
            return Ok(ready);
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed checking {name} status: {error}"))?
        {
            return Err(format!(
                "{name} exited before ready with status {status}; see {}",
                log_path.display()
            ));
        }
        if started.elapsed() >= READY_TIMEOUT {
            return Err(format!(
                "{name} ready failed: timed out waiting for ready line in {}",
                log_path.display()
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn read_ready_line(log_path: &Path) -> Result<Option<SidecarReadyLine>, String> {
    let file = match File::open(log_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("failed to open {}: {error}", log_path.display())),
    };

    for line in BufReader::new(file).lines() {
        let line =
            line.map_err(|error| format!("failed to read {}: {error}", log_path.display()))?;
        if let Ok(ready) = serde_json::from_str::<SidecarReadyLine>(line.trim()) {
            return Ok(Some(ready));
        }
    }

    Ok(None)
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
    if detached_stdio {
        detach_process_group(&mut command);
    }
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

fn detach_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    #[cfg(not(unix))]
    {
        let _ = command;
    }
}
