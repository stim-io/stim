use serde::Serialize;

use super::{
    probes::{AppLoopProbe, FileProbes, RootWorkspaceProbe},
    services::{ServiceProbe, LOCAL_SANTI_HINT, STANDALONE_COMPOSE_HINT},
};

#[derive(Serialize)]
pub(crate) struct DetectSummary {
    pub(crate) state: &'static str,
    pub(crate) ready: bool,
    pub(crate) needs_action: Vec<String>,
}

pub(crate) fn summarize(files: &FileProbes, services: &[ServiceProbe]) -> DetectSummary {
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

pub(crate) fn operation_hints(
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
