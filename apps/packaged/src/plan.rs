use serde::Serialize;
use stim_sidecar::{
    identity::{namespace_or_default, SidecarMode, SidecarStamp, SOURCE_APP_PACKAGED},
    stamp::create_stamp_args,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PackagedSidecarPlan {
    pub(crate) sidecars: Vec<PackagedSidecarEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PackagedSidecarEntry {
    pub(crate) stamp: SidecarStamp,
    pub(crate) role: String,
    pub(crate) instance_id: String,
    pub(crate) stamp_args: Vec<String>,
}

pub(crate) fn packaged_sidecar_plan(namespace: Option<&str>) -> PackagedSidecarPlan {
    let namespace = namespace_or_default(namespace);
    let mode = SidecarMode::Runtime;

    PackagedSidecarPlan {
        sidecars: [
            (
                SidecarStamp {
                    app: "renderer".into(),
                    namespace: namespace.clone(),
                    mode,
                    source: SOURCE_APP_PACKAGED.into(),
                },
                "renderer-delivery",
                format!("{namespace}-renderer"),
            ),
            (
                SidecarStamp {
                    app: "controller".into(),
                    namespace: namespace.clone(),
                    mode,
                    source: SOURCE_APP_PACKAGED.into(),
                },
                "controller-runtime",
                format!("{namespace}-controller"),
            ),
            (
                SidecarStamp {
                    app: "tauri".into(),
                    namespace: namespace.clone(),
                    mode,
                    source: SOURCE_APP_PACKAGED.into(),
                },
                "tauri-host",
                format!("{namespace}-tauri"),
            ),
        ]
        .into_iter()
        .map(|(stamp, role, instance_id)| PackagedSidecarEntry {
            stamp_args: create_stamp_args(&stamp),
            stamp,
            role: role.into(),
            instance_id,
        })
        .collect(),
    }
}

#[cfg(test)]
mod tests {
    use stim_sidecar::identity::SidecarMode;

    use super::packaged_sidecar_plan;

    #[test]
    fn packaged_plan_models_runtime_sidecars() {
        let plan = packaged_sidecar_plan(Some("default"));

        assert_eq!(plan.sidecars.len(), 3);
        assert!(plan
            .sidecars
            .iter()
            .all(|sidecar| sidecar.stamp.mode == SidecarMode::Runtime));
        assert_eq!(
            plan.sidecars
                .iter()
                .map(|sidecar| sidecar.stamp.app.as_str())
                .collect::<Vec<_>>(),
            vec!["renderer", "controller", "tauri"]
        );
        assert_eq!(
            plan.sidecars
                .iter()
                .map(|sidecar| sidecar.stamp.source.as_str())
                .collect::<Vec<_>>(),
            vec!["app:packaged", "app:packaged", "app:packaged"]
        );
        assert!(plan.sidecars.iter().all(|sidecar| sidecar
            .stamp_args
            .iter()
            .any(|arg| arg == "--stim-stamp-mode=runtime")));
        assert!(plan.sidecars.iter().all(|sidecar| sidecar
            .stamp_args
            .iter()
            .all(|arg| !arg.starts_with("--stim-stamp-role")
                && !arg.starts_with("--stim-stamp-instance"))));
    }
}
