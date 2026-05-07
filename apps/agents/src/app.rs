use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{handler, openapi::ApiDoc, state::AppState};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(handler::health))
        .route("/api/v1/agents/instances", get(handler::list_instances))
        .route("/api/v1/agents/profiles", get(handler::list_profiles))
        .route(
            "/api/v1/agents/selection",
            get(handler::get_selection).put(handler::select_instance),
        )
        .route(
            "/api/v1/agents/instances/{instance_id}",
            get(handler::get_instance),
        )
        .route(
            "/api/v1/agents/instances/{instance_id}/probe",
            post(handler::probe_instance),
        )
        .route(
            "/api/v1/agents/instances/{instance_id}/provider/probe",
            post(handler::probe_instance_provider),
        )
        .route(
            "/api/v1/agents/instances/{instance_id}/profiles/apply",
            post(handler::apply_profile),
        )
        .route(
            "/api/v1/agents/instances/{instance_id}/launch",
            post(handler::launch_instance),
        )
        .route(
            "/api/v1/agents/instances/{instance_id}/stop",
            post(handler::stop_instance),
        )
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", ApiDoc::openapi()))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any),
        )
        .with_state(state)
}
