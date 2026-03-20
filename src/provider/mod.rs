pub mod openai;
pub mod local;

use crate::types::subtitle::TranscriptionData;
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait Transcriber: Send + Sync {
    async fn transcription(
        &self,
        audio_file: &Path,
        language: &str,
        work_dir: &Path,
    ) -> anyhow::Result<TranscriptionData>;
}

#[async_trait]
pub trait ChatCompleter: Send + Sync {
    async fn chat_completion(&self, query: &str) -> anyhow::Result<String>;
}

#[async_trait]
pub trait Ttser: Send + Sync {
    async fn text_to_speech(
        &self,
        text: &str,
        voice: &str,
        output_file: &Path,
    ) -> anyhow::Result<()>;
}
