//! CLI art, banners, and emoji-rich progress reporting for KrillinAI

const BANNER: &str = r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║   ██╗  ██╗██████╗ ██╗██╗     ██╗     ██╗███╗   ██╗           ║
    ║   ██║ ██╔╝██╔══██╗██║██║     ██║     ██║████╗  ██║           ║
    ║   █████╔╝ ██████╔╝██║██║     ██║     ██║██╔██╗ ██║           ║
    ║   ██╔═██╗ ██╔══██╗██║██║     ██║     ██║██║╚██╗██║           ║
    ║   ██║  ██╗██║  ██║██║███████╗███████╗██║██║ ╚████║           ║
    ║   ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝           ║
    ║                                                               ║
    ║          🎬  Video Dubbing & Subtitle Engine  🎬              ║
    ║              ⚡ Powered by Rust + MLX ⚡                      ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
"#;

const DRAGON_ART: &str = r#"
              ___====-_  _-====___
        _--^^^#####//      \\#####^^^--_
     _-^##########// (    ) \\##########^-_
    -############//  |\^^/|  \\############-
  _/############//   (@::@)   \\############\_
 /#############((     \\//     ))#############\
-###############\\    (oo)    //###############-
-#################\\  / "" \  //#################-
-###################\\/      \//###################-
_#/|##########/\######(   /\   )######/\##########|\#_
|/ |#/\#/\#/\/  \#/\##\  |  |  /##/\#/  \/\#/\#/\#| \|
`  |/  V  V  `   V  \#\| |  | |/#/  V   '  V  V  \|  '
   `   `  `      `   / | |  | | \   '      '  '   '
                    (  | |  | |  )
                   __\ | |  | | /__
                  (vvv(VVV)(VVV)vvv)
"#;

/// Print the startup banner with system info
pub fn print_banner(host: &str, port: u16) {
    eprintln!("{BANNER}");
    eprintln!("    🌐 Server: http://{host}:{port}");
    eprintln!("    📁 Tasks:  ./tasks/");
    eprintln!("    ⚙️  Config: ./config/config.toml");
    eprintln!();
}

/// Print the dragon ASCII art on first startup
pub fn print_dragon() {
    eprintln!("{DRAGON_ART}");
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

pub fn step_transcribe_segment(i: usize, total: usize) {
    let bar = progress_bar(i + 1, total, 20);
    tracing::info!("   📝 Transcribing segment {}/{total} {bar}", i + 1);
}

pub fn step_transcribe_lang_detected(lang: &str) {
    tracing::info!("   🎯 Detected language: {lang}");
}

pub fn step_translate_start(from: &str, to: &str) {
    tracing::info!("   🔄 Translating: {from} → {to}");
}

pub fn step_transcribe_done(blocks: usize) {
    tracing::info!("   ✅ {blocks} subtitle blocks generated");
}

pub fn step_tts_start(provider: &str, voice: &str) {
    tracing::info!("🔊 ─── Step 3/5: Text-to-Speech ────────────────");
    tracing::info!("   🎤 TTS: {provider}");
    tracing::info!("   🗣️  Voice: {voice}");
}

pub fn step_tts_progress(done: usize, total: usize) {
    let bar = progress_bar(done, total, 20);
    tracing::info!("   🎵 TTS progress: {done}/{total} {bar}");
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
