use crate::provider::Ttser;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tokio_tungstenite::tungstenite::Message;

pub struct AliyunTtsClient {
    pub access_key_id: String,
    pub access_key_secret: String,
    pub app_key: String,
}

impl AliyunTtsClient {
    pub fn new(ak_id: &str, ak_secret: &str, app_key: &str) -> Self {
        Self {
            access_key_id: ak_id.to_string(),
            access_key_secret: ak_secret.to_string(),
            app_key: app_key.to_string(),
        }
    }
}

#[async_trait]
impl Ttser for AliyunTtsClient {
    async fn text_to_speech(
        &self,
        text: &str,
        voice: &str,
        output_file: &Path,
    ) -> anyhow::Result<()> {
        // Get token
        let token =
            super::create_token(&self.access_key_id, &self.access_key_secret).await?;

        let url = format!(
            "wss://nls-gateway-cn-beijing.aliyuncs.com/ws/v1?token={token}"
        );

        let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await?;
        let (mut write, mut read) = ws_stream.split();

        let task_id = uuid::Uuid::new_v4().to_string().replace('-', "");
        let msg_id = uuid::Uuid::new_v4().to_string().replace('-', "");

        // Send StartSynthesis
        let start_msg = serde_json::json!({
            "header": {
                "appkey": self.app_key,
                "message_id": msg_id,
                "task_id": task_id,
                "namespace": "FlowingSpeechSynthesizer",
                "name": "StartSynthesis"
            },
            "payload": {
                "voice": voice,
                "format": "wav",
                "sample_rate": 44100,
                "volume": 50,
                "speech_rate": 0,
                "pitch_rate": 0,
            }
        });

        write
            .send(Message::Text(start_msg.to_string().into()))
            .await?;

        // Wait for SynthesisStarted
        wait_for_event(&mut read, "SynthesisStarted").await?;

        // Send RunSynthesis
        let run_msg = serde_json::json!({
            "header": {
                "appkey": self.app_key,
                "message_id": uuid::Uuid::new_v4().to_string().replace('-', ""),
                "task_id": task_id,
                "namespace": "FlowingSpeechSynthesizer",
                "name": "RunSynthesis"
            },
            "payload": {
                "text": text
            }
        });
        write
            .send(Message::Text(run_msg.to_string().into()))
            .await?;

        // Send StopSynthesis
        let stop_msg = serde_json::json!({
            "header": {
                "appkey": self.app_key,
                "message_id": uuid::Uuid::new_v4().to_string().replace('-', ""),
                "task_id": task_id,
                "namespace": "FlowingSpeechSynthesizer",
                "name": "StopSynthesis"
            }
        });
        write
            .send(Message::Text(stop_msg.to_string().into()))
            .await?;

        // Receive audio data until SynthesisCompleted
        let mut file = tokio::fs::File::create(output_file).await?;

        loop {
            match read.next().await {
                Some(Ok(Message::Binary(data))) => {
                    file.write_all(&data).await?;
                }
                Some(Ok(Message::Text(text))) => {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        let name = json["header"]["name"].as_str().unwrap_or("");
                        if name == "SynthesisCompleted" {
                            break;
                        }
                    }
                }
                Some(Err(e)) => {
                    anyhow::bail!("WebSocket error: {e}");
                }
                None => break,
                _ => {}
            }
        }

        file.flush().await?;
        Ok(())
    }
}

async fn wait_for_event<S>(read: &mut S, event_name: &str) -> anyhow::Result<()>
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let timeout = tokio::time::Duration::from_secs(10);
    let result = tokio::time::timeout(timeout, async {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        let name = json["header"]["name"].as_str().unwrap_or("");
                        if name == event_name {
                            return Ok(());
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => return Err(anyhow::anyhow!("WebSocket error: {e}")),
            }
        }
        Err(anyhow::anyhow!("WebSocket closed before {event_name}"))
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err(anyhow::anyhow!("Timeout waiting for {event_name}")),
    }
}
