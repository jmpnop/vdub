pub mod task_store;

use std::path::{Path, PathBuf};

/// Paths to external CLI tool binaries
#[derive(Debug, Clone, Default)]
pub struct BinPaths {
    pub ffmpeg: String,
    pub ffprobe: String,
    pub ytdlp: String,
    pub fasterwhisper: String,
    pub whisperx: String,
    pub whisperkit: String,
    pub whispercpp: String,
    pub edge_tts: String,
    pub mlx_whisper: String,
    pub mlx_audio: String,
}

impl BinPaths {
    /// Try to locate tools on PATH, fall back to ./bin/
    pub fn detect() -> Self {
        use crate::util::cli_art;

        cli_art::print_tool_scan();

        let bins = Self {
            ffmpeg: which("ffmpeg"),
            ffprobe: which("ffprobe"),
            ytdlp: which("yt-dlp"),
            fasterwhisper: which("fasterwhisper"),
            whisperx: which("whisperx"),
            whisperkit: which("whisperkit"),
            whispercpp: which("whisper-cpp"),
            edge_tts: which("edge-tts"),
            mlx_whisper: which("mlx_whisper"),
            mlx_audio: which("python3"), // mlx-audio runs via python3 -m
        };

        // Report what we found
        for (name, path) in [
            ("ffmpeg", &bins.ffmpeg),
            ("ffprobe", &bins.ffprobe),
            ("yt-dlp", &bins.ytdlp),
            ("fasterwhisper", &bins.fasterwhisper),
            ("whisperkit", &bins.whisperkit),
            ("whisper-cpp", &bins.whispercpp),
            ("edge-tts", &bins.edge_tts),
            ("mlx_whisper", &bins.mlx_whisper),
        ] {
            if Path::new(path).exists() || which_system(name).is_some() {
                cli_art::tool_detected(name, path);
            }
        }

        bins
    }

    /// Validate that required tools exist. Returns warnings for missing optional tools.
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        // Required
        if !is_available(&self.ffmpeg) {
            warnings.push("⚠️  ffmpeg not found — required for audio/video processing".to_string());
        }
        if !is_available(&self.ffprobe) {
            warnings.push("⚠️  ffprobe not found — required for media analysis".to_string());
        }

        // Optional
        if !is_available(&self.ytdlp) {
            warnings.push("ℹ️  yt-dlp not found — URL downloads will not work".to_string());
        }

        warnings
    }
}

fn which(name: &str) -> String {
    // Check ./bin/ first, then PATH
    let local = format!("./bin/{name}");
    if Path::new(&local).exists() {
        return local;
    }
    // Fall back to bare name (relies on PATH)
    name.to_string()
}

fn which_system(name: &str) -> Option<PathBuf> {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
}

fn is_available(path: &str) -> bool {
    Path::new(path).exists() || which_system(path).is_some()
}
