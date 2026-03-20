use clap::Parser;
use vdub::config::{Config, TtsProvider};
use vdub::service::Service;
use vdub::storage::BinPaths;
use vdub::types::task::{EmbedVideoType, StepParam, SubtitleResultType};
use vdub::util::cli_art;
use tracing_subscriber::EnvFilter;

/// vdub — dub any video from the command line
#[derive(Parser)]
#[command(name = "vdub")]
struct Cli {
    /// YouTube URL or local file path
    url: String,

    /// Source language (auto-detected if omitted)
    #[arg(long, short = 'f')]
    from: Option<String>,

    /// Target language (auto-selected EN↔RU if omitted)
    #[arg(long, short = 't')]
    to: Option<String>,

    /// Disable TTS dubbing (subtitles only)
    #[arg(long)]
    no_tts: bool,

    /// Disable subtitle embedding into video
    #[arg(long)]
    no_embed: bool,

    /// TTS voice code
    #[arg(long)]
    voice: Option<String>,

    /// Disable bilingual subtitles
    #[arg(long)]
    no_bilingual: bool,

    /// Replace original audio instead of adding second track
    #[arg(long)]
    replace_audio: bool,

    /// Also generate vertical video
    #[arg(long)]
    vertical: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    cli_art::print_skull();

    let config = Config::load()?;
    let venv_bin = vdub::util::deps::ensure_dependencies(&config).await?;
    let bins = BinPaths::detect_with_venv(venv_bin.as_deref());

    for w in &bins.validate() {
        tracing::warn!("{w}");
    }

    let service = Service::from_config_with_bins(&config, &bins);

    // Build params from CLI flags
    let origin_language = match &cli.from {
        Some(lang) => lang.clone(),
        None if config.app.auto_detect_language => "auto".to_string(),
        None => "en".to_string(),
    };

    let target_language = match &cli.to {
        Some(lang) => lang.clone(),
        None if config.app.auto_detect_language => "auto".to_string(),
        None => config.app.default_target_language.clone(),
    };

    let subtitle_result_type = if cli.no_bilingual {
        SubtitleResultType::TargetOnly
    } else {
        SubtitleResultType::BilingualTranslationOnBottom
    };

    let embed_type = if cli.no_embed {
        EmbedVideoType::None
    } else if cli.vertical {
        EmbedVideoType::All
    } else {
        EmbedVideoType::Horizontal
    };

    let default_voice = match config.tts.provider {
        TtsProvider::EdgeTts => "en-US-AriaNeural",
        TtsProvider::MlxAudio => "af_heart",
    };
    let voice = cli.voice.as_deref().unwrap_or(default_voice);

    let task_id = format!("vdub_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let task_base_path = format!("./tasks/{task_id}");
    tokio::fs::create_dir_all(format!("{task_base_path}/output")).await?;

    let mut param = StepParam {
        task_id: task_id.clone(),
        task_base_path,
        link: cli.url,
        audio_file_path: String::new(),
        input_video_path: String::new(),
        video_with_tts_file_path: String::new(),
        subtitle_result_type,
        enable_modal_filter: false,
        enable_tts: !cli.no_tts,
        tts_voice_code: voice.to_string(),
        voice_clone_audio_url: String::new(),
        origin_language,
        target_language,
        user_ui_language: String::new(),
        replace_words_map: Default::default(),
        bilingual_srt_file_path: String::new(),
        short_origin_mixed_srt_file_path: String::new(),
        tts_source_file_path: String::new(),
        tts_result_file_path: String::new(),
        embed_subtitle_video_type: embed_type,
        vertical_video_major_title: String::new(),
        vertical_video_minor_title: String::new(),
        max_word_one_line: 12,
        subtitle_infos: Vec::new(),
        multi_track_audio: !cli.replace_audio,
        detected_language: String::new(),
    };

    // Run pipeline
    cli_art::step_download_start(&param.link);
    vdub::service::link_to_file::link_to_file(&bins, &mut param, &config.app.proxy).await?;
    cli_art::step_download_done();

    cli_art::step_transcribe_start(config.transcribe.provider.as_str(), &param.origin_language);
    vdub::service::audio_to_subtitle::audio_to_subtitle(
        &bins, &config, &service.transcriber, &service.chat_completer, &mut param,
    ).await?;

    if param.enable_tts {
        cli_art::step_tts_start(config.tts.provider.as_str(), &param.tts_voice_code);
        vdub::service::srt_to_speech::srt_to_speech(
            &bins, &config, &service.tts_client, &mut param,
        ).await?;
    }

    vdub::service::srt_embed::embed_subtitles(&bins, &mut param).await?;

    cli_art::step_finalize_start();
    vdub::service::upload_subtitles::upload_subtitles(&mut param).await?;
    cli_art::step_finalize_done(param.subtitle_infos.len());
    cli_art::pipeline_success(&task_id);

    println!("\n📁 Output: {}", param.output_dir());
    for info in &param.subtitle_infos {
        println!("   {} → {}", info.name, info.download_url);
    }

    Ok(())
}
