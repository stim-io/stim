use serde::{Deserialize, Serialize};

use crate::identity::SidecarStamp;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LiveInspectState {
    Ready,
    Degraded,
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveInspectEnvelope<T> {
    pub inspected_at: String,
    pub stamp: SidecarStamp,
    pub role: Option<String>,
    pub instance_id: Option<String>,
    pub state: LiveInspectState,
    pub detail: Option<String>,
    pub payload: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmptyInspectPayload {}

#[cfg(test)]
mod tests {
    use crate::{
        identity::{SidecarMode, SidecarStamp, SOURCE_TOOL_STIM_DEV},
        inspect::{EmptyInspectPayload, LiveInspectEnvelope, LiveInspectState},
    };

    #[test]
    fn live_inspect_envelope_is_current_fact_not_persisted_state() {
        let envelope = LiveInspectEnvelope {
            inspected_at: "2026-04-27T00:00:00Z".into(),
            stamp: SidecarStamp {
                app: "controller".into(),
                namespace: "default".into(),
                mode: SidecarMode::Dev,
                source: SOURCE_TOOL_STIM_DEV.into(),
            },
            role: Some("controller-runtime".into()),
            instance_id: Some("controller-1".into()),
            state: LiveInspectState::Ready,
            detail: None,
            payload: EmptyInspectPayload {},
        };

        assert_eq!(envelope.state, LiveInspectState::Ready);
        assert_eq!(envelope.stamp.mode, SidecarMode::Dev);
    }
}
