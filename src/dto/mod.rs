use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

/// API response wrapper — matches Go's response.Response
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub error: i32,
    pub msg: String,
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            error: 0,
            msg: "success".to_string(),
            data: Some(data),
        }
    }

    pub fn success_msg(msg: &str, data: T) -> Self {
        Self {
            error: 0,
            msg: msg.to_string(),
            data: Some(data),
        }
    }
}

impl ApiResponse<()> {
    pub fn error(msg: &str) -> Self {
        Self {
            error: -1,
            msg: msg.to_string(),
            data: None,
        }
    }

    pub fn ok() -> Self {
        Self {
            error: 0,
            msg: "success".to_string(),
            data: None,
        }
    }
}

/// Implement IntoResponse so handlers can return ApiResponse directly
impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

// --- Subtitle Task DTOs ---

#[derive(Debug, Deserialize)]
pub struct StartTaskRequest {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub origin_language: String,
    #[serde(default)]
    pub target_lang: String,
    #[serde(default)]
    pub bilingual: u8,
    #[serde(default)]
    pub translation_subtitle_pos: u8,
    #[serde(default)]
    pub modal_filter: u8,
    #[serde(default)]
    pub tts: u8,
    #[serde(default)]
    pub tts_voice_code: String,
    #[serde(default)]
    pub tts_voice_clone_src_file_url: String,
    #[serde(default)]
    pub replace: Vec<String>,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub embed_subtitle_video_type: String,
    #[serde(default)]
    pub vertical_major_title: String,
    #[serde(default)]
    pub vertical_minor_title: String,
    #[serde(default)]
    pub origin_language_word_one_line: usize,
    /// When true, add dubbed audio as a second track instead of replacing
    #[serde(default = "default_true")]
    pub multi_track: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Serialize)]
pub struct StartTaskResponse {
    pub task_id: String,
}

#[derive(Debug, Deserialize)]
pub struct GetTaskRequest {
    #[serde(default, rename = "taskId")]
    pub task_id: String,
}

#[derive(Debug, Serialize)]
pub struct GetTaskResponse {
    pub task_id: String,
    pub process_percent: u8,
    pub video_info: Option<VideoInfo>,
    pub subtitle_info: Vec<SubtitleInfoDto>,
    pub target_language: String,
    pub speech_download_url: String,
}

#[derive(Debug, Serialize)]
pub struct VideoInfo {
    pub title: String,
    pub description: String,
    pub translated_title: String,
    pub translated_description: String,
    pub language: String,
}

#[derive(Debug, Serialize)]
pub struct SubtitleInfoDto {
    pub name: String,
    pub download_url: String,
}

// --- File DTOs ---

#[derive(Debug, Serialize)]
pub struct UploadFileResponse {
    pub file_path: Vec<String>,
}
