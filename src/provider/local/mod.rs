pub mod edge_tts;
pub mod fasterwhisper;
#[cfg(target_os = "macos")]
pub mod mlx_audio;
#[cfg(target_os = "macos")]
pub mod mlx_whisper;
pub mod whispercpp;
pub mod whisperkit;
