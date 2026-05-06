use stim_sidecar::identity::{SidecarMode, SidecarStamp, SOURCE_TOOL_STIM_DEV};

pub(super) fn renderer_stamp(namespace: &str) -> SidecarStamp {
    SidecarStamp {
        app: "renderer".into(),
        namespace: namespace.into(),
        mode: SidecarMode::Dev,
        source: SOURCE_TOOL_STIM_DEV.into(),
    }
}

pub(super) fn controller_stamp(namespace: &str) -> SidecarStamp {
    SidecarStamp {
        app: "controller".into(),
        namespace: namespace.into(),
        mode: SidecarMode::Dev,
        source: SOURCE_TOOL_STIM_DEV.into(),
    }
}

pub(super) fn tauri_stamp(namespace: &str) -> SidecarStamp {
    SidecarStamp {
        app: "tauri".into(),
        namespace: namespace.into(),
        mode: SidecarMode::Dev,
        source: SOURCE_TOOL_STIM_DEV.into(),
    }
}
