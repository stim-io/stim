use stim_shared::control_plane::{
    agents_runtime_heartbeat_topic, agents_runtime_snapshot_topic,
    controller_runtime_heartbeat_topic, controller_runtime_snapshot_topic, namespace_or_default,
    AgentsRuntimeHeartbeat, AgentsRuntimeSnapshot, AgentsRuntimeState, ControllerRuntimeHeartbeat,
    ControllerRuntimeSnapshot, ControllerRuntimeState,
};

#[test]
fn uses_default_namespace() {
    assert_eq!(namespace_or_default(None), "default");
    assert_eq!(namespace_or_default(Some("")), "default");
}

#[test]
fn builds_controller_topics() {
    assert_eq!(
        controller_runtime_snapshot_topic("dev-a"),
        "stim://control/dev-a/controller/runtime/snapshot"
    );
    assert_eq!(
        controller_runtime_heartbeat_topic("dev-a"),
        "stim://control/dev-a/controller/runtime/heartbeat"
    );
}

#[test]
fn builds_agents_topics() {
    assert_eq!(
        agents_runtime_snapshot_topic("dev-a"),
        "stim://control/dev-a/agents/runtime/snapshot"
    );
    assert_eq!(
        agents_runtime_heartbeat_topic("dev-a"),
        "stim://control/dev-a/agents/runtime/heartbeat"
    );
}

#[test]
fn preserves_http_target() {
    let snapshot = ControllerRuntimeSnapshot {
        namespace: "dev-a".into(),
        instance_id: "controller-1".into(),
        published_at: "2026-04-14T00:00:00Z".into(),
        state: ControllerRuntimeState::Ready,
        http_base_url: Some("http://127.0.0.1:43100".into()),
        detail: None,
    };
    let heartbeat = ControllerRuntimeHeartbeat {
        namespace: "dev-a".into(),
        instance_id: "controller-1".into(),
        published_at: "2026-04-14T00:00:05Z".into(),
        sequence: 5,
        state: ControllerRuntimeState::Ready,
    };

    assert_eq!(
        snapshot.http_base_url.as_deref(),
        Some("http://127.0.0.1:43100")
    );
    assert_eq!(heartbeat.state, ControllerRuntimeState::Ready);
}

#[test]
fn agents_preserve_http_target() {
    let snapshot = AgentsRuntimeSnapshot {
        namespace: "dev-a".into(),
        instance_id: "agents-1".into(),
        published_at: "2026-05-06T00:00:00Z".into(),
        state: AgentsRuntimeState::Ready,
        http_base_url: Some("http://127.0.0.1:43200".into()),
        detail: None,
    };
    let heartbeat = AgentsRuntimeHeartbeat {
        namespace: "dev-a".into(),
        instance_id: "agents-1".into(),
        published_at: "2026-05-06T00:00:05Z".into(),
        sequence: 5,
        state: AgentsRuntimeState::Ready,
    };

    assert_eq!(
        snapshot.http_base_url.as_deref(),
        Some("http://127.0.0.1:43200")
    );
    assert_eq!(heartbeat.state, AgentsRuntimeState::Ready);
}
