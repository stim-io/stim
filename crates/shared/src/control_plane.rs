//! Re-exports of the canonical control-plane types from
//! `stim-proto`. This module exists for backward-compat with
//! existing `stim_shared::control_plane::*` import paths inside
//! the stim repo. New code should import from `stim_proto`
//! directly; this shim can disappear once all internal call
//! sites migrate.

pub use stim_proto::{
    agents_runtime_heartbeat_topic, agents_runtime_snapshot_topic,
    controller_runtime_heartbeat_topic, controller_runtime_snapshot_topic, namespace_or_default,
    namespaced_control_topic, AgentsRuntimeHeartbeat, AgentsRuntimeSnapshot, AgentsRuntimeState,
    ControllerRuntimeHeartbeat, ControllerRuntimeSnapshot, ControllerRuntimeState,
    RendererDeliveryLaunchBridge, DEFAULT_IPC_NAMESPACE, DEFAULT_SIDECAR_NAMESPACE,
    IPC_NAMESPACE_ENV, LEGACY_IPC_NAMESPACE_ENV, SIDECAR_NAMESPACE_ENV,
};
