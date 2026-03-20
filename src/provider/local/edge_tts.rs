use crate::provider::Ttser;
use async_trait::async_trait;
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;

pub struct EdgeTtsClient {
    pub bin_path: String,
}

impl EdgeTtsClient {
    pub fn new(bin_path: &str) -> Self {
        Self {
            bin_path: bin_path.to_string(),
        }
    }
}

#[async_trait]
impl Ttser for EdgeTtsClient {
    async fn text_to_speech(
        &self,
        text: &str,
        voice: &str,
        output_file: &Path,
    ) -> anyhow::Result<()> {
        // Write text to temp file to avoid CLI escaping issues
        let temp_dir = output_file.parent().unwrap_or(Path::new("."));
        let temp_file = temp_dir.join("edge_tts_input.txt");
        tokio::fs::write(&temp_file, text).await?;

        let max_attempts = 3;
        for attempt in 0..max_attempts {
            let output = tokio::process::Command::new(&self.bin_path)
                .args([
                    "--text-file",
                    temp_file.to_str().unwrap(),
                    "--voice",
                    voice.trim(),
                    "--write-media",
                    output_file.to_str().unwrap(),
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;

            if output.status.success() && output_file.exists() {
                let _ = tokio::fs::remove_file(&temp_file).await;
                return Ok(());
            }

            if attempt < max_attempts - 1 {
                let wait = (attempt + 1) * 2;
                tracing::warn!(
                    "Edge TTS attempt {}/{max_attempts} failed, retrying in {wait}s",
                    attempt + 1
                );
                tokio::time::sleep(std::time::Duration::from_secs(wait as u64)).await;
            }
        }

        let _ = tokio::fs::remove_file(&temp_file).await;
        anyhow::bail!("Edge TTS failed after {max_attempts} attempts")
    }
}
