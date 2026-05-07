use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    schema::{
        AgentInstanceActionResponse, AgentInstanceListResponse, AgentInstanceSnapshot,
        AgentProfileApplyRequest, AgentProfileApplyResponse, AgentProfileListResponse,
        AgentProviderProbeResponse, AgentSelectionRequest, AgentSelectionResponse, ErrorResponse,
    },
    state::{AgentRegistryError, AppState},
};

#[utoipa::path(
    get,
    path = "/api/v1/health",
    operation_id = "health",
    tag = "health",
    responses((status = 200, description = "Health check response", body = String))
)]
pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json("ok"))
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/instances",
    operation_id = "list_agent_instances",
    tag = "agents",
    responses((status = 200, description = "Known agent runtime instances", body = AgentInstanceListResponse))
)]
pub async fn list_instances(State(state): State<AppState>) -> Json<AgentInstanceListResponse> {
    let registry = state.registry();
    Json(AgentInstanceListResponse {
        active_instance_id: registry.active_instance_id().await,
        instances: registry.list_instances().await,
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/profiles",
    operation_id = "list_agent_profiles",
    tag = "agents",
    responses((status = 200, description = "Known agent provider profiles", body = AgentProfileListResponse))
)]
pub async fn list_profiles(State(state): State<AppState>) -> Json<AgentProfileListResponse> {
    Json(AgentProfileListResponse {
        profiles: state.registry().list_profiles(),
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/selection",
    operation_id = "get_agent_selection",
    tag = "agents",
    responses((status = 200, description = "Current active agent instance selection", body = AgentSelectionResponse))
)]
pub async fn get_selection(State(state): State<AppState>) -> Json<AgentSelectionResponse> {
    Json(AgentSelectionResponse {
        active_instance_id: state.registry().active_instance_id().await,
    })
}

#[utoipa::path(
    put,
    path = "/api/v1/agents/selection",
    operation_id = "select_agent_instance",
    tag = "agents",
    request_body(content = AgentSelectionRequest),
    responses(
        (status = 200, description = "Updated active agent instance selection", body = AgentSelectionResponse),
        (status = 404, description = "Agent instance not found", body = ErrorResponse)
    )
)]
pub async fn select_instance(
    State(state): State<AppState>,
    Json(request): Json<AgentSelectionRequest>,
) -> Result<Json<AgentSelectionResponse>, ApiError> {
    match state.registry().select_instance(&request.instance_id).await {
        Ok(active_instance_id) => Ok(Json(AgentSelectionResponse { active_instance_id })),
        Err(AgentRegistryError::NotFound) => {
            Err(ApiError::not_found("agent instance not registered"))
        }
        Err(error) => Err(ApiError::bad_gateway(error.message())),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/agents/instances/{instance_id}",
    operation_id = "get_agent_instance",
    tag = "agents",
    params(("instance_id" = String, Path, description = "Agent instance identifier")),
    responses(
        (status = 200, description = "Agent runtime instance snapshot", body = AgentInstanceSnapshot),
        (status = 404, description = "Agent instance not found", body = ErrorResponse)
    )
)]
pub async fn get_instance(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
) -> Result<Json<AgentInstanceSnapshot>, ApiError> {
    state
        .registry()
        .get_instance(&instance_id)
        .await
        .map(Json)
        .ok_or_else(|| ApiError::not_found("agent instance not registered"))
}

#[utoipa::path(
    post,
    path = "/api/v1/agents/instances/{instance_id}/profiles/apply",
    operation_id = "apply_agent_profile",
    tag = "agents",
    params(("instance_id" = String, Path, description = "Agent instance identifier")),
    request_body(content = AgentProfileApplyRequest),
    responses(
        (status = 200, description = "Agent profile applied to Santi instance", body = AgentProfileApplyResponse),
        (status = 400, description = "Profile cannot be applied", body = ErrorResponse),
        (status = 404, description = "Agent instance or profile not found", body = ErrorResponse),
        (status = 502, description = "Santi config apply failed", body = ErrorResponse)
    )
)]
pub async fn apply_profile(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
    Json(request): Json<AgentProfileApplyRequest>,
) -> Result<Json<AgentProfileApplyResponse>, ApiError> {
    match state
        .registry()
        .apply_profile(&instance_id, &request.profile_id)
        .await
    {
        Ok(response) => Ok(Json(response)),
        Err(AgentRegistryError::NotFound | AgentRegistryError::ProfileNotFound) => Err(
            ApiError::not_found("agent instance or profile not registered"),
        ),
        Err(AgentRegistryError::SecretMissing(_)) => Err(ApiError::bad_request(
            "agent profile secret is not available to agents",
        )),
        Err(error) => Err(ApiError::bad_gateway(error.message())),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/agents/instances/{instance_id}/probe",
    operation_id = "probe_agent_instance",
    tag = "agents",
    params(("instance_id" = String, Path, description = "Agent instance identifier")),
    responses(
        (status = 200, description = "Fresh agent runtime instance probe", body = AgentInstanceSnapshot),
        (status = 404, description = "Agent instance not found", body = ErrorResponse)
    )
)]
pub async fn probe_instance(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
) -> Result<Json<AgentInstanceSnapshot>, ApiError> {
    get_instance(State(state), Path(instance_id)).await
}

#[utoipa::path(
    post,
    path = "/api/v1/agents/instances/{instance_id}/provider/probe",
    operation_id = "probe_agent_instance_provider",
    tag = "agents",
    params(("instance_id" = String, Path, description = "Agent instance identifier")),
    responses(
        (status = 200, description = "Fresh Santi-owned provider probe for the agent instance", body = AgentProviderProbeResponse),
        (status = 404, description = "Agent instance not found", body = ErrorResponse),
        (status = 502, description = "Santi provider probe could not be retrieved", body = ErrorResponse)
    )
)]
pub async fn probe_instance_provider(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
) -> Result<Json<AgentProviderProbeResponse>, ApiError> {
    match state.registry().probe_instance_provider(&instance_id).await {
        Ok(provider_probe) => Ok(Json(AgentProviderProbeResponse {
            instance_id,
            provider_probe,
        })),
        Err(AgentRegistryError::NotFound) => {
            Err(ApiError::not_found("agent instance not registered"))
        }
        Err(error) => Err(ApiError::bad_gateway(error.message())),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/agents/instances/{instance_id}/launch",
    operation_id = "launch_agent_instance",
    tag = "agents",
    params(("instance_id" = String, Path, description = "Agent instance identifier")),
    responses(
        (status = 200, description = "Managed Santi instance launch action result", body = AgentInstanceActionResponse),
        (status = 400, description = "Agent instance is not launchable", body = ErrorResponse),
        (status = 404, description = "Agent instance not found", body = ErrorResponse),
        (status = 409, description = "Managed Santi instance is already running under this agents sidecar", body = ErrorResponse),
        (status = 502, description = "Managed Santi instance could not be launched", body = ErrorResponse)
    )
)]
pub async fn launch_instance(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
) -> Result<Json<AgentInstanceActionResponse>, ApiError> {
    match state.registry().launch_instance(&instance_id).await {
        Ok(response) => Ok(Json(response)),
        Err(error) => Err(ApiError::from_registry_error(error)),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/agents/instances/{instance_id}/stop",
    operation_id = "stop_agent_instance",
    tag = "agents",
    params(("instance_id" = String, Path, description = "Agent instance identifier")),
    responses(
        (status = 200, description = "Managed Santi instance stop action result", body = AgentInstanceActionResponse),
        (status = 400, description = "Agent instance is not stoppable", body = ErrorResponse),
        (status = 404, description = "Agent instance not found", body = ErrorResponse),
        (status = 409, description = "Managed Santi instance is not running under this agents sidecar", body = ErrorResponse),
        (status = 502, description = "Managed Santi instance could not be stopped", body = ErrorResponse)
    )
)]
pub async fn stop_instance(
    State(state): State<AppState>,
    Path(instance_id): Path<String>,
) -> Result<Json<AgentInstanceActionResponse>, ApiError> {
    match state.registry().stop_instance(&instance_id).await {
        Ok(response) => Ok(Json(response)),
        Err(error) => Err(ApiError::from_registry_error(error)),
    }
}

pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "bad_request",
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: "conflict",
            message: message.into(),
        }
    }

    fn bad_gateway(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            code: "bad_gateway",
            message: message.into(),
        }
    }

    fn from_registry_error(error: AgentRegistryError) -> Self {
        match error {
            AgentRegistryError::NotFound => Self::not_found("agent instance not registered"),
            AgentRegistryError::ProfileNotFound => Self::not_found("agent profile not registered"),
            AgentRegistryError::Unmanaged | AgentRegistryError::LaunchUnavailable => {
                Self::bad_request(error.message())
            }
            AgentRegistryError::SecretMissing(_) => Self::bad_request(error.message()),
            AgentRegistryError::AlreadyRunning | AgentRegistryError::NotRunning => {
                Self::conflict(error.message())
            }
            AgentRegistryError::LaunchFailed(_)
            | AgentRegistryError::StopFailed(_)
            | AgentRegistryError::RequestFailed(_)
            | AgentRegistryError::BadStatus(_)
            | AgentRegistryError::DecodeFailed(_) => Self::bad_gateway(error.message()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(ErrorResponse {
                code: self.code.to_string(),
                message: self.message,
            }),
        )
            .into_response()
    }
}
