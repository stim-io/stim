use std::time::Duration;

use stim_shared::inspection::{RendererActionRequest, RendererProbeRequest};
use stim_tauri::inspection::{
    renderer_action::renderer_action_timeout, renderer_probe::renderer_probe_timeout,
};

#[test]
fn action_timeout_is_bounded() {
    assert_eq!(
        renderer_action_timeout(&RendererActionRequest::MessagingNewConversation),
        Duration::from_secs(10)
    );
    assert_eq!(
        renderer_action_timeout(&RendererActionRequest::MessagingSend {
            text: "hello".into(),
            target_endpoint_id: None,
        }),
        Duration::from_secs(130)
    );
}

#[test]
fn probes_use_short_timeout() {
    assert_eq!(
        renderer_probe_timeout(&RendererProbeRequest::LandingBasics),
        Duration::from_secs(10)
    );
    assert_eq!(
        renderer_probe_timeout(&RendererProbeRequest::MessagingState),
        Duration::from_secs(10)
    );
}
