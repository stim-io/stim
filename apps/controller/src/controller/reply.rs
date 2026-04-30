use stim_proto::{MessageContent, ReplyEvent, ReplyEventKind, ReplySnapshot};

use super::{messages::assistant_card_content, types::ControllerError};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ControllerReplyContent {
    pub(super) text: String,
    pub(super) content: MessageContent,
}

pub(super) fn request_protocol_reply(
    santi_base_url: &str,
    reply_id: &str,
) -> Result<ControllerReplyContent, ControllerError> {
    let body = reqwest::blocking::Client::new()
        .get(format!(
            "{santi_base_url}/api/v1/stim/replies/{reply_id}/events"
        ))
        .send()
        .map_err(|error| ControllerError::Server(format!("reply event request failed: {error}")))?
        .error_for_status()
        .map_err(|error| ControllerError::Server(format!("reply event status failed: {error}")))?
        .text()
        .map_err(|error| {
            ControllerError::Server(format!("reply event body read failed: {error}"))
        })?;

    let streamed = parse_reply_event_stream(&body)?;
    let snapshot = reqwest::blocking::Client::new()
        .get(format!("{santi_base_url}/api/v1/stim/replies/{reply_id}"))
        .send()
        .map_err(|error| {
            ControllerError::Server(format!("reply snapshot request failed: {error}"))
        })?
        .error_for_status()
        .map_err(|error| ControllerError::Server(format!("reply snapshot status failed: {error}")))?
        .json::<ReplySnapshot>()
        .map_err(|error| {
            ControllerError::Server(format!("reply snapshot decode failed: {error}"))
        })?;

    if !snapshot.output_text.trim().is_empty() {
        return Ok(ControllerReplyContent {
            text: snapshot.output_text.clone(),
            content: assistant_card_content(&snapshot.output_text),
        });
    }

    Ok(ControllerReplyContent {
        text: streamed.clone(),
        content: assistant_card_content(&streamed),
    })
}

fn parse_reply_event_stream(body: &str) -> Result<String, ControllerError> {
    let mut reply = String::new();
    let mut completed = false;

    for line in body.lines() {
        let Some(payload) = line.strip_prefix("data:") else {
            continue;
        };
        let payload = payload.trim();

        if payload == "[DONE]" {
            break;
        }

        let event: ReplyEvent = serde_json::from_str(payload).map_err(|error| {
            ControllerError::Server(format!("reply event SSE decode failed: {error}"))
        })?;

        match event.event {
            ReplyEventKind::OutputTextDelta { delta } => {
                reply.push_str(&delta);
            }
            ReplyEventKind::Completed => {
                completed = true;
            }
            ReplyEventKind::Failed { error } => {
                return Err(ControllerError::Server(format!(
                    "reply event stream failed: {}: {}",
                    error.code, error.message
                )));
            }
        }
    }

    if reply.trim().is_empty() {
        return Err(ControllerError::Server(
            "reply event stream completed without assistant reply text".into(),
        ));
    }

    if !completed {
        return Err(ControllerError::Server(
            "reply event stream ended without completion event".into(),
        ));
    }

    Ok(reply)
}
