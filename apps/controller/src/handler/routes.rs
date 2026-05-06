use axum::{routing::get, routing::post, Router};
use tower_http::cors::{Any, CorsLayer};

use crate::model::ControllerHttpState;

use super::{
    conversations::conversation_messages,
    discovery::{discover_endpoint, register_endpoint, registry_snapshot},
    health::health,
    operation_socket::controller_operation_socket,
    roundtrip::first_message_roundtrip,
};

pub(crate) fn build_router(app_state: ControllerHttpState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route(
            "/api/v1/discovery/endpoints/{endpoint_id}",
            get(discover_endpoint).put(register_endpoint),
        )
        .route("/api/v1/debug/registry", get(registry_snapshot))
        .route("/api/v1/messages/roundtrip", post(first_message_roundtrip))
        .route(
            "/api/v1/controller/operations/ws",
            get(controller_operation_socket),
        )
        .route(
            "/api/v1/conversations/{conversation_id}/messages",
            get(conversation_messages),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any),
        )
        .with_state(app_state)
}
