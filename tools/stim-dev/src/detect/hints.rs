use serde::Serialize;

use super::{
    probes::{AppLoopProbe, FileProbes, RootWorkspaceProbe},
    services::{ServiceProbe, LOCAL_SANTI_HINT, STANDALONE_COMPOSE_HINT},
};

#[derive(Serialize)]
pub(super) struct DetectSummary {
    pub(super) state: &'static str,
    pub(super) ready: bool,
    pub(super) needs_action: Vec<String>,
}

pub(super) fn summarize(files: &FileProbes, services: &[ServiceProbe]) -> DetectSummary {
    let mut needs_action = Vec::new();

    if files.santi_link_auth.state != "present" {
        needs_action.push("santi-link auth.json missing".into());
    }

    needs_action.extend(
        services
            .iter()
            .filter(|service| !service.is_ready())
            .map(|service| format!("{} {}", service.name, service.state)),
    );

    let ready = needs_action.is_empty();
    let state = if ready { "ready" } else { "needs-action" };

    DetectSummary {
        state,
        ready,
        needs_action,
    }
}

pub(super) fn operation_hints(
    root_workspace: &RootWorkspaceProbe,
    files: &FileProbes,
    services: &[ServiceProbe],
    summary: &DetectSummary,
    app_loop: &AppLoopProbe,
) -> Vec<String> {
    let mut hints = vec![
        "stim-dev detect is read-only; it reports prerequisites and suggested next commands but does not start or stop Docker or sidecars.".into(),
    ];

    if services
        .iter()
        .any(|service| !service.is_ready() && service.uses_compose_default())
    {
        hints.push(match root_workspace.path.as_deref() {
            Some(root) if root_workspace.compose_file.state == "present" => format!(
                "Root compose prerequisites are not ready. From {root}, run: {STANDALONE_COMPOSE_HINT}"
            ),
            _ => "Root compose prerequisites are not ready and no attached root docker-compose.yml was detected. Attach under the root workspace or set STIM_SERVER_BASE_URL and SANTI_BASE_URL to reachable services.".into(),
        });
    }

    if services
        .iter()
        .any(|service| !service.is_ready() && service.uses_local_santi_default())
    {
        hints.push(match root_workspace.path.as_deref() {
            Some(root) => format!(
                "Default local Santi is not ready. From {root}, run: {LOCAL_SANTI_HINT}"
            ),
            None => "Default local Santi is not ready. Attach under the root workspace and run the root-owned local Santi foreground command, or set SANTI_BASE_URL to a reachable service.".into(),
        });
    }

    if files.santi_link_auth.state != "present" {
        hints.push("Provider-backed standalone smoke needs modules/santi-link/auth.json before santi-link can serve real upstream requests.".into());
    }

    if services
        .iter()
        .any(|service| !service.is_ready() && service.source == "env-override")
    {
        hints.push("One or more environment override endpoints are unreachable; fix the env var target or unset it to use the default local endpoint for that service.".into());
    }

    if app_loop.has_residue_without_processes() {
        hints.push("App-loop bridge/log/lock residue exists without stamped app-loop processes. If a previous restart timed out or you want a clean smoke run, run: stim-dev reset".into());
    }

    if summary.ready {
        if app_loop.is_running() {
            hints.push("Standalone prerequisites are reachable and stamped app-loop processes exist. Next inspection step: stim-dev status".into());
        } else if app_loop.has_residue_without_processes() {
            hints.push("Standalone prerequisites are reachable. Clean app-loop step: stim-dev reset, then stim-dev restart".into());
        } else {
            hints.push(
                "Standalone prerequisites are reachable. Next app-loop step: stim-dev restart"
                    .into(),
            );
        }
    } else {
        hints.push("After applying the suggested prerequisite fix, rerun: stim-dev detect".into());
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            probes::{
                AppLoopProbe, AppLoopResidueProbe, FileProbes, PathProbe, RootWorkspaceProbe,
            },
            services::ServiceProbe,
        },
        operation_hints, DetectSummary,
    };

    #[test]
    fn hints_explain_root_prerequisites_without_owning_lifecycle() {
        let root_workspace = root_workspace();
        let files = present_files();
        let services = vec![
            ServiceProbe {
                name: "stim-server",
                source: "compose-default",
                env_var: Some("STIM_SERVER_BASE_URL"),
                base_url: "http://127.0.0.1:18083".into(),
                health_path: "/api/v1/health",
                state: "unavailable",
                detail: "connection refused".into(),
            },
            ServiceProbe {
                name: "santi",
                source: "local-santi-default",
                env_var: Some("SANTI_BASE_URL"),
                base_url: "http://127.0.0.1:18081".into(),
                health_path: "/api/v1/health",
                state: "unavailable",
                detail: "connection refused".into(),
            },
        ];
        let summary = super::summarize(&files, &services);
        let app_loop = AppLoopProbe {
            state: "stopped-clean",
            detail: "no stamped app-loop processes or residue found".into(),
            stamped_process_count: 0,
            stamped_processes: Vec::new(),
            residue: missing_residue(),
        };

        let hints =
            operation_hints(&root_workspace, &files, &services, &summary, &app_loop).join("\n");

        assert!(hints.contains("docker compose up -d --build stim-server santi-link"));
        assert!(hints.contains("scripts/santi local"));
        assert!(hints.contains("read-only"));
        assert!(!hints.contains("start-standalone"));
    }

    #[test]
    fn ready_summary_points_to_child_app_loop() {
        let root_workspace = root_workspace();
        let files = present_files();
        let services = vec![ServiceProbe {
            name: "santi",
            source: "local-santi-default",
            env_var: Some("SANTI_BASE_URL"),
            base_url: "http://127.0.0.1:18081".into(),
            health_path: "/api/v1/health",
            state: "ready",
            detail: "health returned HTTP 200".into(),
        }];
        let summary = DetectSummary {
            state: "ready",
            ready: true,
            needs_action: Vec::new(),
        };
        let app_loop = AppLoopProbe {
            state: "stopped-clean",
            detail: "no stamped app-loop processes or residue found".into(),
            stamped_process_count: 0,
            stamped_processes: Vec::new(),
            residue: missing_residue(),
        };

        let hints =
            operation_hints(&root_workspace, &files, &services, &summary, &app_loop).join("\n");

        assert!(hints.contains("stim-dev restart"));
        assert!(!hints.contains("docker compose up"));
    }

    #[test]
    fn residue_without_processes_suggests_reset_before_clean_restart() {
        let root_workspace = root_workspace();
        let files = present_files();
        let services = vec![ServiceProbe {
            name: "santi",
            source: "local-santi-default",
            env_var: Some("SANTI_BASE_URL"),
            base_url: "http://127.0.0.1:18081".into(),
            health_path: "/api/v1/health",
            state: "ready",
            detail: "health returned HTTP 200".into(),
        }];
        let summary = DetectSummary {
            state: "ready",
            ready: true,
            needs_action: Vec::new(),
        };
        let app_loop = AppLoopProbe {
            state: "stopped-with-residue",
            detail: "no stamped app-loop processes found, but bridge/log/lock residue exists"
                .into(),
            stamped_process_count: 0,
            stamped_processes: Vec::new(),
            residue: AppLoopResidueProbe {
                bridges: present_dir("app-loop bridges"),
                logs: missing_dir("app-loop logs"),
                locks: missing_dir("app-loop locks"),
            },
        };

        let hints =
            operation_hints(&root_workspace, &files, &services, &summary, &app_loop).join("\n");

        assert!(hints.contains("stim-dev reset"));
        assert!(hints.contains("then stim-dev restart"));
    }

    fn root_workspace() -> RootWorkspaceProbe {
        RootWorkspaceProbe {
            state: "attached",
            path: Some("/workspace".into()),
            compose_file: PathProbe {
                label: "root docker-compose.yml",
                path: Some("/workspace/docker-compose.yml".into()),
                state: "present",
                detail: "file exists".into(),
            },
        }
    }

    fn present_files() -> FileProbes {
        FileProbes {
            santi_link_auth: PathProbe {
                label: "santi-link auth.json",
                path: Some("/workspace/modules/santi-link/auth.json".into()),
                state: "present",
                detail: "file exists".into(),
            },
        }
    }

    fn missing_residue() -> AppLoopResidueProbe {
        AppLoopResidueProbe {
            bridges: missing_dir("app-loop bridges"),
            logs: missing_dir("app-loop logs"),
            locks: missing_dir("app-loop locks"),
        }
    }

    fn present_dir(label: &'static str) -> PathProbe {
        PathProbe {
            label,
            path: Some(format!("/workspace/{label}")),
            state: "present",
            detail: "directory exists".into(),
        }
    }

    fn missing_dir(label: &'static str) -> PathProbe {
        PathProbe {
            label,
            path: Some(format!("/workspace/{label}")),
            state: "missing",
            detail: "directory is missing".into(),
        }
    }
}
