use regex::Regex;
use std::sync::LazyLock;

static SANITIZE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[<>:"/\\|?*\x00-\x1f]"#).unwrap());

static YOUTUBE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:youtube\.com/(?:watch\?v=|embed/|v/|shorts/)|youtu\.be/)([a-zA-Z0-9_-]{11})")
        .unwrap()
});

static BILIBILI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"bilibili\.com/video/(BV[a-zA-Z0-9]+)").unwrap());

/// Remove characters that are invalid in file paths
pub fn sanitize_path_name(name: &str) -> String {
    let sanitized = SANITIZE_RE.replace_all(name, "_").to_string();
    sanitized.trim_matches('.').to_string()
}

/// Generate a random alphanumeric string of length n
pub fn rand_string(n: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ123456789";
    let mut rng = rand::thread_rng();
    (0..n)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Clean punctuation from a word
pub fn clean_punctuation(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'' || *c == '-')
        .collect()
}

/// Extract YouTube video ID from URL
pub fn get_youtube_id(url: &str) -> Option<String> {
    YOUTUBE_RE.captures(url).map(|c| c[1].to_string())
}

/// Extract Bilibili video ID from URL
pub fn get_bilibili_id(url: &str) -> Option<String> {
    BILIBILI_RE.captures(url).map(|c| c[1].to_string())
}

/// Check if a character is part of a CJK script
pub fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |   // CJK Unified
        '\u{3400}'..='\u{4DBF}' |   // CJK Extension A
        '\u{3040}'..='\u{309F}' |   // Hiragana
        '\u{30A0}'..='\u{30FF}' |   // Katakana
        '\u{AC00}'..='\u{D7AF}'     // Hangul
    )
}

/// Check if a language code represents an Asian language
pub fn is_asian_language(code: &str) -> bool {
    matches!(
        code,
        "zh_cn" | "zh_tw" | "ja" | "ko" | "th" | "vi" | "my" | "km" | "lo"
    )
}

/// Count effective (alphanumeric) characters
pub fn count_effective_chars(text: &str) -> usize {
    text.chars().filter(|c| c.is_alphanumeric()).count()
}
