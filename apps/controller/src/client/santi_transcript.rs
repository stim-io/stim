use crate::fetch::{
    FetchClient, FetchError, FetchOutcome, FetchRequestOptions, FetchRetry, FetchRetryDecision,
    FetchRetryPolicy,
};

use super::santi_model::{SantiSessionMessagesResponse, SantiSessionToolActivitiesResponse};

pub(crate) fn fetch_santi_conversation_messages(
    santi_base_url: &str,
    conversation_id: &str,
) -> Result<FetchOutcome<SantiSessionMessagesResponse>, FetchError> {
    FetchClient::new(santi_base_url).get_json(
        format!("/api/v1/sessions/{conversation_id}/messages"),
        FetchRequestOptions::default().with_retry(santi_transcript_retry()),
    )
}

pub(crate) fn fetch_santi_tool_activity(
    santi_base_url: &str,
    conversation_id: &str,
) -> Result<FetchOutcome<SantiSessionToolActivitiesResponse>, FetchError> {
    FetchClient::new(santi_base_url).get_json(
        format!("/api/v1/sessions/{conversation_id}/tool-activities"),
        FetchRequestOptions::default()
            .with_retry(FetchRetry::santi_transient())
            .with_not_found_payload(SantiSessionToolActivitiesResponse::empty),
    )
}

fn santi_transcript_retry() -> FetchRetry {
    FetchRetry::custom(FetchRetryPolicy::santi_transient(), |context| {
        if context.status == Some(404) && context.path.ends_with("/messages") {
            return FetchRetryDecision::Retry;
        }

        if context
            .status
            .and_then(|status| reqwest::StatusCode::from_u16(status).ok())
            .is_some_and(|status| {
                status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS
            })
        {
            return FetchRetryDecision::Retry;
        }

        if context.error.is_some() {
            return FetchRetryDecision::Retry;
        }

        FetchRetryDecision::Fail
    })
}
