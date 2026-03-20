use crate::provider::Transcriber;
use crate::types::subtitle::{TranscriptionData, Word};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;

pub struct WhisperKitProcessor {
    pub bin_path: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
struct WkOutput {
    segments: Option<Vec<WkSegment>>,
    language: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WkSegment {
    text: Option<String>,
    words: Option<Vec<WkWord>>,
}

#[derive(Debug, Deserialize)]
struct WkWord {
    start: f64,
    end: f64,
    word: String,
}

impl WhisperKitProcessor {
    pub fn new(bin_path: &str, model: &str) -> Self {
        Self {
            bin_path: bin_path.to_string(),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl Transcriber for WhisperKitProcessor {
    async fn transcription(
        &self,
        audio_file: &Path,
        language: &str,
        work_dir: &Path,
    ) -> anyhow::Result<TranscriptionData> {
        let model_path = format!("./models/whisperkit/openai_whisper-{}", self.model);

        let mut args = vec![
            "transcribe".to_string(),
            "--model-path".to_string(),
            model_path,
            "--audio-encoder-compute-units".to_string(),
            "all".to_string(),
            "--text-decoder-compute-units".to_string(),
            "all".to_string(),
            "--report".to_string(),
            "--report-path".to_string(),
            work_dir.to_str().unwrap().to_string(),
            "--word-timestamps".to_string(),
            "--skip-special-tokens".to_string(),
            "--audio-path".to_string(),
            audio_file.to_str().unwrap().to_string(),
        ];

        if !language.is_empty() {
            args.push("--language".to_string());
            args.push(language.to_string());
        }

        let output = tokio::process::Command::new(&self.bin_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("WhisperKit failed: {stderr}");
        }

        // Find the JSON output file
        let json_file = format!(
            "{}/{}.json",
            work_dir.display(),
            audio_file.file_stem().unwrap().to_str().unwrap()
        );
        let json_content = tokio::fs::read_to_string(&json_file).await?;
        let wk: WkOutput = serde_json::from_str(&json_content)?;

        let mut words = Vec::new();
        let mut full_text = String::new();

        if let Some(segments) = &wk.segments {
            for seg in segments {
                if let Some(text) = &seg.text {
                    full_text.push_str(text);
                }
                if let Some(w_list) = &seg.words {
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
        }

        Ok(TranscriptionData {
            language: wk.language.unwrap_or_default(),
            text: wk.text.unwrap_or(full_text),
            words,
        })
    }
}
