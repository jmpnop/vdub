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
                "-o", &video_path,
            ];

            let proxy_owned2;
            if !proxy.is_empty() {
                proxy_owned2 = proxy.to_string();
                video_args.push("--proxy");
                video_args.push(&proxy_owned2);
            }

            video_args.push(&param.link);

            match cmd::run_cmd_status(&bins.ytdlp, &video_args).await {
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
