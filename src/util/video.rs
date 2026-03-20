use crate::util::cmd;
use std::path::Path;

/// Replace audio track in video (single track — replaces original)
pub async fn replace_audio(
    ffmpeg: &str,
    video: &Path,
    audio: &Path,
    output: &Path,
) -> anyhow::Result<()> {
    let video_str = video.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", video.display()))?;
    let audio_str = audio.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", audio.display()))?;
    let output_str = output.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", output.display()))?;

    cmd::run_cmd_status(ffmpeg, &[
        "-y",
        "-i", video_str,
        "-i", audio_str,
        "-c:v", "copy",
        "-map", "0:v:0",
        "-map", "1:a:0",
        output_str,
    ]).await
}

/// Add dubbed audio as a second track, keeping the original audio.
/// Sets language metadata on both tracks.
pub async fn add_audio_track(
    ffmpeg: &str,
    video: &Path,
    dubbed_audio: &Path,
    output: &Path,
    original_lang: &str,
    target_lang: &str,
) -> anyhow::Result<()> {
    let video_str = video.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", video.display()))?;
    let audio_str = dubbed_audio.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", dubbed_audio.display()))?;
    let output_str = output.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", output.display()))?;

    let orig_lang_meta = format!("language={original_lang}");
    let target_lang_meta = format!("language={target_lang}");
    let orig_title = format!("title={}", crate::util::cli_art::lang_display_name(original_lang));
    let target_title = format!("title={} Dub", crate::util::cli_art::lang_display_name(target_lang));

    cmd::run_cmd_status(ffmpeg, &[
        "-y",
        "-i", video_str,
        "-i", audio_str,
        "-map", "0:v",
        "-map", "0:a",
        "-map", "1:a",
        "-c:v", "copy",
        "-c:a:0", "copy",
        "-c:a:1", "aac",
        "-b:a:1", "192k",
        "-metadata:s:a:0", &orig_lang_meta,
        "-metadata:s:a:1", &target_lang_meta,
        "-metadata:s:a:0", &orig_title,
        "-metadata:s:a:1", &target_title,
        "-disposition:a:0", "default",
        "-disposition:a:1", "0",
        output_str,
    ]).await
}

/// Get video resolution via ffprobe
pub async fn get_resolution(ffprobe: &str, video: &Path) -> anyhow::Result<(u32, u32)> {
    let video_str = video.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", video.display()))?;

    let stdout = cmd::run_cmd(ffprobe, &[
        "-v", "error",
        "-select_streams", "v:0",
        "-show_entries", "stream=width,height",
        "-of", "csv=p=0:s=x",
        video_str,
    ]).await?;

    let s = String::from_utf8_lossy(&stdout).trim().to_string();
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        anyhow::bail!("Failed to parse resolution: '{s}'");
    }
    let w: u32 = parts[0].parse()?;
    let h: u32 = parts[1].parse()?;
    Ok((w, h))
}
