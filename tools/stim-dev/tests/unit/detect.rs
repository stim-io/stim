use std::path::Path;

use crate::detect::{
    hints::{operation_hints, DetectSummary},
    http::parse_http_base_url,
    probes::{
        attached_root_candidate, AppLoopProbe, AppLoopResidueProbe, FileProbes, PathProbe,
        RootWorkspaceProbe,
    },
    services::ServiceProbe,
};

#[test]
fn parses_plain_http_urls() {
    let endpoint = parse_http_base_url("http://127.0.0.1:18081").unwrap();

    assert_eq!(endpoint.host, "127.0.0.1");
    assert_eq!(endpoint.port, 18081);
    assert_eq!(endpoint.host_header, "127.0.0.1:18081");

    let endpoint = parse_http_base_url("http://localhost").unwrap();
    assert_eq!(endpoint.port, 80);
}

#[test]
fn detects_attached_root() {
    let root = attached_root_candidate(Path::new("/workspace/modules/stim")).unwrap();

    assert_eq!(root, Path::new("/workspace"));
    assert!(attached_root_candidate(Path::new("/workspace/stim")).is_none());
}

#[test]
fn hints_explain_prereqs() {
    let root = root_workspace();
    let files = present_files();
    let services = vec![
        service("stim-server", "compose-default", "unavailable"),
        service("santi", "local-santi-default", "unavailable"),
    ];
    let summary = DetectSummary {
        state: "needs-action",
        ready: false,
        needs_action: Vec::new(),
    };
    let app_loop = stopped_clean();

    let hints = operation_hints(&root, &files, &services, &summary, &app_loop).join("\n");

    assert!(hints.contains("docker compose up -d --build stim-server santi-link"));
    assert!(hints.contains("scripts/santi local"));
    assert!(hints.contains("read-only"));
}

#[test]
fn ready_summary_points_child() {
    let root = root_workspace();
    let files = present_files();
    let services = vec![service("santi", "local-santi-default", "ready")];
    let summary = DetectSummary {
        state: "ready",
        ready: true,
        needs_action: Vec::new(),
    };
    let app_loop = stopped_clean();

    let hints = operation_hints(&root, &files, &services, &summary, &app_loop).join("\n");

    assert!(hints.contains("stim-dev restart"));
    assert!(!hints.contains("docker compose up"));
}

#[test]
fn residue_suggests_reset() {
    let root = root_workspace();
    let files = present_files();
    let services = vec![service("santi", "local-santi-default", "ready")];
    let summary = DetectSummary {
        state: "ready",
        ready: true,
        needs_action: Vec::new(),
    };
    let app_loop = AppLoopProbe {
        state: "stopped-with-residue",
        detail: "residue exists".into(),
        stamped_process_count: 0,
        stamped_processes: Vec::new(),
        residue: AppLoopResidueProbe {
            bridges: present_dir("app-loop bridges"),
            logs: missing_dir("app-loop logs"),
            locks: missing_dir("app-loop locks"),
        },
    };

    let hints = operation_hints(&root, &files, &services, &summary, &app_loop).join("\n");

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

fn service(name: &'static str, source: &'static str, state: &'static str) -> ServiceProbe {
    ServiceProbe {
        name,
        source,
        env_var: Some("SANTI_BASE_URL"),
        base_url: "http://127.0.0.1:18081".into(),
        health_path: "/api/v1/health",
        state,
        detail: "test".into(),
    }
}

fn stopped_clean() -> AppLoopProbe {
    AppLoopProbe {
        state: "stopped-clean",
        detail: "no residue".into(),
        stamped_process_count: 0,
        stamped_processes: Vec::new(),
        residue: AppLoopResidueProbe {
            bridges: missing_dir("app-loop bridges"),
            logs: missing_dir("app-loop logs"),
            locks: missing_dir("app-loop locks"),
        },
    }
}

fn present_dir(label: &'static str) -> PathProbe {
    PathProbe {
        label,
        path: Some("/tmp/stim".into()),
        state: "present",
        detail: "directory exists".into(),
    }
}

fn missing_dir(label: &'static str) -> PathProbe {
    PathProbe::missing(label, None, "directory is missing")
}
