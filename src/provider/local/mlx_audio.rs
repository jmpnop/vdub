use crate::provider::Ttser;
use async_trait::async_trait;
use std::path::Path;

/// MLX Audio TTS client — runs Kokoro on Apple Silicon via Metal GPU.
/// Supports: English, Japanese, Mandarin, Spanish, French.
/// ~82M parameters, very fast inference on M-series chips.
#[cfg(target_os = "macos")]
pub struct MlxAudioClient {
    pub model: String,
    pub default_voice: String,
}

#[cfg(target_os = "macos")]
impl MlxAudioClient {
    pub fn new(model: &str, voice: &str) -> Self {
        Self {
            model: model.to_string(),
            default_voice: voice.to_string(),
        }
    }
}

#[cfg(target_os = "macos")]
#[async_trait]
impl Ttser for MlxAudioClient {
    async fn text_to_speech(
        &self,
        text: &str,
        voice: &str,
        output_file: &Path,
    ) -> anyhow::Result<()> {
        let voice = if voice.is_empty() {
            &self.default_voice
        } else {
            voice
        };

        let output_str = output_file
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", output_file.display()))?;

        // Write text to temp file to avoid shell escaping issues
        let temp_dir = output_file.parent().unwrap_or(Path::new("."));
        let temp_file = temp_dir.join("mlx_audio_input.txt");
        tokio::fs::write(&temp_file, text).await?;

        let temp_str = temp_file
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", temp_file.display()))?;

        let result = crate::util::cmd::run_cmd_status(
            "python3",
            &[
                "-m",
                "mlx_audio.tts.generate",
                "--model",
                &self.model,
                "--text-file",
                temp_str,
                "--voice",
                voice,
                "--output",
                output_str,
            ],
        )
        .await;

        let _ = tokio::fs::remove_file(&temp_file).await;
        result
    }
}
