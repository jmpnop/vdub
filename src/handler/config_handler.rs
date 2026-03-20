use crate::dto::ApiResponse;
use crate::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use std::sync::Arc;

pub async fn get_config(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let config = state.config.read().await;
    ApiResponse::success(config.clone())
}

pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(new_config): Json<crate::config::Config>,
) -> impl IntoResponse {
    if let Err(e) = new_config.validate() {
        return ApiResponse::<()>::error(&format!("Validation failed: {e}")).into_response();
    }

    if let Err(e) = new_config.save() {
        return ApiResponse::<()>::error(&format!("Failed to save config: {e}")).into_response();
    }

    // Rebuild service with new config
    let bins = state.bin_paths.read().await.clone();
    let new_service = crate::service::Service::from_config_with_bins(&new_config, &bins);

    let mut config = state.config.write().await;
    *config = new_config;
    let mut service = state.service.write().await;
    *service = new_service;
    state.config_updated.store(true, std::sync::atomic::Ordering::SeqCst);

    tracing::info!("🔄 Configuration updated and service rebuilt");
    ApiResponse::<()>::ok().into_response()
}
