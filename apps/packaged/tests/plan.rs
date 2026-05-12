#[path = "../src/plan.rs"]
mod plan;

use stim_sidecar::identity::SidecarMode;

#[test]
fn plan_models_sidecars() {
    let plan = plan::packaged_sidecar_plan(Some("default"));

    assert_eq!(plan.sidecars.len(), 4);
    assert!(plan
        .sidecars
        .iter()
        .all(|sidecar| sidecar.stamp.mode == SidecarMode::Runtime));
    assert_eq!(
        plan.sidecars
            .iter()
            .map(|sidecar| sidecar.stamp.app.as_str())
            .collect::<Vec<_>>(),
        vec!["renderer", "agents", "controller", "tauri"]
    );
    assert_eq!(
        plan.sidecars
            .iter()
            .map(|sidecar| sidecar.stamp.source.as_str())
            .collect::<Vec<_>>(),
        vec![
            "app:packaged",
            "app:packaged",
            "app:packaged",
            "app:packaged"
        ]
    );
    assert!(plan.sidecars.iter().all(|sidecar| sidecar
        .stamp_args
        .iter()
        .any(|arg| arg == "--sidecar-stamp-mode=runtime")));
    assert!(plan.sidecars.iter().all(|sidecar| sidecar
        .stamp_args
        .iter()
        .all(|arg| !arg.starts_with("--sidecar-stamp-role")
            && !arg.starts_with("--sidecar-stamp-instance"))));
}
