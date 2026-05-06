use axum::Json;

pub(super) async fn health() -> Json<&'static str> {
    Json("ok")
}
