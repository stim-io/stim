use std::sync::{Arc, Mutex};

use stim_proto::DiscoveryRecord;
use stim_shared::control_plane::{ControllerRuntimeHeartbeat, ControllerRuntimeSnapshot};

#[derive(Debug, Clone)]
pub struct ControllerServiceHandle {
    pub(crate) snapshot: Arc<Mutex<ControllerRuntimeSnapshot>>,
    pub(crate) heartbeat: Arc<Mutex<ControllerRuntimeHeartbeat>>,
}

impl ControllerServiceHandle {
    pub fn snapshot(&self) -> ControllerRuntimeSnapshot {
        self.snapshot.lock().expect("snapshot poisoned").clone()
    }

    pub fn heartbeat(&self) -> ControllerRuntimeHeartbeat {
        self.heartbeat.lock().expect("heartbeat poisoned").clone()
    }
}

#[derive(Debug, Clone)]
pub struct ControllerHttpState {
    pub(crate) snapshot: Arc<Mutex<ControllerRuntimeSnapshot>>,
    pub(crate) stim_server_base_url: String,
    pub(crate) santi_base_url: String,
    pub(crate) registered_endpoint_ids: Arc<Mutex<Vec<String>>>,
    pub(crate) self_discovery: DiscoveryRecord,
}
