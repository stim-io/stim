pub(crate) use std::collections::BTreeMap;

pub(crate) use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
pub(crate) use stim_agents::{
    app::build_router,
    state::{
        AppState, SantiInstanceConfig, SantiLaunchConfig, SantiProfileConfig,
        SantiProfileProviderConfig, SantiProfileSecretConfig,
    },
};
pub(crate) use tower::util::ServiceExt;

pub(crate) async fn start_mock_santi() -> String {
    async fn health() -> impl IntoResponse {
        (StatusCode::OK, axum::Json("ok"))
    }

    async fn meta() -> impl IntoResponse {
        axum::Json(serde_json::json!({
            "api_version": "v1",
            "service_name": "santi-api",
            "service_version": "0.1.0",
            "mode": "standalone",
            "launch_profile": "local-foreground",
            "bind_addr": "127.0.0.1:18081",
            "provider": {
                "api": "responses",
                "model": "gpt-5.4",
                "gateway_base_url": "http://127.0.0.1:18082/openai/v1"
            },
            "runtime": {
                "execution_root": "/workspace",
                "runtime_root": "/runtime",
                "standalone_sqlite_path": "/runtime/santi.sqlite"
            },
            "capabilities": {
                "health": true,
                "sessions": true,
                "soul": true,
                "admin_hooks": true,
                "streaming": true
            }
        }))
    }

    async fn provider_probe() -> impl IntoResponse {
        axum::Json(serde_json::json!({
            "state": "ready",
            "checked_url": "http://127.0.0.1:18082/openai/v1/health",
            "http_status": 200,
            "detail": "provider gateway health probe returned success"
        }))
    }

    async fn config() -> impl IntoResponse {
        axum::Json(serde_json::json!({
            "config_version": 2,
            "last_event_id": "config.applied-test",
            "source": "admin-apply",
            "launch_profile": "local-foreground",
            "provider": {
                "api": "responses",
                "model": "gpt-5.4",
                "gateway_base_url": "http://127.0.0.1:18082/openai/v1"
            },
            "runtime": {
                "execution_root": "/workspace",
                "runtime_root": "/runtime",
                "standalone_sqlite_path": "/runtime/santi.sqlite"
            }
        }))
    }

    async fn apply_config() -> impl IntoResponse {
        axum::Json(serde_json::json!({
            "event_id": "config.applied-test",
            "config_version": 3,
            "source": "admin-apply",
            "status": "applied",
            "launch_profile": "test-launch",
            "provider": {
                "api": "responses",
                "model": "gpt-test",
                "gateway_base_url": "http://127.0.0.1:18082/openai/v1"
            },
            "runtime": {
                "execution_root": "/workspace",
                "runtime_root": "/runtime",
                "standalone_sqlite_path": "/runtime/santi.sqlite"
            },
            "detail": "provider config applied for subsequent turns"
        }))
    }

    let app = Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/meta", get(meta))
        .route("/api/v1/admin/config", get(config))
        .route("/api/v1/admin/config/apply", post(apply_config))
        .route("/api/v1/admin/provider/probe", post(provider_probe));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{addr}")
}
