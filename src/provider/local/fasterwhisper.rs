use crate::provider::Transcriber;
use crate::types::subtitle::{TranscriptionData, Word};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;

pub struct FasterWhisperProcessor {
    pub bin_path: String,
    pub model: String,
    pub gpu: bool,
}

#[derive(Debug, Deserialize)]
struct FwOutput {
    segments: Vec<FwSegment>,
    language: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FwSegment {
    text: Option<String>,
    words: Option<Vec<FwWord>>,
}

#[derive(Debug, Deserialize)]
struct FwWord {
    start: f64,
    end: f64,
    word: String,
}

impl FasterWhisperProcessor {
    pub fn new(bin_path: &str, model: &str, gpu: bool) -> Self {
        Self {
            bin_path: bin_path.to_string(),
            model: model.to_string(),
            gpu,
        }
    }
}

#[async_trait]
impl Transcriber for FasterWhisperProcessor {
    async fn transcription(
        &self,
        audio_file: &Path,
        language: &str,
        work_dir: &Path,
    ) -> anyhow::Result<TranscriptionData> {
        let mut args = vec![
            "--model_dir".to_string(),
            "./models/".to_string(),
            "--model".to_string(),
            self.model.clone(),
            "--one_word".to_string(),
            "2".to_string(),
            "--output_format".to_string(),
            "json".to_string(),
            "--output_dir".to_string(),
            work_dir.to_str().unwrap().to_string(),
        ];

        if !language.is_empty() {
            args.push("--language".to_string());
            args.push(language.to_string());
        }

        if self.gpu {
            args.push("--compute_type".to_string());
            args.push("float16".to_string());
        }

        args.push(audio_file.to_str().unwrap().to_string());

        let output = tokio::process::Command::new(&self.bin_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("FasterWhisper failed: {stderr}");
        }

        // Parse output JSON
        let json_file = format!(
            "{}/{}.json",
            work_dir.display(),
            audio_file.file_stem().unwrap().to_str().unwrap()
        );
        let json_content = tokio::fs::read_to_string(&json_file).await?;
        let fw: FwOutput = serde_json::from_str(&json_content)?;

        let mut words = Vec::new();
        let mut full_text = String::new();

        for segment in &fw.segments {
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
            language: fw.language.unwrap_or_default(),
            text: fw.text.unwrap_or(full_text),
            words,
        })
    }
}
