use axum::http::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductChatTurnStart {
    pub(crate) session_id: String,
    pub(crate) user_message_id: String,
    pub(crate) assistant_message_id: String,
    pub(crate) user_participant_id: String,
    pub(crate) assistant_participant_id: String,
    pub(crate) user_text: String,
    pub(crate) operation_id: String,
    pub(crate) correlation_id: String,
    pub(crate) causation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductChatTurn {
    pub(crate) session_id: String,
    pub(crate) session_event_id: String,
    pub(crate) user_message_id: String,
    pub(crate) user_message_event_id: String,
    pub(crate) assistant_message_id: String,
    pub(crate) assistant_message_event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductChatTurnCompletion {
    pub(crate) assistant_message_id: String,
    pub(crate) assistant_chunk_event_id: Option<String>,
    pub(crate) assistant_final_event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProductChatTurnChunk {
    pub(crate) assistant_message_id: String,
    pub(crate) assistant_chunk_event_id: String,
}

pub(crate) fn start_product_chat_turn(
    base_url: &str,
    start: ProductChatTurnStart,
) -> Result<ProductChatTurn, (StatusCode, String)> {
    let client = reqwest::blocking::Client::new();
    let session = ensure_chat_session(&client, base_url, &start)?;
    let user_message = create_chat_message(
        &client,
        base_url,
        &start.session_id,
        ChatMessageCreateBody {
            message_id: &start.user_message_id,
            participant_id: &start.user_participant_id,
            message_kind: "user",
            content_kind: "text",
            state: "completed",
            initial_text: Some(&start.user_text),
            operation_id: Some(&start.operation_id),
            correlation_id: Some(&start.correlation_id),
            causation_id: start.causation_id.as_deref(),
        },
    )?;
    let assistant_message = create_chat_message(
        &client,
        base_url,
        &start.session_id,
        ChatMessageCreateBody {
            message_id: &start.assistant_message_id,
            participant_id: &start.assistant_participant_id,
            message_kind: "assistant",
            content_kind: "text",
            state: "pending",
            initial_text: None,
            operation_id: Some(&start.operation_id),
            correlation_id: Some(&start.correlation_id),
            causation_id: Some(&start.user_message_id),
        },
    )?;

    Ok(ProductChatTurn {
        session_id: session.session_id,
        session_event_id: session.last_event_id,
        user_message_id: user_message.message_id,
        user_message_event_id: user_message.last_event_id,
        assistant_message_id: assistant_message.message_id,
        assistant_message_event_id: assistant_message.last_event_id,
    })
}

pub(crate) fn complete_product_chat_turn(
    base_url: &str,
    turn: &ProductChatTurn,
    response_text: &str,
    causation_id: Option<&str>,
) -> Result<ProductChatTurnCompletion, (StatusCode, String)> {
    let client = reqwest::blocking::Client::new();
    let assistant_chunk_event_id = if response_text.is_empty() {
        None
    } else {
        let chunk = append_chat_message_chunk(
            &client,
            base_url,
            &turn.session_id,
            &turn.assistant_message_id,
            response_text,
            causation_id,
        )?;
        Some(chunk.last_event_id)
    };

    let finalized = post_json::<_, ChatMessageRecord>(
        &client,
        format!(
            "{base_url}/api/v1/chat/sessions/{}/messages/{}/finalize",
            turn.session_id, turn.assistant_message_id
        ),
        &ChatMessageFinalizeBody {
            state: "completed",
            failure_detail: None,
            causation_id,
        },
        "stim-server chat message finalize",
    )?;

    Ok(ProductChatTurnCompletion {
        assistant_message_id: finalized.message_id,
        assistant_chunk_event_id,
        assistant_final_event_id: finalized.last_event_id,
    })
}

pub(crate) fn append_chat_turn_chunk(
    base_url: &str,
    turn: &ProductChatTurn,
    text: &str,
    causation_id: Option<&str>,
) -> Result<ProductChatTurnChunk, (StatusCode, String)> {
    let client = reqwest::blocking::Client::new();
    let chunk = append_chat_message_chunk(
        &client,
        base_url,
        &turn.session_id,
        &turn.assistant_message_id,
        text,
        causation_id,
    )?;

    Ok(ProductChatTurnChunk {
        assistant_message_id: chunk.message_id,
        assistant_chunk_event_id: chunk.last_event_id,
    })
}

pub(crate) fn fail_product_chat_turn(
    base_url: &str,
    turn: &ProductChatTurn,
    failure_detail: &str,
    causation_id: Option<&str>,
) -> Result<ProductChatTurnCompletion, (StatusCode, String)> {
    let client = reqwest::blocking::Client::new();
    let finalized = post_json::<_, ChatMessageRecord>(
        &client,
        format!(
            "{base_url}/api/v1/chat/sessions/{}/messages/{}/finalize",
            turn.session_id, turn.assistant_message_id
        ),
        &ChatMessageFinalizeBody {
            state: "failed",
            failure_detail: Some(failure_detail),
            causation_id,
        },
        "stim-server chat message fail",
    )?;

    Ok(ProductChatTurnCompletion {
        assistant_message_id: finalized.message_id,
        assistant_chunk_event_id: None,
        assistant_final_event_id: finalized.last_event_id,
    })
}

fn append_chat_message_chunk(
    client: &reqwest::blocking::Client,
    base_url: &str,
    session_id: &str,
    message_id: &str,
    text: &str,
    causation_id: Option<&str>,
) -> Result<ChatMessageRecord, (StatusCode, String)> {
    post_json::<_, ChatMessageRecord>(
        client,
        format!("{base_url}/api/v1/chat/sessions/{session_id}/messages/{message_id}/chunks"),
        &ChatMessageChunkAppendBody { text, causation_id },
        "stim-server chat message chunk append",
    )
}

fn ensure_chat_session(
    client: &reqwest::blocking::Client,
    base_url: &str,
    start: &ProductChatTurnStart,
) -> Result<ChatSessionRecord, (StatusCode, String)> {
    let get_response = client
        .get(format!(
            "{base_url}/api/v1/chat/sessions/{}",
            start.session_id
        ))
        .send()
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("stim-server chat session request failed: {error}"),
            )
        })?;
    if get_response.status().is_success() {
        return decode_response(get_response, "stim-server chat session decode");
    }
    if get_response.status() != reqwest::StatusCode::NOT_FOUND {
        return status_error(get_response, "stim-server chat session status failed");
    }

    post_json::<_, ChatSessionRecord>(
        client,
        format!("{base_url}/api/v1/chat/sessions"),
        &ChatSessionCreateBody {
            session_id: &start.session_id,
            created_by_participant_id: &start.user_participant_id,
        },
        "stim-server chat session create",
    )
}

fn create_chat_message(
    client: &reqwest::blocking::Client,
    base_url: &str,
    session_id: &str,
    body: ChatMessageCreateBody<'_>,
) -> Result<ChatMessageRecord, (StatusCode, String)> {
    post_json::<_, ChatMessageRecord>(
        client,
        format!("{base_url}/api/v1/chat/sessions/{session_id}/messages"),
        &body,
        "stim-server chat message create",
    )
}

fn post_json<T, U>(
    client: &reqwest::blocking::Client,
    url: String,
    body: &T,
    context: &str,
) -> Result<U, (StatusCode, String)>
where
    T: Serialize,
    U: DeserializeOwned,
{
    let response = client.post(url).json(body).send().map_err(|error| {
        (
            StatusCode::BAD_GATEWAY,
            format!("{context} request failed: {error}"),
        )
    })?;
    decode_response(response, context)
}

fn decode_response<T>(
    response: reqwest::blocking::Response,
    context: &str,
) -> Result<T, (StatusCode, String)>
where
    T: DeserializeOwned,
{
    if !response.status().is_success() {
        return status_error(response, context);
    }

    response.json::<T>().map_err(|error| {
        (
            StatusCode::BAD_GATEWAY,
            format!("{context} decode failed: {error}"),
        )
    })
}

fn status_error<T>(
    response: reqwest::blocking::Response,
    context: &str,
) -> Result<T, (StatusCode, String)> {
    let status = response.status();
    let body = response.text().unwrap_or_default();
    Err((status, format!("{context}: {status} {body}")))
}

#[derive(Serialize)]
struct ChatSessionCreateBody<'a> {
    session_id: &'a str,
    created_by_participant_id: &'a str,
}

#[derive(Deserialize)]
struct ChatSessionRecord {
    session_id: String,
    last_event_id: String,
}

#[derive(Serialize)]
struct ChatMessageCreateBody<'a> {
    message_id: &'a str,
    participant_id: &'a str,
    message_kind: &'a str,
    content_kind: &'a str,
    state: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    initial_text: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    correlation_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    causation_id: Option<&'a str>,
}

#[derive(Serialize)]
struct ChatMessageChunkAppendBody<'a> {
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    causation_id: Option<&'a str>,
}

#[derive(Serialize)]
struct ChatMessageFinalizeBody<'a> {
    state: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_detail: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    causation_id: Option<&'a str>,
}

#[derive(Deserialize)]
struct ChatMessageRecord {
    message_id: String,
    last_event_id: String,
}
