use crate::provider::Transcriber;
use crate::types::subtitle::{TranscriptionData, Word, parse_timestamp};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;

pub struct WhisperCppProcessor {
    pub bin_path: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
struct WcppOutput {
    transcription: Option<Vec<WcppTranscription>>,
}

#[derive(Debug, Deserialize)]
struct WcppTranscription {
    text: Option<String>,
    tokens: Option<Vec<WcppToken>>,
}

#[derive(Debug, Deserialize)]
struct WcppToken {
    text: Option<String>,
    timestamps: Option<WcppTimestamps>,
    #[allow(dead_code)]
    id: Option<i64>,
    #[allow(dead_code)]
    p: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct WcppTimestamps {
    from: Option<String>,
    to: Option<String>,
}

impl WhisperCppProcessor {
    pub fn new(bin_path: &str, model: &str) -> Self {
        Self {
            bin_path: bin_path.to_string(),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl Transcriber for WhisperCppProcessor {
    async fn transcription(
        &self,
        audio_file: &Path,
        language: &str,
        work_dir: &Path,
    ) -> anyhow::Result<TranscriptionData> {
        let output_name = audio_file
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let output_path = work_dir.join(output_name);

        let model_path = format!("./models/whispercpp/ggml-{}.bin", self.model);

        let mut args = vec![
            "-m".to_string(),
            model_path,
            "--output-json-full".to_string(),
            "--flash-attn".to_string(),
            "--split-on-word".to_string(),
            "--output-file".to_string(),
            output_path.to_str().unwrap().to_string(),
            "--file".to_string(),
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
            anyhow::bail!("whisper.cpp failed: {stderr}");
        }

        let json_file = format!("{}.json", output_path.display());
        let json_content = tokio::fs::read_to_string(&json_file).await?;
        let wcpp: WcppOutput = serde_json::from_str(&json_content)?;

        let mut words = Vec::new();
        let mut full_text = String::new();

        if let Some(transcriptions) = &wcpp.transcription {
            for t in transcriptions {
                if let Some(text) = &t.text {
                    full_text.push_str(text);
                }
                if let Some(tokens) = &t.tokens {
                    for token in tokens {
                        let text = token
                            .text
                            .as_deref()
                            .unwrap_or("")
                            .replace("--", " ")
                            .trim()
                            .to_string();
                        if text.is_empty() {
                            continue;
                        }

                        let (start, end) = if let Some(ts) = &token.timestamps {
                            let s = ts
                                .from
                                .as_deref()
                                .and_then(parse_timestamp)
                                .unwrap_or(0.0);
                            let e = ts
                                .to
                                .as_deref()
                                .and_then(parse_timestamp)
                                .unwrap_or(s);
                            (s, e)
                        } else {
                            (0.0, 0.0)
                        };

                        words.push(Word {
                            num: words.len(),
                            text,
                            start,
                            end,
                        });
                    }
                }
            }
        }

        Ok(TranscriptionData {
            language: String::new(),
            text: full_text,
            words,
        })
    }
}
