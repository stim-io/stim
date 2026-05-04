use super::types::SantiSessionMessagesResponse;

pub(crate) fn fetch_santi_conversation_messages(
    santi_base_url: &str,
    conversation_id: &str,
) -> Result<SantiSessionMessagesResponse, String> {
    reqwest::blocking::Client::new()
        .get(format!(
            "{santi_base_url}/api/v1/sessions/{conversation_id}/messages"
        ))
        .send()
        .map_err(|error| format!("santi transcript request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("santi transcript status failed: {error}"))?
        .json::<SantiSessionMessagesResponse>()
        .map_err(|error| format!("santi transcript decode failed: {error}"))
}
