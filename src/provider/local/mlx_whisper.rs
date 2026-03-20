use crate::provider::Transcriber;
use crate::types::subtitle::{TranscriptionData, Word};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;

/// MLX Whisper processor — runs natively on Apple Silicon via Metal GPU.
/// Uses the same JSON output format as FasterWhisper, so parsing is shared.
#[cfg(target_os = "macos")]
pub struct MlxWhisperProcessor {
    pub model: String,
}

#[derive(Debug, Deserialize)]
struct MlxOutput {
    segments: Vec<MlxSegment>,
    language: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MlxSegment {
    text: Option<String>,
    words: Option<Vec<MlxWord>>,
}

#[derive(Debug, Deserialize)]
struct MlxWord {
    start: f64,
    end: f64,
    word: String,
}

#[cfg(target_os = "macos")]
impl MlxWhisperProcessor {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
        }
    }
}

#[cfg(target_os = "macos")]
#[async_trait]
impl Transcriber for MlxWhisperProcessor {
    async fn transcription(
        &self,
        audio_file: &Path,
        language: &str,
        work_dir: &Path,
    ) -> anyhow::Result<TranscriptionData> {
        let audio_str = audio_file
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", audio_file.display()))?;
        let work_str = work_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", work_dir.display()))?;

        let mut args = vec![
            audio_str.to_string(),
            "--model".to_string(),
            self.model.clone(),
            "-f".to_string(),
            "json".to_string(),
            "--output-dir".to_string(),
            work_str.to_string(),
            "--word-timestamps".to_string(),
            "True".to_string(),
        ];

        // Omit --language to enable auto-detection
        if !language.is_empty() && language != "auto" {
            args.push("--language".to_string());
            args.push(language.to_string());
        }

        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        crate::util::cmd::run_cmd_status("mlx_whisper", &args_ref).await?;

        // Parse output JSON — same format as FasterWhisper
        let stem = audio_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("audio");
        let json_file = format!("{work_str}/{stem}.json");
        let json_content = tokio::fs::read_to_string(&json_file).await?;
        let mlx: MlxOutput = serde_json::from_str(&json_content)?;

        let mut words = Vec::new();
        let mut full_text = String::new();

        for segment in &mlx.segments {
            if let Some(text) = &segment.text {
                full_text.push_str(text);
            }
            if let Some(w_list) = &segment.words {
                for w in w_list {
                    let text = w.word.replace("--", " ").trim().to_string();
                    if !text.is_empty() {
                        words.push(Word {
                            num: words.len(),
                            text,
                            start: w.start,
                            end: w.end,
                        });
                    }
                }
            }
        }

        Ok(TranscriptionData {
            language: mlx.language.unwrap_or_default(),
            text: mlx.text.unwrap_or(full_text),
            words,
        })
    }
}
