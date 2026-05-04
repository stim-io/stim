use serde::{Deserialize, Serialize};

use crate::control_plane::{ControllerRuntimeHeartbeat, ControllerRuntimeSnapshot};

pub const RENDERER_PROBE_REQUEST_EVENT: &str = "stim://inspection/renderer-probe-request";
pub const RENDERER_PROBE_RESPONSE_EVENT: &str = "stim://inspection/renderer-probe-response";
pub const RENDERER_ACTION_REQUEST_EVENT: &str = "stim://inspection/renderer-action-request";
pub const RENDERER_ACTION_RESPONSE_EVENT: &str = "stim://inspection/renderer-action-response";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectBridgeRequest {
    pub request_id: String,
    pub requested_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectBridgeResponse {
    pub request_id: String,
    pub requested_at: String,
    pub responded_at: String,
    pub result: InspectResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerRuntimeBridgeRequest {
    pub request_id: String,
    pub requested_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerRuntimeBridgeResponse {
    pub request_id: String,
    pub requested_at: String,
    pub responded_at: String,
    pub snapshot: ControllerRuntimeSnapshot,
    pub heartbeat: ControllerRuntimeHeartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum InspectResult {
    Success { snapshot: InspectSnapshot },
    Failure { reason: InspectFailureReason },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InspectFailureReason {
    NoMainWindow,
    InspectFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererProbeBridgeRequest {
    pub request_id: String,
    pub requested_at: String,
    pub probe: RendererProbeRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererProbeBridgeResponse {
    pub request_id: String,
    pub requested_at: String,
    pub responded_at: String,
    pub result: RendererProbeResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererActionBridgeRequest {
    pub request_id: String,
    pub requested_at: String,
    pub action: RendererActionRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererActionBridgeResponse {
    pub request_id: String,
    pub requested_at: String,
    pub responded_at: String,
    pub result: RendererActionResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "probe", rename_all = "kebab-case")]
pub enum RendererProbeRequest {
    LandingBasics,
    MessagingState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum RendererActionRequest {
    MessagingNewConversation,
    MessagingSend {
        text: String,
        target_endpoint_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RendererProbeResult {
    Success { snapshot: RendererProbeSnapshot },
    Failure { reason: RendererProbeFailureReason },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RendererActionResult {
    Success {
        snapshot: RendererActionSnapshot,
    },
    Failure {
        reason: RendererActionFailureReason,
        detail: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RendererProbeFailureReason {
    NoMainWindow,
    ProbeFailed,
    ProbeTimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RendererActionFailureReason {
    NoMainWindow,
    ActionFailed,
    ActionTimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererProbeSnapshot {
    pub inspected_at: String,
    pub probe: RendererProbeSnapshotKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RendererProbeSnapshotKind {
    LandingBasics(RendererLandingBasicsSnapshot),
    MessagingState(RendererMessagingStateSnapshot),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum RendererActionSnapshot {
    MessagingNewConversation(RendererMessagingNewConversationSnapshot),
    MessagingSend(RendererMessagingSendSnapshot),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererLandingBasicsSnapshot {
    pub document_ready_state: String,
    pub document_title: String,
    pub landing_shell_present: bool,
    pub landing_card_present: bool,
    pub session_drawer_present: bool,
    pub session_drawer_collapsed: bool,
    pub landing_title_text: Option<String>,
    pub primary_action_label: Option<String>,
    pub active_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererMessagingStateSnapshot {
    pub document_ready_state: String,
    pub active_session_id: Option<String>,
    pub active_conversation_id: Option<String>,
    pub chat_entry_count: usize,
    pub user_entry_count: usize,
    pub assistant_entry_count: usize,
    pub last_user_text: Option<String>,
    pub last_assistant_text: Option<String>,
    pub response_text: Option<String>,
    pub response_source: Option<String>,
    pub final_sent_text: Option<String>,
    pub assistant_response_content_kind: Option<String>,
    pub assistant_fragment_present: bool,
    pub error_message: Option<String>,
    pub primary_action_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererMessagingSendSnapshot {
    pub submitted_text: String,
    pub target_endpoint_id: String,
    pub before: RendererMessagingStateSnapshot,
    pub after: RendererMessagingStateSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererMessagingNewConversationSnapshot {
    pub before: RendererMessagingStateSnapshot,
    pub after: RendererMessagingStateSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererProbeEventRequest {
    pub request_id: String,
    pub requested_at: String,
    pub probe: RendererProbeRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererProbeEventResponse {
    pub request_id: String,
    pub requested_at: String,
    pub result: RendererProbeResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererActionEventRequest {
    pub request_id: String,
    pub requested_at: String,
    pub action: RendererActionRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendererActionEventResponse {
    pub request_id: String,
    pub requested_at: String,
    pub result: RendererActionResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectSnapshot {
    pub inspected_at: String,
    pub app: AppInspectSnapshot,
    pub window: WindowInspectSnapshot,
    pub current_monitor: Option<MonitorInspectSnapshot>,
    pub primary_monitor: Option<MonitorInspectSnapshot>,
    pub available_monitor_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInspectSnapshot {
    pub name: String,
    pub version: String,
    pub crate_name: String,
    pub expected_renderer_origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInspectSnapshot {
    pub label: String,
    pub title: String,
    pub url: String,
    pub scale_factor: f64,
    pub inner_size: PhysicalSizeSnapshot,
    pub outer_size: PhysicalSizeSnapshot,
    pub outer_position: PhysicalPositionSnapshot,
    pub is_visible: bool,
    pub is_focused: bool,
    pub is_minimized: bool,
    pub is_maximized: bool,
    pub is_fullscreen: bool,
    pub is_decorated: bool,
    pub is_resizable: bool,
    pub is_enabled: bool,
    pub matches_expected_renderer_origin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInspectSnapshot {
    pub name: Option<String>,
    pub scale_factor: f64,
    pub size: PhysicalSizeSnapshot,
    pub position: PhysicalPositionSnapshot,
    pub work_area: PhysicalRectSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalSizeSnapshot {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalPositionSnapshot {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalRectSnapshot {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotBridgeRequest {
    pub request_id: String,
    pub requested_at: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ScreenshotResult {
    Success {
        path: String,
        captured_at: String,
        label: Option<String>,
    },
    Failure {
        reason: ScreenshotFailureReason,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScreenshotFailureReason {
    NoMainWindow,
    UnsupportedPlatform,
    CaptureFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotBridgeResponse {
    pub request_id: String,
    pub requested_at: String,
    pub responded_at: String,
    pub result: ScreenshotResult,
}
