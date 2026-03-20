use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub llm: OpenaiCompatibleConfig,
    #[serde(default)]
    pub transcribe: TranscribeConfig,
    #[serde(default)]
    pub tts: TtsConfig,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Provider Enums — type-safe, exhaustive matching
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TranscribeProvider {
    #[default]
    Openai,
    Fasterwhisper,
    Whisperkit,
    Whispercpp,
    Aliyun,
    #[serde(rename = "mlx-whisper")]
    MlxWhisper,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TtsProvider {
    #[default]
    Openai,
    Aliyun,
    #[serde(rename = "edge-tts")]
    EdgeTts,
    #[serde(rename = "mlx-audio")]
    MlxAudio,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Config structs
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_segment_duration")]
    pub segment_duration: u32,
    #[serde(default = "default_transcribe_parallel")]
    pub transcribe_parallel_num: u32,
    #[serde(default = "default_translate_parallel")]
    pub translate_parallel_num: u32,
    #[serde(default = "default_tts_parallel")]
    pub tts_parallel_num: u32,
    #[serde(default = "default_max_attempts")]
    pub transcribe_max_attempts: u32,
    #[serde(default = "default_max_attempts")]
    pub translate_max_attempts: u32,
    #[serde(default = "default_max_sentence_length")]
    pub max_sentence_length: u32,
    #[serde(default)]
    pub proxy: String,
    /// When true, auto-detect source language and auto-select target (EN↔RU)
    #[serde(default = "default_true")]
    pub auto_detect_language: bool,
    /// Default target language when auto-detect is off and no target specified
    #[serde(default = "default_target_lang")]
    pub default_target_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenaiCompatibleConfig {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocalModelConfig {
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AliyunSpeechConfig {
    #[serde(default)]
    pub access_key_id: String,
    #[serde(default)]
    pub access_key_secret: String,
    #[serde(default)]
    pub app_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AliyunOssConfig {
    #[serde(default)]
    pub access_key_id: String,
    #[serde(default)]
    pub access_key_secret: String,
    #[serde(default)]
    pub bucket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AliyunTranscribeConfig {
    #[serde(default)]
    pub oss: AliyunOssConfig,
    #[serde(default)]
    pub speech: AliyunSpeechConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AliyunTtsConfig {
    #[serde(default)]
    pub oss: AliyunOssConfig,
    #[serde(default)]
    pub speech: AliyunSpeechConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlxWhisperConfig {
    #[serde(default = "default_mlx_whisper_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlxAudioConfig {
    #[serde(default = "default_mlx_audio_model")]
    pub model: String,
    #[serde(default = "default_mlx_audio_voice")]
    pub voice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribeConfig {
    #[serde(default)]
    pub provider: TranscribeProvider,
    #[serde(default)]
    pub enable_gpu_acceleration: bool,
    #[serde(default = "default_openai_transcribe")]
    pub openai: OpenaiCompatibleConfig,
    #[serde(default = "default_local_model")]
    pub fasterwhisper: LocalModelConfig,
    #[serde(default = "default_local_model")]
    pub whisperkit: LocalModelConfig,
    #[serde(default = "default_local_model")]
    pub whispercpp: LocalModelConfig,
    #[serde(default)]
    pub aliyun: AliyunTranscribeConfig,
    #[serde(default)]
    pub mlx_whisper: MlxWhisperConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    #[serde(default)]
    pub provider: TtsProvider,
    #[serde(default = "default_openai_tts")]
    pub openai: OpenaiCompatibleConfig,
    #[serde(default)]
    pub aliyun: AliyunTtsConfig,
    #[serde(default)]
    pub mlx_audio: MlxAudioConfig,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Default value functions
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn default_segment_duration() -> u32 { 5 }
fn default_transcribe_parallel() -> u32 { 1 }
fn default_translate_parallel() -> u32 { 3 }
fn default_tts_parallel() -> u32 { 3 }
fn default_max_attempts() -> u32 { 3 }
fn default_max_sentence_length() -> u32 { 70 }
fn default_host() -> String { "127.0.0.1".to_string() }
fn default_port() -> u16 { 8888 }
fn default_true() -> bool { true }
fn default_target_lang() -> String { "ru".to_string() }

fn default_openai_transcribe() -> OpenaiCompatibleConfig {
    OpenaiCompatibleConfig {
        model: "whisper-1".to_string(),
        ..Default::default()
    }
}

fn default_local_model() -> LocalModelConfig {
    LocalModelConfig {
        model: "large-v2".to_string(),
    }
}

fn default_openai_tts() -> OpenaiCompatibleConfig {
    OpenaiCompatibleConfig {
        model: "gpt-4o-mini-tts".to_string(),
        ..Default::default()
    }
}

fn default_mlx_whisper_model() -> String {
    "mlx-community/whisper-large-v3-mlx".to_string()
}
fn default_mlx_audio_model() -> String {
    "mlx-community/Kokoro-82M-bf16".to_string()
}
fn default_mlx_audio_voice() -> String {
    "af_heart".to_string()
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Default impls
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl Default for Config {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            server: ServerConfig::default(),
            llm: OpenaiCompatibleConfig {
                model: "gpt-4o-mini".to_string(),
                ..Default::default()
            },
            transcribe: TranscribeConfig::default(),
            tts: TtsConfig::default(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            segment_duration: default_segment_duration(),
            transcribe_parallel_num: default_transcribe_parallel(),
            translate_parallel_num: default_translate_parallel(),
            tts_parallel_num: default_tts_parallel(),
            transcribe_max_attempts: default_max_attempts(),
            translate_max_attempts: default_max_attempts(),
            max_sentence_length: default_max_sentence_length(),
            proxy: String::new(),
            auto_detect_language: true,
            default_target_language: default_target_lang(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

impl Default for TranscribeConfig {
    fn default() -> Self {
        Self {
            provider: TranscribeProvider::default(),
            enable_gpu_acceleration: false,
            openai: default_openai_transcribe(),
            fasterwhisper: default_local_model(),
            whisperkit: default_local_model(),
            whispercpp: default_local_model(),
            aliyun: AliyunTranscribeConfig::default(),
            mlx_whisper: MlxWhisperConfig::default(),
        }
    }
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            provider: TtsProvider::default(),
            openai: default_openai_tts(),
            aliyun: AliyunTtsConfig::default(),
            mlx_audio: MlxAudioConfig::default(),
        }
    }
}

impl Default for MlxWhisperConfig {
    fn default() -> Self {
        Self {
            model: default_mlx_whisper_model(),
        }
    }
}

impl Default for MlxAudioConfig {
    fn default() -> Self {
        Self {
            model: default_mlx_audio_model(),
            voice: default_mlx_audio_voice(),
        }
    }
}

impl TranscribeProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::Fasterwhisper => "fasterwhisper",
            Self::Whisperkit => "whisperkit",
            Self::Whispercpp => "whispercpp",
            Self::Aliyun => "aliyun",
            Self::MlxWhisper => "mlx-whisper",
        }
    }
}

impl TtsProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::Aliyun => "aliyun",
            Self::EdgeTts => "edge-tts",
            Self::MlxAudio => "mlx-audio",
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Config impl
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = "./config/config.toml";
        if Path::new(config_path).exists() {
            let content = fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&content)?;
            tracing::info!("⚙️  Configuration loaded from {}", config_path);
            Ok(config)
        } else {
            tracing::info!("⚙️  No config file found, using defaults");
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = "./config/config.toml";
        let dir = Path::new(config_path).parent().unwrap();
        fs::create_dir_all(dir)?;
        let content = toml::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        tracing::info!("💾 Configuration saved to {}", config_path);
        Ok(())
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        match self.transcribe.provider {
            TranscribeProvider::Openai => {
                if self.transcribe.openai.api_key.is_empty() && self.llm.api_key.is_empty() {
                    anyhow::bail!("OpenAI transcription requires an API key");
                }
            }
            TranscribeProvider::Fasterwhisper => {}
            TranscribeProvider::Whisperkit => {
                #[cfg(not(target_os = "macos"))]
                anyhow::bail!("WhisperKit is only supported on macOS");
            }
            TranscribeProvider::Whispercpp => {}
            TranscribeProvider::Aliyun => {
                let speech = &self.transcribe.aliyun.speech;
                if speech.access_key_id.is_empty()
                    || speech.access_key_secret.is_empty()
                    || speech.app_key.is_empty()
                {
                    anyhow::bail!("Aliyun transcription requires access_key_id, access_key_secret, and app_key");
                }
            }
            TranscribeProvider::MlxWhisper => {
                #[cfg(not(target_os = "macos"))]
                anyhow::bail!("MLX Whisper is only supported on macOS (Apple Silicon)");
            }
        }

        match self.tts.provider {
            TtsProvider::Openai => {
                if self.tts.openai.api_key.is_empty() && self.llm.api_key.is_empty() {
                    anyhow::bail!("OpenAI TTS requires an API key");
                }
            }
            TtsProvider::Aliyun => {}
            TtsProvider::EdgeTts => {}
            TtsProvider::MlxAudio => {
                #[cfg(not(target_os = "macos"))]
                anyhow::bail!("MLX Audio is only supported on macOS (Apple Silicon)");
            }
        }

        Ok(())
    }
}
