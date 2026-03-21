use crate::storage::BinPaths;
use crate::types::task::{self, EmbedVideoType, StepParam};
use crate::util::cmd;
use std::path::Path;

/// Step 1: Download or locate the source video/audio
/// - For YouTube/Bilibili URLs: use yt-dlp with best practices
/// - For local files: extract audio via ffmpeg
pub async fn link_to_file(bins: &BinPaths, param: &mut StepParam, proxy: &str) -> anyhow::Result<()> {
    let audio_path = format!("{}/{}", param.task_base_path, task::AUDIO_FILE_NAME);
    let video_path = format!("{}/{}", param.task_base_path, task::VIDEO_FILE_NAME);

    if param.link.starts_with("local:") {
        // Local file: extract audio via ffmpeg
        let local_path = param.link.trim_start_matches("local:");
        tracing::info!("   📂 Local file: {local_path}");

        cmd::run_cmd_status(
            &bins.ffmpeg,
            &[
                "-y", "-i", local_path,
                "-vn", "-ar", "44100", "-ac", "2", "-ab", "192k",
                "-f", "mp3", &audio_path,
            ],
        )
        .await?;

        // If we need video embedding, copy/use the original video
        if param.embed_subtitle_video_type != EmbedVideoType::None || param.enable_tts {
            tokio::fs::copy(local_path, &video_path).await?;
            param.input_video_path = video_path;
        }
    } else {
        // YouTube / Bilibili / other URL: download via yt-dlp
        tracing::info!("   🌐 URL: {}", param.link);

        // First, try to get metadata (title, language, available subs)
        let metadata = fetch_metadata(&bins.ytdlp, &param.link, proxy).await;
        if let Ok(ref meta) = metadata {
            if let Some(title) = meta["title"].as_str() {
                tracing::info!("   📹 Title: {title}");
            }
            if let Some(lang) = meta["language"].as_str() {
                tracing::info!("   🌍 Video language: {lang}");
                // Use detected language if origin_language is "auto"
                if param.origin_language == "auto" && !lang.is_empty() {
                    // Normalize "en-US" → "en", "zh-CN" → "zh_cn", etc.
                    let normalized = lang.split('-').next().unwrap_or(lang).to_lowercase();
                    param.origin_language = normalized;
                }
            }
        }

        // Download audio with best quality
        let mut args = vec![
            "-f", "bestaudio[ext=m4a]/bestaudio[ext=mp3]/bestaudio",
            "-x", "--audio-format", "mp3",
            "--no-playlist",
            "--restrict-filenames",
            "-o", &audio_path,
        ];

        let proxy_owned;
        if !proxy.is_empty() {
            proxy_owned = proxy.to_string();
            args.push("--proxy");
            args.push(&proxy_owned);
        }

        args.push(&param.link);
        cmd::run_cmd_status(&bins.ytdlp, &args).await?;

        // Download existing subtitles if available
        download_subtitles(&bins.ytdlp, &param.link, &param.task_base_path, proxy).await;

        // If we need video, download with best MP4 quality
        if param.embed_subtitle_video_type != EmbedVideoType::None || param.enable_tts {
            tracing::info!("   📥 Downloading video for embedding/TTS...");

            let mut video_args = vec![
                "-f", "bv*[ext=mp4]+ba[ext=m4a]/bv*+ba/b",
                "--merge-output-format", "mp4",
                "--no-playlist",
                "--restrict-filenames",
                "--newline",
                "-o", &video_path,
            ];

            let proxy_owned2;
            if !proxy.is_empty() {
                proxy_owned2 = proxy.to_string();
                video_args.push("--proxy");
                video_args.push(&proxy_owned2);
            }

            video_args.push(&param.link);

            let result = cmd::run_cmd_with_progress(
                &bins.ytdlp,
                &video_args,
                |line| {
                    if let Some(pct) = parse_ytdlp_progress(line) {
                        let filled = (pct as usize * 20) / 100;
                        let empty = 20 - filled;
                        let bar = format!("{}{}",
                            "█".repeat(filled),
                            "░".repeat(empty),
                        );
                        tracing::info!("   📥 Downloading: {pct:5.1}% {bar}");
                    }
                },
            ).await;

            match result {
                Ok(()) => {
                    param.input_video_path = video_path;
                }
                Err(e) => {
                    tracing::warn!("   ⚠️  Video download failed (continuing with audio only): {e}");
                }
            }
        }
    }

    // Verify audio file exists
    if !Path::new(&audio_path).exists() {
        anyhow::bail!("Audio file was not created: {audio_path}");
    }

    param.audio_file_path = audio_path;
    tracing::info!("   📁 Audio: {}", param.audio_file_path);
    Ok(())
}

/// Fetch video metadata via yt-dlp --dump-json
async fn fetch_metadata(
    ytdlp: &str,
    url: &str,
    proxy: &str,
) -> anyhow::Result<serde_json::Value> {
    let mut args = vec!["--dump-json", "--no-download", "--no-playlist"];

    let proxy_owned;
    if !proxy.is_empty() {
        proxy_owned = proxy.to_string();
        args.push("--proxy");
        args.push(&proxy_owned);
    }

    args.push(url);
    let stdout = cmd::run_cmd(ytdlp, &args).await?;
    let metadata: serde_json::Value = serde_json::from_slice(&stdout)?;
    Ok(metadata)
}

/// Download existing subtitles from the video source (if any)
async fn download_subtitles(
    ytdlp: &str,
    url: &str,
    task_dir: &str,
    proxy: &str,
) {
    let out_template = format!("{task_dir}/yt_subs");
    let mut args = vec![
        "--write-subs",
        "--write-auto-subs",
        "--sub-langs", "en,ru,zh",
        "--sub-format", "srt/vtt/best",
        "--convert-subs", "srt",
        "--skip-download",
        "--no-playlist",
        "-o", &out_template,
    ];

    let proxy_owned;
    if !proxy.is_empty() {
        proxy_owned = proxy.to_string();
        args.push("--proxy");
        args.push(&proxy_owned);
    }

    args.push(url);

    match cmd::run_cmd_status(ytdlp, &args).await {
        Ok(()) => {
            tracing::info!("   📝 Downloaded existing subtitles from source");
        }
        Err(_) => {
            tracing::debug!("   ℹ️  No subtitles available from source");
        }
    }
}

/// Parse yt-dlp progress line like `[download]  45.2% of 50.00MiB at 2.50MiB/s ETA 00:11`
fn parse_ytdlp_progress(line: &str) -> Option<f32> {
    let line = line.trim();
    if !line.starts_with("[download]") {
        return None;
    }
    let rest = line.strip_prefix("[download]")?.trim();
    if !rest.contains('%') {
        return None;
    }
    let pct_str = rest.split('%').next()?.trim();
    pct_str.parse::<f32>().ok()
}
