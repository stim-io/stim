use serde::Serialize;

use crate::{control::current_namespace, shared::clock::timestamp_now};

mod hints;
mod http;
mod probes;
mod services;

use hints::{operation_hints, summarize, DetectSummary};
use probes::{AppLoopProbe, FileProbes, RootWorkspaceProbe};
use services::{default_service_probes, ServiceProbe};

pub(crate) fn detect() -> Result<(), String> {
    let report = DetectReport::collect();
    let output = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to serialize detect report: {error}"))?;

    println!("{output}");
    Ok(())
}

#[derive(Serialize)]
struct DetectReport {
    command: &'static str,
    mode: &'static str,
    namespace: String,
    checked_at: String,
    root_workspace: RootWorkspaceProbe,
    files: FileProbes,
    app_loop: AppLoopProbe,
    services: Vec<ServiceProbe>,
    summary: DetectSummary,
    hints: Vec<String>,
}

impl DetectReport {
    fn collect() -> Self {
        let stim_root = stim_platform::paths::workspace_root();
        let root_workspace = RootWorkspaceProbe::collect(&stim_root);
        let files = FileProbes::collect(root_workspace.path.as_deref());
        let namespace = current_namespace();
        let app_loop = AppLoopProbe::collect(&namespace);
        let services = default_service_probes();
        let summary = summarize(&files, &services);
        let hints = operation_hints(&root_workspace, &files, &services, &summary, &app_loop);

        Self {
            command: "stim-dev detect",
            mode: "standalone-prerequisites",
            namespace,
            checked_at: timestamp_now(),
            root_workspace,
            files,
            app_loop,
            services,
            summary,
            hints,
        }
    }
}
