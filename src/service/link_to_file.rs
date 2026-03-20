use crate::storage::BinPaths;
use crate::types::task::{self, EmbedVideoType, StepParam};
use std::path::Path;
use std::process::Stdio;

/// Step 1: Download or locate the source video/audio
/// - For YouTube/Bilibili URLs: use yt-dlp to download audio (and optionally video)
/// - For local files: extract audio via ffmpeg
pub async fn link_to_file(bins: &BinPaths, param: &mut StepParam, proxy: &str) -> anyhow::Result<()> {
    let audio_path = format!("{}/{}", param.task_base_path, task::AUDIO_FILE_NAME);
    let video_path = format!("{}/{}", param.task_base_path, task::VIDEO_FILE_NAME);

    if param.link.starts_with("local:") {
        // Local file: extract audio via ffmpeg
        let local_path = param.link.trim_start_matches("local:");
        tracing::info!("Extracting audio from local file: {local_path}");

        let status = tokio::process::Command::new(&bins.ffmpeg)
            .args([
                "-y",
                "-i", local_path,
                "-vn",
                "-ar", "44100",
                "-ac", "2",
                "-ab", "192k",
                "-f", "mp3",
                &audio_path,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("ffmpeg failed to extract audio from local file");
        }

        // If we need video embedding, copy/use the original video
        if param.embed_subtitle_video_type != EmbedVideoType::None || param.enable_tts {
            tokio::fs::copy(local_path, &video_path).await?;
            param.input_video_path = video_path;
        }
    } else {
        // YouTube / Bilibili: download via yt-dlp
        tracing::info!("Downloading audio from URL: {}", param.link);

        let mut args = vec![
            "-f".to_string(),
            "bestaudio[ext=m4a]/bestaudio[ext=mp3]/bestaudio/worst".to_string(),
            "-x".to_string(),
            "--audio-format".to_string(),
            "mp3".to_string(),
            "-o".to_string(),
            audio_path.clone(),
        ];

        if !proxy.is_empty() {
            args.push("--proxy".to_string());
            args.push(proxy.to_string());
        }

        args.push(param.link.clone());

        let output = tokio::process::Command::new(&bins.ytdlp)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("yt-dlp audio download failed: {stderr}");
        }

        // If we need video, download that too
        if param.embed_subtitle_video_type != EmbedVideoType::None || param.enable_tts {
            tracing::info!("Downloading video for embedding/TTS");

            let mut video_args = vec![
                "-f".to_string(),
                "bestvideo[height<=1080][ext=mp4]+bestaudio[ext=m4a]/best[height<=1080][ext=mp4]/best".to_string(),
                "-o".to_string(),
                video_path.clone(),
            ];

            if !proxy.is_empty() {
                video_args.push("--proxy".to_string());
                video_args.push(proxy.to_string());
            }

            video_args.push(param.link.clone());

            let video_output = tokio::process::Command::new(&bins.ytdlp)
                .args(&video_args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;

            if !video_output.status.success() {
                let stderr = String::from_utf8_lossy(&video_output.stderr);
                tracing::warn!("Video download failed (continuing with audio only): {stderr}");
            } else {
                param.input_video_path = video_path;
            }
        }
    }

    // Verify audio file exists
    if !Path::new(&audio_path).exists() {
        anyhow::bail!("Audio file was not created: {audio_path}");
    }

    param.audio_file_path = audio_path;
    tracing::info!("Step 1 complete: audio at {}", param.audio_file_path);
    Ok(())
}
