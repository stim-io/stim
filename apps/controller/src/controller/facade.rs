use std::collections::HashMap;

use stim_proto::DiscoveryRecord;

use super::types::ControllerError;

pub trait StimServerFacade {
    fn server_base_url(&self) -> &str;
    fn discover_endpoint(&self, endpoint_id: &str) -> Result<DiscoveryRecord, ControllerError>;
}

#[derive(Debug, Clone)]
pub struct InMemoryStimServerFacade {
    server_base_url: String,
    records_by_endpoint: HashMap<String, DiscoveryRecord>,
}

#[derive(Debug, Clone)]
pub struct HttpStimServerFacade {
    server_base_url: String,
    client: reqwest::blocking::Client,
}

impl HttpStimServerFacade {
    pub fn new(server_base_url: impl Into<String>) -> Self {
        Self {
            server_base_url: server_base_url.into(),
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl InMemoryStimServerFacade {
    pub fn new(server_base_url: impl Into<String>, records: Vec<DiscoveryRecord>) -> Self {
        let records_by_endpoint = records
            .into_iter()
            .map(|record| (record.endpoint_declaration.endpoint_id.clone(), record))
            .collect();

        Self {
            server_base_url: server_base_url.into(),
            records_by_endpoint,
        }
    }
}

pub fn in_memory_facade(
    server_base_url: impl Into<String>,
    records: Vec<DiscoveryRecord>,
) -> InMemoryStimServerFacade {
    InMemoryStimServerFacade::new(server_base_url, records)
}

impl StimServerFacade for InMemoryStimServerFacade {
    fn server_base_url(&self) -> &str {
        &self.server_base_url
    }

    fn discover_endpoint(&self, endpoint_id: &str) -> Result<DiscoveryRecord, ControllerError> {
        self.records_by_endpoint
            .get(endpoint_id)
            .cloned()
            .ok_or_else(|| ControllerError::UnknownEndpoint(endpoint_id.into()))
    }
}

impl StimServerFacade for HttpStimServerFacade {
    fn server_base_url(&self) -> &str {
        &self.server_base_url
    }

    fn discover_endpoint(&self, endpoint_id: &str) -> Result<DiscoveryRecord, ControllerError> {
        let response = self
            .client
            .get(format!(
                "{}/api/v1/discovery/endpoints/{}",
                self.server_base_url, endpoint_id
            ))
            .send()
            .map_err(|error| {
                ControllerError::Server(format!("discover request failed: {error}"))
            })?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ControllerError::UnknownEndpoint(endpoint_id.into()));
        }

        response
            .error_for_status()
            .map_err(|error| ControllerError::Server(format!("discover status failed: {error}")))?
            .json::<DiscoveryRecord>()
            .map_err(|error| ControllerError::Server(format!("discover decode failed: {error}")))
    }
}
