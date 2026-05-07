use std::time::Duration;

use stim_shared::inspection::{RendererActionRequest, RendererProbeRequest};

use crate::shared::bridge::{renderer_action_timeout, renderer_probe_timeout};

#[test]
fn inspect_probes_are_short() {
    assert_eq!(
        renderer_probe_timeout(&RendererProbeRequest::LandingBasics),
        Duration::from_secs(10)
    );
    assert_eq!(
        renderer_probe_timeout(&RendererProbeRequest::MessagingState),
        Duration::from_secs(10)
    );
}

#[test]
fn actions_have_timeout() {
    assert_eq!(
        renderer_action_timeout(&RendererActionRequest::MessagingSend {
            text: "hello".into(),
            target_endpoint_id: None,
        }),
        Duration::from_secs(140)
    );
}
