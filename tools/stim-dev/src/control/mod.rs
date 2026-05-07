pub(crate) mod agents;
pub(crate) mod inspect;
mod lifecycle;
mod namespace;
mod output;
pub(crate) mod processes;

pub(crate) use agents::{agents, get_agents_json, post_agents_empty};
pub(crate) use inspect::{inspect, require_renderer_landing};
pub(crate) use lifecycle::{list, reset, status, stop};
pub(crate) use namespace::current_namespace;
pub(crate) use processes::{
    stamped_processes_for_namespace, stop_matching_processes, stop_renderer_processes,
    stop_tauri_host_processes,
};
