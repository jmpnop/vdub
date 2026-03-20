pub mod link_to_file;
pub mod audio_to_subtitle;
pub mod srt_to_speech;
pub mod srt_embed;
pub mod upload_subtitles;
pub mod split_audio;
pub mod timestamps;

use crate::config::{Config, TranscribeProvider, TtsProvider};
use crate::provider::local::edge_tts::EdgeTtsClient;
use crate::provider::local::fasterwhisper::FasterWhisperProcessor;
use crate::provider::local::whispercpp::WhisperCppProcessor;
use crate::provider::local::whisperkit::WhisperKitProcessor;
use crate::provider::openai::OpenAiClient;
use crate::provider::{ChatCompleter, Transcriber, Ttser};
use crate::storage::BinPaths;
use std::sync::Arc;

pub struct Service {
    pub transcriber: Arc<dyn Transcriber>,
    pub chat_completer: Arc<dyn ChatCompleter>,
    pub tts_client: Arc<dyn Ttser>,
}

impl Service {
    pub fn from_config(config: &Config) -> Self {
        Self::from_config_with_bins(config, &BinPaths::detect())
    }

    pub fn from_config_with_bins(config: &Config, bins: &BinPaths) -> Self {
        let proxy = if config.app.proxy.is_empty() {
            None
        } else {
            Some(config.app.proxy.as_str())
        };

        // Transcriber — all free/local providers
        let transcriber: Arc<dyn Transcriber> = match config.transcribe.provider {
            TranscribeProvider::Fasterwhisper => Arc::new(FasterWhisperProcessor::new(
                &bins.fasterwhisper,
                &config.transcribe.fasterwhisper.model,
                config.transcribe.enable_gpu_acceleration,
            )),
            TranscribeProvider::Whispercpp => Arc::new(WhisperCppProcessor::new(
                &bins.whispercpp,
                &config.transcribe.whispercpp.model,
            )),
            TranscribeProvider::Whisperkit => Arc::new(WhisperKitProcessor::new(
                &bins.whisperkit,
                &config.transcribe.whisperkit.model,
            )),
            #[cfg(target_os = "macos")]
            TranscribeProvider::MlxWhisper => {
                Arc::new(crate::provider::local::mlx_whisper::MlxWhisperProcessor::new(
                    &config.transcribe.mlx_whisper.model,
                ))
            }
            #[cfg(not(target_os = "macos"))]
            TranscribeProvider::MlxWhisper => {
                tracing::warn!("⚠️  MLX Whisper not available on this platform, falling back to faster-whisper");
                Arc::new(FasterWhisperProcessor::new(
                    &bins.fasterwhisper,
                    &config.transcribe.fasterwhisper.model,
                    config.transcribe.enable_gpu_acceleration,
                ))
            }
        };

        // Chat completer (any OpenAI-compatible API — works with mlx_lm.server too)
        let chat_completer: Arc<dyn ChatCompleter> = Arc::new(OpenAiClient::new(
            &config.llm.base_url,
            &config.llm.api_key,
            &config.llm.model,
            proxy,
        ));

        // TTS — all free/local providers
        let tts_client: Arc<dyn Ttser> = match config.tts.provider {
            TtsProvider::EdgeTts => Arc::new(EdgeTtsClient::new(&bins.edge_tts)),
            #[cfg(target_os = "macos")]
            TtsProvider::MlxAudio => {
                Arc::new(crate::provider::local::mlx_audio::MlxAudioClient::new(
                    &config.tts.mlx_audio.model,
                    &config.tts.mlx_audio.voice,
                ))
            }
            #[cfg(not(target_os = "macos"))]
            TtsProvider::MlxAudio => {
                tracing::warn!("⚠️  MLX Audio not available on this platform, falling back to Edge TTS");
                Arc::new(EdgeTtsClient::new(&bins.edge_tts))
            }
        };

        Self {
            transcriber,
            chat_completer,
            tts_client,
        }
    }
}
