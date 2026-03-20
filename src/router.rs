use crate::handler::{config_handler, file_handler, subtitle_task};
use crate::AppState;
use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use rust_embed::Embed;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

pub fn build_router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/api/capability/subtitleTask", post(subtitle_task::start_task))
        .route("/api/capability/subtitleTask", get(subtitle_task::get_task))
        .route("/api/file", post(file_handler::upload_file))
        .route("/api/file/*filepath", get(file_handler::download_file))
        .route("/api/config", get(config_handler::get_config))
        .route("/api/config", post(config_handler::update_config));

    Router::new()
        .merge(api)
        .route("/", get(serve_index))
        .route("/static/{*path}", get(serve_static))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn serve_index() -> impl IntoResponse {
    match StaticAssets::get("index.html") {
        Some(content) => Html(String::from_utf8_lossy(&content.data).to_string()).into_response(),
        None => (axum::http::StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

async fn serve_static(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    match StaticAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            (
                [(header::CONTENT_TYPE, mime)],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => (axum::http::StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}
