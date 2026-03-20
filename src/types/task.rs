use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TaskStatus {
    Processing = 1,
    Success = 2,
    Failed = 3,
}

// Subtitle result type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SubtitleResultType {
    OriginOnly = 1,
    TargetOnly = 2,
    BilingualTranslationOnTop = 3,
    BilingualTranslationOnBottom = 4,
}

// Embed subtitle video type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum EmbedVideoType {
    #[default]
    None,
    Horizontal,
    Vertical,
    All,
}


impl From<&str> for EmbedVideoType {
    fn from(s: &str) -> Self {
        match s {
            "horizontal" => Self::Horizontal,
            "vertical" => Self::Vertical,
            "all" => Self::All,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleInfo {
    pub name: String,
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTask {
    pub task_id: String,
    pub title: String,
    pub description: String,
    pub translated_title: String,
    pub translated_description: String,
    pub origin_language: String,
    pub target_language: String,
    pub video_src: String,
    pub status: TaskStatus,
    pub fail_reason: String,
    pub process_pct: u8,
    pub subtitle_infos: Vec<SubtitleInfo>,
    pub speech_download_url: String,
    pub create_time: i64,
    pub update_time: i64,
}

impl SubtitleTask {
    pub fn new(task_id: String, video_src: String, origin_lang: String, target_lang: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            task_id,
            title: String::new(),
            description: String::new(),
            translated_title: String::new(),
            translated_description: String::new(),
            origin_language: origin_lang,
            target_language: target_lang,
            video_src,
            status: TaskStatus::Processing,
            fail_reason: String::new(),
            process_pct: 0,
            subtitle_infos: Vec::new(),
            speech_download_url: String::new(),
            create_time: now,
            update_time: now,
        }
    }

    pub fn set_failed(&mut self, reason: String) {
        self.status = TaskStatus::Failed;
        self.fail_reason = reason;
        self.update_time = chrono::Utc::now().timestamp();
    }

    pub fn set_progress(&mut self, pct: u8) {
        self.process_pct = pct.min(100);
        self.update_time = chrono::Utc::now().timestamp();
    }

    pub fn set_success(&mut self) {
        self.status = TaskStatus::Success;
        self.process_pct = 100;
        self.update_time = chrono::Utc::now().timestamp();
    }
}

/// Parameters passed through the pipeline steps
pub struct StepParam {
    pub task_id: String,
    pub task_base_path: String,
    pub link: String,
    pub audio_file_path: String,
    pub input_video_path: String,
    pub video_with_tts_file_path: String,
    pub subtitle_result_type: SubtitleResultType,
    pub enable_modal_filter: bool,
    pub enable_tts: bool,
    pub tts_voice_code: String,
    pub voice_clone_audio_url: String,
    pub origin_language: String,
    pub target_language: String,
    pub user_ui_language: String,
    pub replace_words_map: HashMap<String, String>,
    pub bilingual_srt_file_path: String,
    pub short_origin_mixed_srt_file_path: String,
    pub tts_source_file_path: String,
    pub tts_result_file_path: String,
    pub embed_subtitle_video_type: EmbedVideoType,
    pub vertical_video_major_title: String,
    pub vertical_video_minor_title: String,
    pub max_word_one_line: usize,
    pub subtitle_infos: Vec<SubtitleInfo>,
    /// Whether to add dubbed audio as a second track (true) or replace original (false)
    pub multi_track_audio: bool,
    /// Detected language from Whisper (filled during transcription)
    pub detected_language: String,
}

impl StepParam {
    pub fn output_dir(&self) -> String {
        format!("{}/output", self.task_base_path)
    }
}

// File name constants
pub const AUDIO_FILE_NAME: &str = "origin_audio.mp3";
pub const VIDEO_FILE_NAME: &str = "origin_video.mp4";
pub const SPLIT_AUDIO_PATTERN: &str = "split_audio_{:03}.mp3";
pub const BILINGUAL_SRT_FILE: &str = "bilingual_srt.srt";
pub const ORIGIN_LANG_SRT_FILE: &str = "origin_language_srt.srt";
pub const TARGET_LANG_SRT_FILE: &str = "target_language_srt.srt";
pub const SHORT_ORIGIN_MIXED_SRT_FILE: &str = "short_origin_mixed_srt.srt";
pub const SHORT_ORIGIN_SRT_FILE: &str = "short_origin_srt.srt";
pub const TTS_FINAL_AUDIO: &str = "tts_final_audio.wav";
pub const VIDEO_WITH_TTS: &str = "video_with_tts.mp4";
pub const HORIZONTAL_EMBED: &str = "horizontal_embed.mp4";
pub const VERTICAL_EMBED: &str = "vertical_embed.mp4";
