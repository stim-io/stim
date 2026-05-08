mod client;
mod config;
mod factory;
pub mod fetch;
mod handler;
mod model;
mod runtime;
mod service;

pub use model::{ControllerError, ControllerProofSummary, ControllerServiceHandle};

pub fn spawn_local_controller(namespace: Option<&str>) -> Result<ControllerServiceHandle, String> {
    runtime::spawn_local_controller(namespace)
}

pub fn install_sidecar_runtime(
    stamp: stim_sidecar::identity::SidecarStamp,
    handle: ControllerServiceHandle,
) -> Result<(), String> {
    runtime::install_sidecar_runtime(stamp, handle)
}

pub fn run_controller_proof() -> Result<ControllerProofSummary, ControllerError> {
    service::run()
}
