//! CLI art, banners, and emoji-rich progress reporting for vdub

const BANNER: &str = r#"
    ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
    ┃                                   ┃
    ┃   ██╗   ██╗██████╗ ██╗   ██╗██████╗  ┃
    ┃   ██║   ██║██╔══██╗██║   ██║██╔══██╗ ┃
    ┃   ██║   ██║██║  ██║██║   ██║██████╔╝ ┃
    ┃   ╚██╗ ██╔╝██║  ██║██║   ██║██╔══██╗ ┃
    ┃    ╚████╔╝ ██████╔╝╚██████╔╝██████╔╝ ┃
    ┃     ╚═══╝  ╚═════╝  ╚═════╝ ╚═════╝  ┃
    ┃                                   ┃
    ┃      🎬  Video Dubbing Engine  🎬     ┃
    ┃         ⚡ Rust + MLX ⚡              ┃
    ┃                                   ┃
    ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
"#;

const STARTUP_ART: &str = include_str!("../../assets/option3_hires.txt");

/// Print the startup banner with system info
pub fn print_banner(host: &str, port: u16) {
    eprintln!("{BANNER}");
    eprintln!("    🌐 Server: http://{host}:{port}");
    eprintln!("    📁 Tasks:  ./tasks/");
    eprintln!("    ⚙️  Config: ./config/config.toml");
    eprintln!();
}

/// Print the La Catrina startup art (Coco-style sugar skull)
pub fn print_skull() {
    eprintln!("{STARTUP_ART}");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Pipeline plan summary
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn pipeline_plan(
    param: &crate::types::task::StepParam,
    transcribe_provider: &str,
    tts_provider: &str,
) {
    use crate::types::task::{EmbedVideoType, SubtitleResultType};

    tracing::info!("🎯 ═══════════════════════════════════════════════");
    tracing::info!("🎯  Pipeline Plan");
    tracing::info!("🎯 ═══════════════════════════════════════════════");

    // Language
    let from = if param.origin_language == "auto" { "auto-detect" } else { lang_display_name(&param.origin_language) };
    let to = if param.target_language == "auto" { "auto-select" } else { lang_display_name(&param.target_language) };
    tracing::info!("   🌍 Language:   {from} → {to}");

    // Subtitles
    let sub_type = match param.subtitle_result_type {
        SubtitleResultType::BilingualTranslationOnTop | SubtitleResultType::BilingualTranslationOnBottom => "bilingual",
        SubtitleResultType::TargetOnly => "target language only",
        SubtitleResultType::OriginOnly => "original language only",
    };
    tracing::info!("   📝 Subtitles:  {sub_type}");

    // ASR
    tracing::info!("   🧠 ASR:        {transcribe_provider}");

    // TTS / Audio
    if param.enable_tts {
        tracing::info!("   🔊 TTS:        {tts_provider} (voice: {})", param.tts_voice_code);
        if param.multi_track_audio {
            tracing::info!("   🎵 Audio:      dual-track (original + dubbed)");
        } else {
            tracing::info!("   🎵 Audio:      single-track (dubbed replaces original)");
        }
    } else {
        tracing::info!("   🔊 TTS:        disabled");
    }

    // Video embed
    let embed = match param.embed_subtitle_video_type {
        EmbedVideoType::Horizontal => "horizontal",
        EmbedVideoType::Vertical => "vertical",
        EmbedVideoType::All => "horizontal + vertical",
        EmbedVideoType::None => "disabled",
    };
    tracing::info!("   🎞️  Embed:      {embed}");
    tracing::info!("   📁 Output:     {}/output", param.task_base_path);
    tracing::info!("🎯 ═══════════════════════════════════════════════");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Pipeline step progress emojis
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn step_download_start(url: &str) {
    tracing::info!("📥 ─── Step 1/5: Downloading ────────────────────");
    tracing::info!("   🔗 Source: {url}");
}

pub fn step_download_done() {
    tracing::info!("   ✅ Download complete");
}

pub fn step_transcribe_start(provider: &str, lang: &str) {
    tracing::info!("🎙️ ─── Step 2/5: Transcribe & Translate ────────");
    tracing::info!("   🧠 ASR: {provider}");
    let lang_display = if lang.is_empty() || lang == "auto" { "🔍 auto-detect" } else { lang };
    tracing::info!("   🌍 Language: {lang_display}");
}

pub fn step_transcribe_segment(done: usize, total: usize, eta: Option<std::time::Duration>) {
    let bar = progress_bar(done, total, 20);
    let eta_str = format_eta(eta);
    tracing::info!("   📝 Transcribed {done}/{total} {bar}{eta_str}");
}

pub fn step_transcribe_lang_detected(lang: &str) {
    tracing::info!("   🎯 Detected language: {lang}");
}

pub fn step_translate_start(from: &str, to: &str) {
    tracing::info!("   🔄 Translating: {from} → {to}");
}

pub fn step_translate_progress(done: usize, total: usize, eta: Option<std::time::Duration>) {
    let bar = progress_bar(done, total, 20);
    let eta_str = format_eta(eta);
    tracing::info!("   🔄 Translated {done}/{total} segments {bar}{eta_str}");
}

pub fn step_transcribe_done(blocks: usize) {
    tracing::info!("   ✅ {blocks} subtitle blocks generated");
}

pub fn step_tts_start(provider: &str, voice: &str) {
    tracing::info!("🔊 ─── Step 3/5: Text-to-Speech ────────────────");
    tracing::info!("   🎤 TTS: {provider}");
    tracing::info!("   🗣️  Voice: {voice}");
}

pub fn step_tts_progress(done: usize, total: usize, eta: Option<std::time::Duration>) {
    let bar = progress_bar(done, total, 20);
    let eta_str = format_eta(eta);
    tracing::info!("   🎵 TTS progress: {done}/{total} {bar}{eta_str}");
}

pub fn step_tts_done() {
    tracing::info!("   ✅ TTS audio generated");
}

pub fn step_embed_start(video_type: &str) {
    tracing::info!("🎞️ ─── Step 4/5: Embed Subtitles ────────────────");
    tracing::info!("   📐 Format: {video_type}");
}

pub fn step_embed_done() {
    tracing::info!("   ✅ Subtitles embedded");
}

pub fn step_finalize_start() {
    tracing::info!("📦 ─── Step 5/5: Finalize ──────────────────────");
}

pub fn step_finalize_done(file_count: usize) {
    tracing::info!("   ✅ {file_count} output files ready");
}

pub fn pipeline_success(task_id: &str) {
    tracing::info!("🎉 ═══════════════════════════════════════════════");
    tracing::info!("🎉  Pipeline complete: {task_id}");
    tracing::info!("🎉 ═══════════════════════════════════════════════");
}

pub fn pipeline_failed(task_id: &str, err: &str) {
    tracing::error!("💥 ═══════════════════════════════════════════════");
    tracing::error!("💥  Pipeline FAILED: {task_id}");
    tracing::error!("💥  Error: {err}");
    tracing::error!("💥 ═══════════════════════════════════════════════");
}

pub fn tool_detected(name: &str, path: &str) {
    let icon = match name {
        "ffmpeg" | "ffprobe" => "🎬",
        "yt-dlp" => "📺",
        "edge-tts" => "🗣️",
        "mlx_whisper" => "🧠",
        "mlx-audio" => "🎵",
        _ if name.contains("whisper") => "🎙️",
        _ => "🔧",
    };
    tracing::info!("   {icon} {name}: {path}");
}

pub fn tool_missing(name: &str) {
    tracing::warn!("   ⚠️  {name}: not found");
}

pub fn print_tool_scan() {
    tracing::info!("🔍 Scanning for external tools...");
}

pub fn auto_lang_info(detected: &str, target: &str) {
    tracing::info!("   🤖 Auto mode: {detected} → {target}");
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn format_eta(eta: Option<std::time::Duration>) -> String {
    match eta {
        Some(d) => {
            let secs = d.as_secs();
            let h = secs / 3600;
            let m = (secs % 3600) / 60;
            let s = secs % 60;
            if h > 0 {
                format!(" ETA {h}h{m:02}m{s:02}s")
            } else if m > 0 {
                format!(" ETA {m}m{s:02}s")
            } else {
                format!(" ETA {s}s")
            }
        }
        None => String::new(),
    }
}

fn progress_bar(done: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return "░".repeat(width);
    }
    let filled = (done * width) / total;
    let empty = width - filled;
    format!("█{}░{}", "█".repeat(filled.saturating_sub(1)), "░".repeat(empty))
}

/// ISO 639-1 (2-letter) to ISO 639-2/T (3-letter) for ffmpeg metadata
pub fn lang_to_iso639_2(code: &str) -> &str {
    match code {
        "en" => "eng",
        "ru" => "rus",
        "zh_cn" | "zh_tw" | "zh" => "zho",
        "ja" => "jpn",
        "ko" => "kor",
        "fr" => "fra",
        "de" => "deu",
        "es" => "spa",
        "pt" => "por",
        "it" => "ita",
        "nl" => "nld",
        "pl" => "pol",
        "tr" => "tur",
        "ar" => "ara",
        "th" => "tha",
        "vi" => "vie",
        "id" => "ind",
        "hi" => "hin",
        "uk" => "ukr",
        "sv" => "swe",
        "da" => "dan",
        "fi" => "fin",
        "no" => "nor",
        "el" => "ell",
        "cs" => "ces",
        "ro" => "ron",
        "hu" => "hun",
        "he" => "heb",
        "fa" => "fas",
        "bn" => "ben",
        "ta" => "tam",
        "ka" => "kat",
        _ => code,
    }
}

/// Get display name for a language code
pub fn lang_display_name(code: &str) -> &str {
    match code {
        "en" => "English",
        "ru" => "Russian",
        "zh_cn" => "Chinese (Simplified)",
        "zh_tw" => "Chinese (Traditional)",
        "ja" => "Japanese",
        "ko" => "Korean",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        "pt" => "Portuguese",
        "it" => "Italian",
        "uk" => "Ukrainian",
        "ar" => "Arabic",
        "hi" => "Hindi",
        "th" => "Thai",
        "vi" => "Vietnamese",
        "tr" => "Turkish",
        _ => code,
    }
}

/// Determine auto target language based on detected source language.
/// English ↔ Russian by default. Other languages → English.
pub fn auto_target_language(detected: &str) -> &str {
    match detected {
        "en" | "english" => "ru",
        "ru" | "russian" => "en",
        _ => "en",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn startup_art_is_not_empty() {
        assert!(!STARTUP_ART.is_empty(), "STARTUP_ART must not be empty");
    }

    #[test]
    fn startup_art_has_leading_spaces() {
        let ansi_re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        for (i, line) in STARTUP_ART.lines().enumerate() {
            let visible = ansi_re.replace_all(line, "");
            if visible.trim().is_empty() {
                continue;
            }
            let leading = visible.len() - visible.trim_start().len();
            assert!(
                leading > 0,
                "Line {i} has no leading spaces: visible = {:?}",
                &visible[..visible.len().min(40)]
            );
        }
    }

    #[test]
    fn startup_art_contains_ansi_escapes() {
        assert!(
            STARTUP_ART.contains('\x1b'),
            "STARTUP_ART must contain ANSI escape sequences"
        );
    }

    #[test]
    fn startup_art_lines_are_consistent_width() {
        let ansi_re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        let widths: Vec<usize> = STARTUP_ART
            .lines()
            .map(|line| ansi_re.replace_all(line, "").chars().count())
            .collect();
        // All non-empty lines should have the same visible width (80 cols)
        let non_empty: Vec<usize> = widths.iter().copied().filter(|&w| w > 0).collect();
        if let Some(&expected) = non_empty.first() {
            for (i, &w) in non_empty.iter().enumerate() {
                assert_eq!(
                    w, expected,
                    "Line {i} has visible width {w}, expected {expected}"
                );
            }
        }
    }

    #[test]
    fn banner_is_valid() {
        assert!(!BANNER.is_empty(), "BANNER must not be empty");
        assert!(BANNER.contains("VDUB") || BANNER.contains("██"), "BANNER must contain VDUB text");
    }
}
