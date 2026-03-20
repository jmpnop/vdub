use crate::types::subtitle::{SrtBlock, TranslatedItem, Word, format_time_range};

/// Generate SRT blocks with timestamps by aligning translated sentences to word-level timestamps
pub fn generate_srt_with_timestamps(
    items: &[TranslatedItem],
    words: &[Word],
    time_offset: f64,
) -> Vec<SrtBlock> {
    if items.is_empty() || words.is_empty() {
        return Vec::new();
    }

    let full_text: String = words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
    let full_clean = clean_for_match(&full_text);

    let mut blocks = Vec::new();
    let mut last_end_time = time_offset;
    let mut search_offset = 0usize;

    for (idx, item) in items.iter().enumerate() {
        let sentence_clean = clean_for_match(&item.origin_text);

        if sentence_clean.is_empty() {
            continue;
        }

        // Find the sentence position in the full text
        let (start_time, end_time) = if let Some(pos) = full_clean[search_offset..].find(&sentence_clean) {
            let abs_pos = search_offset + pos;
            let char_end = abs_pos + sentence_clean.len();

            let start = char_index_to_time(&full_clean, abs_pos, words, time_offset);
            let end = char_index_to_time(&full_clean, char_end, words, time_offset);

            search_offset = char_end;
            (start.max(last_end_time), end)
        } else {
            // Fallback: distribute evenly among remaining items
            let remaining = items.len() - idx;
            let total_remaining = if let Some(last_word) = words.last() {
                last_word.end + time_offset - last_end_time
            } else {
                0.0
            };
            let chunk = total_remaining / remaining as f64;
            (last_end_time, last_end_time + chunk)
        };

        let start = start_time.max(0.0);
        let end = end_time.max(start + 0.1);

        blocks.push(SrtBlock {
            index: blocks.len() + 1,
            timestamp: format_time_range(start, end),
            origin_language_sentence: item.origin_text.clone(),
            target_language_sentence: item.translated_text.clone(),
        });

        last_end_time = end;
    }

    blocks
}

/// Map a character index in the cleaned full text to a timestamp
fn char_index_to_time(_full_clean: &str, char_idx: usize, words: &[Word], offset: f64) -> f64 {
    let mut accumulated = 0usize;

    for word in words {
        let word_clean = clean_for_match(&word.text);
        let word_len = word_clean.len();

        if accumulated + word_len > char_idx {
            // Interpolate within this word
            let frac = (char_idx - accumulated) as f64 / word_len.max(1) as f64;
            return word.start + (word.end - word.start) * frac + offset;
        }

        accumulated += word_len;
    }

    // Past the end — return last word's end time
    words.last().map(|w| w.end + offset).unwrap_or(offset)
}

/// Remove punctuation and spaces for fuzzy matching
fn clean_for_match(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}
