mod carrier;
mod facade;
mod fixtures;
mod messages;
mod reply;
mod runtime;
mod types;

pub use carrier::HttpSantiCarrier;
pub use facade::{
    in_memory_facade, HttpStimServerFacade, InMemoryStimServerFacade, StimServerFacade,
};
pub use fixtures::{
    http_santi_discovery_fixture, sample_local_discovery_record, sample_santi_discovery_record,
    seed_discovery_records,
};
pub use runtime::{
    first_message_roundtrip, first_message_roundtrip_via_server,
    first_message_roundtrip_with_records, message_roundtrip_via_server,
    message_roundtrip_with_records, run, ControllerRuntime,
};
pub use types::{
    ControllerDiscoveryFixture, ControllerError, ControllerLifecycleProof, ControllerLifecycleStep,
    ControllerProofSummary, ControllerState,
};

#[cfg(test)]
mod tests;
