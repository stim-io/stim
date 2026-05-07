use utoipa::OpenApi;

use crate::{
    handler,
    schema::{
        AgentInstanceAction, AgentInstanceActionResponse, AgentInstanceActionStatus,
        AgentInstanceListResponse, AgentInstanceSnapshot, AgentInstanceState, AgentProcessFacts,
        AgentProcessStopFacts, AgentProfileApplyRequest, AgentProfileApplyResponse,
        AgentProfileApplyStatus, AgentProfileListResponse, AgentProfileSecretState,
        AgentProfileSummary, AgentProviderProbeResponse, AgentSelectionRequest,
        AgentSelectionResponse, ErrorResponse, SantiCapabilityFacts, SantiConfigFacts,
        SantiProviderFacts, SantiProviderProbeFacts, SantiProviderProbeState, SantiRuntimeFacts,
        SantiServiceFacts,
    },
};

#[derive(OpenApi)]
#[openapi(
    paths(
        handler::health,
        handler::list_instances,
        handler::list_profiles,
        handler::get_selection,
        handler::select_instance,
        handler::get_instance,
        handler::probe_instance,
        handler::probe_instance_provider,
        handler::apply_profile,
        handler::launch_instance,
        handler::stop_instance,
    ),
    components(schemas(
        ErrorResponse,
        AgentInstanceAction,
        AgentInstanceActionResponse,
        AgentInstanceActionStatus,
        AgentInstanceListResponse,
        AgentInstanceSnapshot,
        AgentInstanceState,
        AgentProcessFacts,
        AgentProcessStopFacts,
        AgentProfileApplyRequest,
        AgentProfileApplyResponse,
        AgentProfileApplyStatus,
        AgentProfileListResponse,
        AgentProfileSecretState,
        AgentProfileSummary,
        AgentProviderProbeResponse,
        AgentSelectionRequest,
        AgentSelectionResponse,
        SantiCapabilityFacts,
        SantiConfigFacts,
        SantiProviderFacts,
        SantiProviderProbeFacts,
        SantiProviderProbeState,
        SantiRuntimeFacts,
        SantiServiceFacts,
    )),
    tags(
        (name = "health"),
        (name = "agents"),
    )
)]
pub struct ApiDoc;
