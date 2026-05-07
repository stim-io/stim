use std::io::Read;

use stim_proto::{MessageContent, ReplyEvent, ReplyEventKind, ReplySnapshot};

use crate::{factory::assistant_card_content, model::ControllerError};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ControllerReplyContent {
    pub(crate) text: String,
    pub(crate) content: MessageContent,
}

pub(crate) fn request_protocol_reply_stream<F>(
    santi_base_url: &str,
    reply_id: &str,
    on_delta: F,
) -> Result<ControllerReplyContent, ControllerError>
where
    F: FnMut(&str) -> Result<(), ControllerError>,
{
    let client = reqwest::blocking::Client::new();
    let mut response = client
        .get(format!(
            "{santi_base_url}/api/v1/stim/replies/{reply_id}/events"
        ))
        .send()
        .map_err(|error| ControllerError::Server(format!("reply event request failed: {error}")))?
        .error_for_status()
        .map_err(|error| ControllerError::Server(format!("reply event status failed: {error}")))?;

    let streamed = parse_reply_event_stream(&mut response, on_delta)?;
    let snapshot = client
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

fn parse_reply_event_stream<R, F>(
    reader: &mut R,
    mut on_delta: F,
) -> Result<String, ControllerError>
where
    R: Read,
    F: FnMut(&str) -> Result<(), ControllerError>,
{
    let mut state = ReplyStreamState::default();
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 8192];

    loop {
        let read = reader.read(&mut chunk).map_err(|error| {
            ControllerError::Server(format!("reply event body read failed: {error}"))
        })?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);

        while let Some((frame_end, separator_len)) = next_sse_frame_end(&buffer) {
            let frame = buffer.drain(..frame_end).collect::<Vec<_>>();
            buffer.drain(..separator_len);
            handle_reply_event_frame(&frame, &mut state, &mut on_delta)?;
            if state.done {
                break;
            }
        }

        if state.done {
            break;
        }
    }

    if !state.done && buffer.iter().any(|byte| !byte.is_ascii_whitespace()) {
        handle_reply_event_frame(&buffer, &mut state, &mut on_delta)?;
    }

    if state.reply.trim().is_empty() {
        return Err(ControllerError::Server(
            "reply event stream completed without assistant reply text".into(),
        ));
    }

    if !state.completed {
        return Err(ControllerError::Server(
            "reply event stream ended without completion event".into(),
        ));
    }

    Ok(state.reply)
}

#[derive(Default)]
struct ReplyStreamState {
    reply: String,
    completed: bool,
    done: bool,
}

fn handle_reply_event_frame<F>(
    frame: &[u8],
    state: &mut ReplyStreamState,
    on_delta: &mut F,
) -> Result<(), ControllerError>
where
    F: FnMut(&str) -> Result<(), ControllerError>,
{
    let frame = String::from_utf8(frame.to_vec()).map_err(|error| {
        ControllerError::Server(format!("reply event SSE utf-8 decode failed: {error}"))
    })?;
    let payload = frame
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");
    let payload = payload.trim();
    if payload.is_empty() {
        return Ok(());
    }
    if payload == "[DONE]" {
        state.done = true;
        return Ok(());
    }

    let event: ReplyEvent = serde_json::from_str(payload).map_err(|error| {
        ControllerError::Server(format!("reply event SSE decode failed: {error}"))
    })?;

    match event.event {
        ReplyEventKind::OutputTextDelta { delta } => {
            if !delta.is_empty() {
                state.reply.push_str(&delta);
                on_delta(&delta)?;
            }
        }
        ReplyEventKind::Completed => {
            state.completed = true;
        }
        ReplyEventKind::Failed { error } => {
            return Err(ControllerError::Server(format!(
                "reply event stream failed: {}: {}",
                error.code, error.message
            )));
        }
    }

    Ok(())
}

fn next_sse_frame_end(buffer: &[u8]) -> Option<(usize, usize)> {
    let lf = buffer
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|index| (index, 2));
    let crlf = buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| (index, 4));

    match (lf, crlf) {
        (Some(lf), Some(crlf)) => Some(if lf.0 < crlf.0 { lf } else { crlf }),
        (Some(lf), None) => Some(lf),
        (None, Some(crlf)) => Some(crlf),
        (None, None) => None,
    }
}
