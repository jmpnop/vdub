use krillin_rs::config::Config;
use krillin_rs::router::build_router;
use krillin_rs::service::Service;
use krillin_rs::storage::task_store::TaskStore;
use krillin_rs::storage::BinPaths;
use krillin_rs::util::cli_art;
use krillin_rs::AppState;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 🐉 Dragon on first run
    cli_art::print_dragon();

    // Load configuration
    let config = Config::load()?;
    let addr = format!("{}:{}", config.server.host, config.server.port);

    // Ensure all dependencies are installed (Homebrew + pip packages)
    let venv_bin = krillin_rs::util::deps::ensure_dependencies(&config).await?;

    // Detect external tool paths (checks venv/bin first, then ./bin/, then PATH)
    let bin_paths = BinPaths::detect_with_venv(venv_bin.as_deref());

    // Validate tools
    let warnings = bin_paths.validate();
    for w in &warnings {
        tracing::warn!("{w}");
    }

    // Initialize service from config
    let service = Service::from_config_with_bins(&config, &bin_paths);

    // Print startup banner
    cli_art::print_banner(&config.server.host, config.server.port);

    tracing::info!("🧠 ASR provider: {}", config.transcribe.provider.as_str());
    tracing::info!("🔊 TTS provider: {}", config.tts.provider.as_str());
    tracing::info!("💬 LLM model: {}", config.llm.model);
    if config.app.auto_detect_language {
        tracing::info!("🌍 Auto language detection: enabled (EN ↔ RU)");
    }

    // Build shared state
    let state = Arc::new(AppState {
        config: RwLock::new(config),
        task_store: TaskStore::new(),
        bin_paths: RwLock::new(bin_paths),
        service: RwLock::new(service),
        config_updated: AtomicBool::new(false),
    });

    // Build router and start server with graceful shutdown
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("🚀 Server listening on http://{addr}");
    tracing::info!("   Press Ctrl+C to shut down gracefully");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("👋 Shutting down gracefully...");
        })
        .await?;

    tracing::info!("✅ Server stopped");
    Ok(())
}
