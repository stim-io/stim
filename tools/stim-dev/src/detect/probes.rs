use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::control::stamped_processes_for_namespace;

#[derive(Serialize)]
pub(super) struct RootWorkspaceProbe {
    pub(super) state: &'static str,
    pub(super) path: Option<String>,
    pub(super) compose_file: PathProbe,
}

impl RootWorkspaceProbe {
    pub(super) fn collect(stim_root: &Path) -> Self {
        let Some(root) = attached_root_candidate(stim_root) else {
            return Self {
                state: "not-attached",
                path: None,
                compose_file: PathProbe::missing(
                    "root docker-compose.yml",
                    None,
                    "stim workspace is not attached under a root modules/ directory",
                ),
            };
        };

        let compose_file =
            PathProbe::file("root docker-compose.yml", root.join("docker-compose.yml"));
        let state = if compose_file.state == "present" {
            "attached"
        } else {
            "compose-file-missing"
        };

        Self {
            state,
            path: Some(root.to_string_lossy().to_string()),
            compose_file,
        }
    }
}

#[derive(Serialize)]
pub(super) struct FileProbes {
    pub(super) santi_link_auth: PathProbe,
}

impl FileProbes {
    pub(super) fn collect(root_workspace: Option<&str>) -> Self {
        let santi_link_auth = root_workspace
            .map(|root| PathBuf::from(root).join("modules/santi-link/auth.json"))
            .map(|path| PathProbe::file("santi-link auth.json", path))
            .unwrap_or_else(|| {
                PathProbe::missing(
                    "santi-link auth.json",
                    None,
                    "root workspace was not detected",
                )
            });

        Self { santi_link_auth }
    }
}

#[derive(Serialize)]
pub(super) struct PathProbe {
    pub(super) label: &'static str,
    pub(super) path: Option<String>,
    pub(super) state: &'static str,
    pub(super) detail: String,
}

impl PathProbe {
    fn file(label: &'static str, path: PathBuf) -> Self {
        if path.is_file() {
            Self {
                label,
                path: Some(path.to_string_lossy().to_string()),
                state: "present",
                detail: "file exists".into(),
            }
        } else {
            Self::missing(label, Some(path), "file is missing")
        }
    }

    pub(super) fn missing(label: &'static str, path: Option<PathBuf>, detail: &str) -> Self {
        Self {
            label,
            path: path.map(|path| path.to_string_lossy().to_string()),
            state: "missing",
            detail: detail.into(),
        }
    }

    fn directory(label: &'static str, path: PathBuf) -> Self {
        if path.is_dir() {
            Self {
                label,
                path: Some(path.to_string_lossy().to_string()),
                state: "present",
                detail: "directory exists".into(),
            }
        } else {
            Self::missing(label, Some(path), "directory is missing")
        }
    }
}

#[derive(Serialize)]
pub(super) struct AppLoopProbe {
    pub(super) state: &'static str,
    pub(super) detail: String,
    pub(super) stamped_process_count: usize,
    pub(super) stamped_processes: Vec<ProcessProbe>,
    pub(super) residue: AppLoopResidueProbe,
}

impl AppLoopProbe {
    pub(super) fn collect(namespace: &str) -> Self {
        let residue = AppLoopResidueProbe::collect(namespace);
        let processes = match stamped_processes_for_namespace(namespace) {
            Ok(processes) => processes,
            Err(error) => {
                return Self {
                    state: "unknown",
                    detail: format!("failed to inspect stamped processes: {error}"),
                    stamped_process_count: 0,
                    stamped_processes: Vec::new(),
                    residue,
                };
            }
        };
        let stamped_processes = processes
            .into_iter()
            .map(|process| ProcessProbe {
                pid: process.pid,
                ppid: process.ppid,
                command: process.command,
            })
            .collect::<Vec<_>>();
        let stamped_process_count = stamped_processes.len();
        let state = if stamped_process_count > 0 {
            "running"
        } else if residue.is_present() {
            "stopped-with-residue"
        } else {
            "stopped-clean"
        };
        let detail = match state {
            "running" => format!("{stamped_process_count} stamped app-loop processes found"),
            "stopped-with-residue" => {
                "no stamped app-loop processes found, but bridge/log/lock residue exists".into()
            }
            _ => "no stamped app-loop processes or residue found".into(),
        };

        Self {
            state,
            detail,
            stamped_process_count,
            stamped_processes,
            residue,
        }
    }

    pub(super) fn is_running(&self) -> bool {
        self.state == "running"
    }

    pub(super) fn has_residue_without_processes(&self) -> bool {
        self.state == "stopped-with-residue"
    }
}

#[derive(Serialize)]
pub(super) struct ProcessProbe {
    pid: u32,
    ppid: u32,
    command: String,
}

#[derive(Serialize)]
pub(super) struct AppLoopResidueProbe {
    pub(super) bridges: PathProbe,
    pub(super) logs: PathProbe,
    pub(super) locks: PathProbe,
}

impl AppLoopResidueProbe {
    fn collect(namespace: &str) -> Self {
        let layout = stim_sidecar::layout::SidecarLayout::new(
            stim_platform::paths::dev_root(),
            Some(namespace),
        );

        Self {
            bridges: PathProbe::directory("app-loop bridges", layout.bridges_root),
            logs: PathProbe::directory("app-loop logs", layout.logs_root),
            locks: PathProbe::directory("app-loop locks", layout.locks_root),
        }
    }

    fn is_present(&self) -> bool {
        self.bridges.state == "present"
            || self.logs.state == "present"
            || self.locks.state == "present"
    }
}

pub(super) fn attached_root_candidate(stim_root: &Path) -> Option<PathBuf> {
    let modules_dir = stim_root.parent()?;
    if modules_dir.file_name()? != "modules" {
        return None;
    }

    modules_dir.parent().map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::attached_root_candidate;

    #[test]
    fn detects_attached_root_from_modules_stim_path() {
        let root = attached_root_candidate(Path::new("/workspace/modules/stim")).unwrap();

        assert_eq!(root, Path::new("/workspace"));
        assert!(attached_root_candidate(Path::new("/workspace/stim")).is_none());
    }
}
