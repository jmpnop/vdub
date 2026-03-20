use vdub::config::Config;
use vdub::router::build_router;
use vdub::service::Service;
use vdub::storage::task_store::TaskStore;
use vdub::storage::BinPaths;
use vdub::util::cli_art;
use vdub::AppState;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    cli_art::print_skull();

    let config = Config::load()?;
    let addr = format!("{}:{}", config.server.host, config.server.port);

    let venv_bin = vdub::util::deps::ensure_dependencies(&config).await?;
    let bin_paths = BinPaths::detect_with_venv(venv_bin.as_deref());

    for w in &bin_paths.validate() {
        tracing::warn!("{w}");
    }

    let service = Service::from_config_with_bins(&config, &bin_paths);

    cli_art::print_banner(&config.server.host, config.server.port);
    tracing::info!("🧠 ASR provider: {}", config.transcribe.provider.as_str());
    tracing::info!("🔊 TTS provider: {}", config.tts.provider.as_str());
    tracing::info!("💬 LLM model: {}", config.llm.model);
    if config.app.auto_detect_language {
        tracing::info!("🌍 Auto language detection: enabled (EN ↔ RU)");
    }

    let state = Arc::new(AppState {
        config: RwLock::new(config),
        task_store: TaskStore::new(),
        bin_paths: RwLock::new(bin_paths),
        service: RwLock::new(service),
        config_updated: AtomicBool::new(false),
    });

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("🚀 Server listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("👋 Shutting down...");
        })
        .await?;

    Ok(())
}
