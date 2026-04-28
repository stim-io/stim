use serde::{Deserialize, Serialize};

pub const DEFAULT_SIDECAR_NAMESPACE: &str = "default";
pub const SIDECAR_NAMESPACE_ENV: &str = "STIM_SIDECAR_NAMESPACE";
pub const LEGACY_IPC_NAMESPACE_ENV: &str = "STIM_IPC_NAMESPACE";

pub const DEFAULT_IPC_NAMESPACE: &str = DEFAULT_SIDECAR_NAMESPACE;
pub const IPC_NAMESPACE_ENV: &str = LEGACY_IPC_NAMESPACE_ENV;

const CONTROL_PREFIX: &str = "stim://control";
const CONTROLLER_RUNTIME_SNAPSHOT_TOPIC: &str = "controller/runtime/snapshot";
const CONTROLLER_RUNTIME_HEARTBEAT_TOPIC: &str = "controller/runtime/heartbeat";

pub fn namespace_or_default(namespace: Option<&str>) -> &str {
    namespace
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_SIDECAR_NAMESPACE)
}

pub fn namespaced_control_topic(namespace: &str, topic: &str) -> String {
    format!(
        "{CONTROL_PREFIX}/{}/{}",
        namespace_or_default(Some(namespace)),
        topic
    )
}

pub fn controller_runtime_snapshot_topic(namespace: &str) -> String {
    namespaced_control_topic(namespace, CONTROLLER_RUNTIME_SNAPSHOT_TOPIC)
}

pub fn controller_runtime_heartbeat_topic(namespace: &str) -> String {
    namespaced_control_topic(namespace, CONTROLLER_RUNTIME_HEARTBEAT_TOPIC)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControllerRuntimeState {
    Starting,
    Ready,
    Degraded,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerRuntimeSnapshot {
    pub namespace: String,
    pub instance_id: String,
    pub published_at: String,
    pub state: ControllerRuntimeState,
    pub http_base_url: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerRuntimeHeartbeat {
    pub namespace: String,
    pub instance_id: String,
    pub published_at: String,
    pub sequence: u64,
    pub state: ControllerRuntimeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RendererDeliveryLaunchBridge {
    pub namespace: String,
    pub renderer_url: String,
    pub source: String,
    pub published_at: String,
}

#[cfg(test)]
mod tests {
    use super::{
        controller_runtime_heartbeat_topic, controller_runtime_snapshot_topic,
        namespace_or_default, ControllerRuntimeHeartbeat, ControllerRuntimeSnapshot,
        ControllerRuntimeState,
    };

    #[test]
    fn uses_default_namespace_when_missing() {
        assert_eq!(namespace_or_default(None), "default");
        assert_eq!(namespace_or_default(Some("")), "default");
    }

    #[test]
    fn builds_namespaced_controller_topics() {
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
    fn heartbeat_and_snapshot_preserve_http_as_attach_target() {
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
}
