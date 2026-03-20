use crate::util::cmd;
use std::path::Path;

/// Get audio duration in seconds via ffprobe
pub async fn get_duration(ffprobe: &str, input: &Path) -> anyhow::Result<f64> {
    let input_str = input.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", input.display()))?;

    let stdout = cmd::run_cmd(ffprobe, &[
        "-i", input_str,
        "-show_entries", "format=duration",
        "-v", "quiet",
        "-of", "csv=p=0",
    ]).await?;

    let s = String::from_utf8_lossy(&stdout).trim().to_string();
    s.parse::<f64>()
        .map_err(|_| anyhow::anyhow!("Failed to parse duration from ffprobe: '{s}'"))
}

/// Convert audio to mono 16kHz for ASR compatibility
pub async fn process_audio(ffmpeg: &str, input: &Path) -> anyhow::Result<String> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("audio");
    let parent = input.parent().unwrap_or(Path::new("."));
    let output = parent.join(format!("{stem}_mono_16K.mp3"));

    let input_str = input.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", input.display()))?;
    let output_str = output.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", output.display()))?;

    cmd::run_cmd_status(ffmpeg, &[
        "-y", "-i", input_str,
        "-ac", "1", "-ar", "16000", "-b:a", "192k",
        output_str,
    ]).await?;

    Ok(output.to_string_lossy().to_string())
}
