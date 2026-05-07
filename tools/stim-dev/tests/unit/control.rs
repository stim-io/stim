use crate::control::{
    agents::{agents, agents_url, percent_encode_path_segment as encode_agent_id},
    inspect::percent_encode_path_segment as encode_inspect_path,
    processes::{command_is_tauri_host, is_renderer_dev_server},
};

#[test]
fn agents_url_normalizes() {
    assert_eq!(
        agents_url("http://127.0.0.1:43210/", "/api/v1/agents/instances"),
        "http://127.0.0.1:43210/api/v1/agents/instances"
    );
}

#[test]
fn agents_rejects_bad_leaves() {
    assert!(agents(Vec::new())
        .unwrap_err()
        .contains("select <instance_id>"));
    assert!(agents(vec!["select".into()])
        .unwrap_err()
        .contains("select <instance_id>"));
    assert!(agents(vec!["start".into(), "local-santi".into()])
        .unwrap_err()
        .contains("unsupported agents leaf"));
}

#[test]
fn agents_id_is_encoded() {
    assert_eq!(encode_agent_id("local/santi"), "local%2Fsanti");
}

#[test]
fn percent_encode_escapes_path() {
    assert_eq!(encode_inspect_path("local santi/1"), "local%20santi%2F1");
}

#[test]
fn recognizes_renderer_vite() {
    let command = format!(
        "node {}/node_modules/.bin/../vite/bin/vite.js --host 127.0.0.1 --port 1420",
        stim_shared::paths::renderer_vite_dir().display()
    );

    assert!(is_renderer_dev_server(&command));
    assert!(!is_renderer_dev_server(
        "node /tmp/other/vite.js --host 127.0.0.1 --port 1420"
    ));
}

#[test]
fn recognizes_tauri_process() {
    let command = format!(
        "{} --stim-stamp-app=tauri --stim-stamp-namespace=default",
        stim_platform::paths::workspace_root()
            .join("target")
            .join("debug")
            .join("stim-tauri")
            .display()
    );

    assert!(command_is_tauri_host(&command));
    assert!(!command_is_tauri_host("/tmp/stim-tauri"));
}
