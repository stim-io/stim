use stim_shared::message_operation::{
    ControllerOperationReference, ControllerOperationReferenceKind,
};

use crate::client::resolve_delivery_endpoint;

pub(super) async fn resolve_target(
    stim_server_base_url: String,
    target_endpoint_id: String,
    participant_id: Option<String>,
) -> Result<ResolvedTarget, String> {
    tokio::task::spawn_blocking(move || match normalized_participant_id(participant_id.as_deref()) {
        Some(participant_id) => resolve_delivery_endpoint(&stim_server_base_url, participant_id)
            .map(|endpoint_id| ResolvedTarget {
                detail: format!("resolved participant {participant_id} to endpoint {endpoint_id}"),
                endpoint_id,
                participant_id: Some(participant_id.to_string()),
            })
            .map_err(|(_, error)| {
                format!("controller delivery target resolution failed: {error}")
            }),
        None => {
            let endpoint_id = target_endpoint_id.trim().to_string();
            if endpoint_id.is_empty() {
                Err("controller delivery target resolution failed: target endpoint id must not be empty".into())
            } else {
                Ok(ResolvedTarget {
                    detail: format!("using direct endpoint {endpoint_id}"),
                    endpoint_id,
                    participant_id: None,
                })
            }
        }
    })
    .await
    .map_err(|error| format!("controller delivery target resolution join failed: {error}"))?
}

pub(super) struct ResolvedTarget {
    pub(super) endpoint_id: String,
    pub(super) participant_id: Option<String>,
    pub(super) detail: String,
}

impl ResolvedTarget {
    pub(super) fn product_participant_id(&self) -> String {
        self.participant_id
            .clone()
            .unwrap_or_else(|| self.endpoint_id.clone())
    }

    pub(super) fn references(&self) -> Vec<ControllerOperationReference> {
        let mut references = self.endpoint_reference();
        if let Some(participant_id) = &self.participant_id {
            references.push(ControllerOperationReference {
                reference_kind: ControllerOperationReferenceKind::Participant,
                ledger_id: None,
                fact_id: None,
                message_id: None,
                content_id: None,
                revision_id: None,
                relation_id: None,
                participant_id: Some(participant_id.clone()),
                endpoint_id: Some(self.endpoint_id.clone()),
                envelope_id: None,
                reply_id: None,
                detail: Some("stim-server participant delivery-target projection".into()),
            });
        }
        references
    }

    pub(super) fn endpoint_reference(&self) -> Vec<ControllerOperationReference> {
        vec![ControllerOperationReference {
            reference_kind: ControllerOperationReferenceKind::DeliveryEndpoint,
            ledger_id: None,
            fact_id: None,
            message_id: None,
            content_id: None,
            revision_id: None,
            relation_id: None,
            participant_id: self.participant_id.clone(),
            endpoint_id: Some(self.endpoint_id.clone()),
            envelope_id: None,
            reply_id: None,
            detail: Some("resolved protocol delivery endpoint".into()),
        }]
    }
}

fn normalized_participant_id(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
