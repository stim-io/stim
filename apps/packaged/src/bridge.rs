use stim_shared::{
    control_plane::RendererDeliveryLaunchBridge, paths::renderer_delivery_launch_bridge_path,
};
use stim_sidecar::identity::SidecarMode;

use crate::clock::timestamp_now;

pub(crate) fn write_renderer_delivery_bridge(
    namespace: &str,
    mode: SidecarMode,
    renderer_url: &str,
    source: &str,
) -> Result<(), String> {
    let bridge = RendererDeliveryLaunchBridge {
        namespace: namespace.into(),
        renderer_url: renderer_url.into(),
        source: source.into(),
        published_at: timestamp_now(),
    };
    let path = renderer_delivery_launch_bridge_path(mode.as_str(), namespace);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create renderer delivery bridge dir: {error}"))?;
    }
    let body = serde_json::to_string_pretty(&bridge)
        .map_err(|error| format!("failed to serialize renderer delivery bridge: {error}"))?;
    std::fs::write(&path, format!("{body}\n"))
        .map_err(|error| format!("failed to write renderer delivery bridge: {error}"))
}
