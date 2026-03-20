use crate::provider::Transcriber;
use crate::types::subtitle::{TranscriptionData, Word};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;

pub struct AliyunAsrClient {
    pub access_key_id: String,
    pub access_key_secret: String,
    pub app_key: String,
    pub oss_bucket: String,
    pub oss_ak_id: String,
    pub oss_ak_secret: String,
}

#[derive(Debug, Deserialize)]
struct AsrTaskResult {
    #[serde(rename = "StatusText")]
    status_text: Option<String>,
    #[serde(rename = "Result")]
    result: Option<AsrResult>,
}

#[derive(Debug, Deserialize)]
struct AsrResult {
    #[serde(rename = "Sentences")]
    sentences: Option<Vec<AsrSentence>>,
    #[serde(rename = "Words")]
    words: Option<Vec<AsrWord>>,
}

#[derive(Debug, Deserialize)]
struct AsrSentence {
    #[serde(rename = "Text")]
    text: String,
    #[serde(rename = "BeginTime")]
    begin_time: f64,
    #[serde(rename = "EndTime")]
    end_time: f64,
}

#[derive(Debug, Deserialize)]
struct AsrWord {
    #[serde(rename = "Word")]
    word: Option<String>,
    #[serde(rename = "BeginTime")]
    begin_time: Option<f64>,
    #[serde(rename = "EndTime")]
    end_time: Option<f64>,
}

impl AliyunAsrClient {
    pub fn new(
        access_key_id: &str,
        access_key_secret: &str,
        app_key: &str,
        oss_bucket: &str,
        oss_ak_id: &str,
        oss_ak_secret: &str,
    ) -> Self {
        Self {
            access_key_id: access_key_id.to_string(),
            access_key_secret: access_key_secret.to_string(),
            app_key: app_key.to_string(),
            oss_bucket: oss_bucket.to_string(),
            oss_ak_id: oss_ak_id.to_string(),
            oss_ak_secret: oss_ak_secret.to_string(),
        }
    }
}

#[async_trait]
impl Transcriber for AliyunAsrClient {
    async fn transcription(
        &self,
        audio_file: &Path,
        _language: &str,
        work_dir: &Path,
    ) -> anyhow::Result<TranscriptionData> {
        // Step 1: Convert audio to mono 16kHz
        let mono = crate::util::audio::process_audio("ffmpeg", audio_file).await?;

        // Step 2: Upload to OSS
        let file_key = format!(
            "krillin/{}",
            audio_file.file_name().unwrap().to_str().unwrap()
        );
        crate::provider::aliyun::oss::upload_file(
            &self.oss_ak_id,
            &self.oss_ak_secret,
            &self.oss_bucket,
            &file_key,
            &mono,
        )
        .await?;

        let file_link = format!(
            "https://{}.oss-cn-shanghai.aliyuncs.com/{file_key}",
            self.oss_bucket
        );

        // Step 3: Submit ASR task
        let token = super::create_token(&self.access_key_id, &self.access_key_secret).await?;
        let task_id = submit_asr_task(&self.app_key, &file_link, &token).await?;

        // Step 4: Poll for result
        let result = poll_asr_result(&task_id, &token).await?;

        // Step 5: Parse result
        let mut words = Vec::new();
        let mut full_text = String::new();

        if let Some(res) = &result.result {
            if let Some(sentences) = &res.sentences {
                for s in sentences {
                    if !full_text.is_empty() {
                        full_text.push(' ');
                    }
                    full_text.push_str(&s.text);
                }
            }
            if let Some(w_list) = &res.words {
                for w in w_list {
                    if let Some(text) = &w.word {
                        let text = text.trim().to_string();
                        if !text.is_empty() {
                            words.push(Word {
                                num: words.len(),
                                text,
                                start: w.begin_time.unwrap_or(0.0) / 1000.0,
                                end: w.end_time.unwrap_or(0.0) / 1000.0,
                            });
                        }
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

async fn submit_asr_task(app_key: &str, file_link: &str, _token: &str) -> anyhow::Result<String> {
    // Simplified — in production this would call Aliyun's filetrans API
    let body = serde_json::json!({
        "appkey": app_key,
        "file_link": file_link,
        "version": "4.0",
        "enable_words": true,
    });

    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://filetrans.cn-shanghai.aliyuncs.com")
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    resp["TaskId"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Failed to get TaskId: {resp}"))
}

async fn poll_asr_result(task_id: &str, _token: &str) -> anyhow::Result<AsrTaskResult> {
    let client = reqwest::Client::new();
    let poll_interval = std::time::Duration::from_secs(10);
    let max_polls = 60; // 10 minutes

    for _ in 0..max_polls {
        tokio::time::sleep(poll_interval).await;

        let resp: serde_json::Value = client
            .get("https://filetrans.cn-shanghai.aliyuncs.com")
            .query(&[("TaskId", task_id)])
            .send()
            .await?
            .json()
            .await?;

        let status = resp["StatusText"].as_str().unwrap_or("");
        match status {
            "SUCCESS" => {
                let result: AsrTaskResult = serde_json::from_value(resp)?;
                return Ok(result);
            }
            "FAILED" => {
                anyhow::bail!("Aliyun ASR task failed");
            }
            _ => {
                tracing::debug!("ASR task status: {status}, waiting...");
            }
        }
    }

    anyhow::bail!("Aliyun ASR polling timeout")
}
