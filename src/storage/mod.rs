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
    /// Try to locate tools — checks venv/bin first, then ./bin/, then PATH
    pub fn detect_with_venv(venv_bin: Option<&Path>) -> Self {
        use crate::util::cli_art;

        cli_art::print_tool_scan();

        let bins = Self {
            ffmpeg: which("ffmpeg", venv_bin),
            ffprobe: which("ffprobe", venv_bin),
            ytdlp: which("yt-dlp", venv_bin),
            fasterwhisper: which("fasterwhisper", venv_bin),
            whisperx: which("whisperx", venv_bin),
            whisperkit: which_any(&["whisperkit-cli", "whisperkit"], venv_bin),
            whispercpp: which("whisper-cpp", venv_bin),
            edge_tts: which("edge-tts", venv_bin),
            mlx_whisper: which("mlx_whisper", venv_bin),
            mlx_audio: which_any(&["mlx_audio", "python3"], venv_bin),
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

    /// Detect without venv (legacy)
    pub fn detect() -> Self {
        Self::detect_with_venv(None)
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

fn which(name: &str, venv_bin: Option<&Path>) -> String {
    // Check venv/bin first
    if let Some(vbin) = venv_bin {
        let venv_path = vbin.join(name);
        if venv_path.exists() {
            return venv_path.to_string_lossy().to_string();
        }
    }
    // Check ./bin/
    let local = format!("./bin/{name}");
    if Path::new(&local).exists() {
        return local;
    }
    // Fall back to bare name (relies on PATH)
    name.to_string()
}

fn which_any(names: &[&str], venv_bin: Option<&Path>) -> String {
    for name in names {
        let path = which(name, venv_bin);
        if Path::new(&path).exists() || which_system(&path).is_some() {
            return path;
        }
    }
    names[0].to_string()
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
