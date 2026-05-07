use std::{fs, path::PathBuf, time::Duration};

use serde::Serialize;
use serde_json::{json, Value};
use stim_shared::inspection::{
    RendererActionFailureReason, RendererActionRequest, RendererActionResult,
    RendererActionSnapshot, RendererMessagingSendSnapshot,
};

use crate::{
    control::current_namespace,
    shared::{
        bridge::{request_controller_runtime, request_renderer_action},
        clock::timestamp_now,
    },
};

const DEFAULT_SERVER_BASE_URL: &str = "http://127.0.0.1:18083";
const DEFAULT_SANTI_BASE_URL: &str = "http://127.0.0.1:18081";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatRunScenario {
    pub(crate) new_conversation: bool,
    pub(crate) turns: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ChatHarnessEvent {
    sequence: usize,
    emitted_at: String,
    namespace: &'static str,
    eventkey: &'static str,
    payload: Value,
}

pub(crate) fn chat(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [subcommand, rest @ ..] if subcommand == "run" => run_chat(parse_run_args(rest)?),
        [subcommand, run_id] if subcommand == "inspect" => inspect_run(run_id),
        [subcommand] if subcommand == "inspect" => inspect_latest_run(),
        [] => Err("chat requires a subcommand; supported leaves: run [--new] [--turn <text>] [text], inspect [run_id]".into()),
        [subcommand, ..] => Err(format!(
            "unsupported chat leaf: {subcommand}; supported leaves: run [--new] [--turn <text>] [text], inspect [run_id]"
        )),
    }
}

fn run_chat(scenario: ChatRunScenario) -> Result<(), String> {
    let namespace = current_namespace();
    let run_id = format!("chat-{}", timestamp_now());
    let mut trace = ChatTrace::new(run_id.clone());
    trace.push(
        "run-started",
        json!({
            "command": "stim-dev chat run",
            "namespace": namespace,
            "scenario": {
                "new_conversation": scenario.new_conversation,
                "turns": scenario.turns,
            },
        }),
    );

    let controller_runtime = request_controller_runtime(Duration::from_secs(5))
        .map_err(|error| {
            format!(
                "chat run requires a running app loop; run 'stim-dev detect' and 'stim-dev restart' first: {error}"
            )
        })?;
    trace.push(
        "controller-runtime-observed",
        json!({ "controller": controller_runtime.snapshot }),
    );

    if scenario.new_conversation {
        trace.push("renderer-new-conversation-requested", json!({}));
        let new_conversation =
            request_renderer_action(RendererActionRequest::MessagingNewConversation)?;
        trace.push(
            "renderer-new-conversation-completed",
            action_result_json(new_conversation),
        );
    }

    let mut turn_reports = Vec::new();
    let mut last_snapshot = None;

    for (index, text) in scenario.turns.iter().enumerate() {
        let turn_index = index + 1;
        trace.push(
            "turn-submitted",
            json!({
                "turn_index": turn_index,
                "text": text,
                "target_endpoint_id": "endpoint-b",
            }),
        );
        let result = request_renderer_action(RendererActionRequest::MessagingSend {
            text: text.clone(),
            target_endpoint_id: Some("endpoint-b".into()),
        })?;
        trace.push(
            "renderer-action-completed",
            json!({
                "turn_index": turn_index,
                "result": action_result_json(result.clone()),
            }),
        );
        let snapshot = require_messaging_send_snapshot(result, turn_index)?;
        trace.push(
            "renderer-projection-observed",
            json!({
                "turn_index": turn_index,
                "projection": snapshot.after,
            }),
        );
        assert_renderer_projection(&snapshot, text, turn_index)?;
        last_snapshot = Some(snapshot.clone());
        turn_reports.push(json!({
            "turn_index": turn_index,
            "submitted_text": text,
            "conversation_id": snapshot.after.active_conversation_id,
            "last_assistant_text": snapshot.after.last_assistant_text,
            "tool_activity_summary": snapshot.after.latest_tool_activity_summary,
        }));
    }

    let final_snapshot =
        last_snapshot.ok_or_else(|| "chat run scenario has no turns".to_string())?;
    let conversation_id = final_snapshot
        .after
        .active_conversation_id
        .clone()
        .ok_or_else(|| "chat run did not expose an active conversation id".to_string())?;

    let stim_server_session = fetch_json(
        &stim_server_url(&format!(
            "/api/v1/chat/sessions/{}",
            percent_encode_path_segment(&conversation_id)
        )),
        "stim-server chat session",
    );
    trace_observation(
        &mut trace,
        "stim-server-session-observed",
        "stim-server-session-observation-failed",
        &stim_server_session,
    );

    let stim_server_messages = fetch_json(
        &stim_server_url(&format!(
            "/api/v1/chat/sessions/{}/messages",
            percent_encode_path_segment(&conversation_id)
        )),
        "stim-server chat messages",
    );
    trace_observation(
        &mut trace,
        "stim-server-messages-observed",
        "stim-server-messages-observation-failed",
        &stim_server_messages,
    );

    let santi_tool_activities = fetch_json(
        &santi_url(&format!(
            "/api/v1/sessions/{}/tool-activities",
            percent_encode_path_segment(&conversation_id)
        )),
        "santi tool activities",
    );
    trace_observation(
        &mut trace,
        "santi-tool-activities-observed",
        "santi-tool-activities-observation-failed",
        &santi_tool_activities,
    );

    let run_path = write_run_trace(&namespace, &run_id, &trace.events)?;
    let output = json!({
        "namespace": namespace,
        "command": "stim-dev chat run",
        "run_id": run_id,
        "run_path": run_path.display().to_string(),
        "state": "completed",
        "conversation_id": conversation_id,
        "turns": turn_reports,
        "final_renderer_projection": final_snapshot.after,
        "stim_server_session": observation_value(stim_server_session),
        "stim_server_messages": observation_value(stim_server_messages),
        "santi_tool_activities": observation_value(santi_tool_activities),
        "events": trace.events,
    });
    print_json(&output, "chat run")?;
    Ok(())
}

pub(crate) fn parse_run_args(args: &[String]) -> Result<ChatRunScenario, String> {
    let mut new_conversation = false;
    let mut explicit_turns = Vec::new();
    let mut positional = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--new" => {
                new_conversation = true;
                index += 1;
            }
            "--turn" => {
                let text = args
                    .get(index + 1)
                    .ok_or_else(|| "--turn requires text".to_string())?
                    .trim()
                    .to_string();
                if text.is_empty() {
                    return Err("--turn text must not be empty".into());
                }
                explicit_turns.push(text);
                index += 2;
            }
            value if value.starts_with("--") => {
                return Err(format!(
                    "unsupported chat run option: {value}; supported options: --new, --turn <text>"
                ));
            }
            value => {
                positional.push(value.to_string());
                index += 1;
            }
        }
    }

    if !explicit_turns.is_empty() && !positional.is_empty() {
        return Err("chat run cannot mix --turn entries with positional text".into());
    }

    let turns = if explicit_turns.is_empty() {
        let text = positional.join(" ").trim().to_string();
        if text.is_empty() {
            vec![format!("stim-dev chat run {}", timestamp_now())]
        } else {
            vec![text]
        }
    } else {
        explicit_turns
    };

    Ok(ChatRunScenario {
        new_conversation,
        turns,
    })
}

fn inspect_latest_run() -> Result<(), String> {
    let runs_dir = runs_dir(&current_namespace());
    let latest = fs::read_dir(&runs_dir)
        .map_err(|error| format!("failed to read {}: {error}", runs_dir.display()))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join("trace.json").is_file())
        .max_by_key(|entry| entry.file_name());
    let latest = latest
        .ok_or_else(|| format!("no chat runs found under {}", runs_dir.display()))?
        .file_name()
        .to_string_lossy()
        .to_string();

    inspect_run(&latest)
}

fn inspect_run(run_id: &str) -> Result<(), String> {
    let path = run_trace_path(&current_namespace(), run_id);
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    println!("{content}");
    Ok(())
}

struct ChatTrace {
    next_sequence: usize,
    events: Vec<ChatHarnessEvent>,
}

impl ChatTrace {
    fn new(_run_id: String) -> Self {
        Self {
            next_sequence: 1,
            events: Vec::new(),
        }
    }

    fn push(&mut self, eventkey: &'static str, payload: Value) {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        self.events.push(ChatHarnessEvent {
            sequence,
            emitted_at: timestamp_now(),
            namespace: "stim-dev.chat",
            eventkey,
            payload,
        });
    }
}

fn require_messaging_send_snapshot(
    result: RendererActionResult,
    turn_index: usize,
) -> Result<RendererMessagingSendSnapshot, String> {
    match result {
        RendererActionResult::Success { snapshot } => match *snapshot {
            RendererActionSnapshot::MessagingSend(snapshot) => Ok(snapshot),
            RendererActionSnapshot::MessagingNewConversation(_) => Err(format!(
                "chat run turn {turn_index} returned new-conversation snapshot, expected messaging-send"
            )),
        },
        RendererActionResult::Failure { reason, detail } => Err(format!(
            "chat run turn {turn_index} renderer action failed: {}{}",
            action_failure_reason_name(reason),
            detail
                .map(|value| format!(" ({value})"))
                .unwrap_or_default()
        )),
    }
}

fn assert_renderer_projection(
    snapshot: &RendererMessagingSendSnapshot,
    expected_user_text: &str,
    turn_index: usize,
) -> Result<(), String> {
    if let Some(error) = snapshot.after.error_message.as_deref() {
        return Err(format!(
            "chat run turn {turn_index} renderer reported visible error: {error}"
        ));
    }
    if snapshot.after.active_conversation_id.is_none() {
        return Err(format!(
            "chat run turn {turn_index} did not expose active conversation id"
        ));
    }
    if !snapshot
        .after
        .last_user_text
        .as_deref()
        .is_some_and(|text| text.contains(expected_user_text))
    {
        return Err(format!(
            "chat run turn {turn_index} last user text did not include submitted text"
        ));
    }
    if snapshot
        .after
        .last_assistant_text
        .as_deref()
        .is_none_or(str::is_empty)
    {
        return Err(format!(
            "chat run turn {turn_index} did not expose visible assistant text"
        ));
    }

    Ok(())
}

fn trace_observation(
    trace: &mut ChatTrace,
    success_eventkey: &'static str,
    failure_eventkey: &'static str,
    observation: &Result<Value, String>,
) {
    match observation {
        Ok(value) => trace.push(success_eventkey, value.clone()),
        Err(error) => trace.push(failure_eventkey, json!({ "error": error })),
    }
}

fn observation_value(observation: Result<Value, String>) -> Value {
    observation.unwrap_or_else(|error| json!({ "state": "unavailable", "error": error }))
}

fn fetch_json(url: &str, label: &str) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|error| format!("failed to build {label} client: {error}"))?;
    let response = client
        .get(url)
        .send()
        .map_err(|error| format!("{label} GET {url} failed: {error}"))?;
    let status = response.status();

    if !status.is_success() {
        let body = response
            .text()
            .unwrap_or_else(|error| format!("<failed to read body: {error}>"));
        return Err(format!("{label} GET {url} returned {status}: {body}"));
    }

    response
        .json::<Value>()
        .map_err(|error| format!("{label} GET {url} returned invalid JSON: {error}"))
}

fn stim_server_url(path: &str) -> String {
    format!(
        "{}{}",
        service_base_url("STIM_SERVER_BASE_URL", DEFAULT_SERVER_BASE_URL),
        path
    )
}

fn santi_url(path: &str) -> String {
    format!(
        "{}{}",
        service_base_url("SANTI_BASE_URL", DEFAULT_SANTI_BASE_URL),
        path
    )
}

fn service_base_url(env_key: &str, fallback: &str) -> String {
    std::env::var(env_key)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn write_run_trace(
    namespace: &str,
    run_id: &str,
    events: &[ChatHarnessEvent],
) -> Result<PathBuf, String> {
    let run_dir = runs_dir(namespace).join(run_id);
    fs::create_dir_all(&run_dir)
        .map_err(|error| format!("failed to create {}: {error}", run_dir.display()))?;
    let path = run_dir.join("trace.json");
    let content = serde_json::to_string_pretty(events)
        .map_err(|error| format!("failed to serialize chat event trace: {error}"))?;
    fs::write(&path, content)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(path)
}

fn runs_dir(namespace: &str) -> PathBuf {
    stim_platform::paths::dev_root()
        .join(namespace)
        .join("runs")
}

fn run_trace_path(namespace: &str, run_id: &str) -> PathBuf {
    runs_dir(namespace).join(run_id).join("trace.json")
}

pub(crate) fn action_result_json(result: RendererActionResult) -> Value {
    match result {
        RendererActionResult::Success { snapshot } => {
            json!({ "state": "completed", "snapshot": snapshot })
        }
        RendererActionResult::Failure { reason, detail } => json!({
            "state": "failed",
            "reason": action_failure_reason_name(reason),
            "detail": detail,
        }),
    }
}

fn action_failure_reason_name(reason: RendererActionFailureReason) -> &'static str {
    match reason {
        RendererActionFailureReason::NoMainWindow => "no-main-window",
        RendererActionFailureReason::ActionFailed => "action-failed",
        RendererActionFailureReason::ActionTimedOut => "action-timed-out",
    }
}

pub(crate) fn percent_encode_path_segment(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn print_json(value: &Value, context: &str) -> Result<(), String> {
    let output = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize {context}: {error}"))?;
    println!("{output}");
    Ok(())
}
