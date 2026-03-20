use crate::config::Config;
use crate::provider::{ChatCompleter, Transcriber};
use crate::service::split_audio;
use crate::service::timestamps::generate_srt_with_timestamps;
use crate::storage::BinPaths;
use crate::types::subtitle::{SrtBlock, TranscriptionData, TranslatedItem};
use crate::types::task::{StepParam, SubtitleResultType};
use crate::util::cli_art;
use std::fmt::Write as FmtWrite;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

/// Step 2: Transcribe audio and translate to generate SRT files
pub async fn audio_to_subtitle(
    bins: &BinPaths,
    config: &Config,
    transcriber: &Arc<dyn Transcriber>,
    chat_completer: &Arc<dyn ChatCompleter>,
    param: &mut StepParam,
) -> anyhow::Result<()> {
    // Convert audio to mono 16kHz for ASR
    let mono_audio = crate::util::audio::process_audio(&bins.ffmpeg, Path::new(&param.audio_file_path)).await?;

    // Get split points
    let split_points = split_audio::get_split_points(
        &bins.ffmpeg,
        &bins.ffprobe,
        Path::new(&mono_audio),
        config.app.segment_duration,
    )
    .await?;

    let num_segments = split_points.len() - 1;
    tracing::info!("   🔪 Audio split into {num_segments} segments");

    let transcribe_sem = Arc::new(Semaphore::new(config.app.transcribe_parallel_num as usize));
    let translate_sem = Arc::new(Semaphore::new(config.app.translate_parallel_num as usize));
    let max_attempts = config.app.transcribe_max_attempts as usize;
    let max_sentence_len = config.app.max_sentence_length as usize;

    // Determine the language to pass to ASR
    // If "auto", pass empty string to enable auto-detection
    let asr_language = if param.origin_language == "auto" {
        String::new()
    } else {
        param.origin_language.clone()
    };

    // Phase 1: Split and transcribe using JoinSet
    let mut transcription_results: Vec<(usize, TranscriptionData, f64)> = Vec::with_capacity(num_segments);

    let mut join_set = JoinSet::new();
    for i in 0..num_segments {
        let start = split_points[i];
        let end = split_points[i + 1];
        let segment_path = format!("{}/split_audio_{:03}.mp3", param.task_base_path, i);
        let mono_audio = mono_audio.clone();
        let ffmpeg = bins.ffmpeg.clone();
        let transcriber = transcriber.clone();
        let sem = transcribe_sem.clone();
        let lang = asr_language.clone();
        let base_path = param.task_base_path.clone();
        let total = num_segments;

        join_set.spawn(async move {
            // Split
            split_audio::clip_audio(
                &ffmpeg,
                Path::new(&mono_audio),
                Path::new(&segment_path),
                start,
                end,
            )
            .await?;

            // Transcribe with retry
            let _permit = sem.acquire().await?;
            cli_art::step_transcribe_segment(i, total);

            let mut last_err = None;
            for attempt in 0..max_attempts.max(1) {
                match transcriber
                    .transcription(
                        Path::new(&segment_path),
                        &lang,
                        Path::new(&base_path),
                    )
                    .await
                {
                    Ok(data) => {
                        // Save transcription data
                        let json_path = format!("{base_path}/audio_transcription_data_{i:03}.json");
                        if let Ok(json) = serde_json::to_string_pretty(&data) {
                            let _ = tokio::fs::write(&json_path, json).await;
                        }
                        return Ok::<_, anyhow::Error>((i, data, start));
                    }
                    Err(e) => {
                        tracing::warn!(
                            "   ⚠️  Transcription attempt {}/{max_attempts} failed for segment {i}: {e}",
                            attempt + 1
                        );
                        last_err = Some(e);
                    }
                }
            }
            Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Transcription failed")))
        });
    }

    // Collect results as they complete
    while let Some(result) = join_set.join_next().await {
        let result = result??;
        transcription_results.push(result);
    }

    // Sort by segment index
    transcription_results.sort_by_key(|(i, _, _)| *i);

    // Auto language detection: use the detected language from the first segment
    if param.origin_language == "auto" && !transcription_results.is_empty() {
        let detected = &transcription_results[0].1.language;
        if !detected.is_empty() {
            cli_art::step_transcribe_lang_detected(detected);
            param.detected_language = detected.clone();
            param.origin_language = detected.clone();

            // Auto-determine target language (EN↔RU)
            if param.target_language == "auto" {
                param.target_language = cli_art::auto_target_language(detected).to_string();
            }

            cli_art::auto_lang_info(
                cli_art::lang_display_name(&param.origin_language),
                cli_art::lang_display_name(&param.target_language),
            );
        } else {
            // Fallback: assume English → Russian
            tracing::warn!("   ⚠️  Language detection returned empty, defaulting to en → ru");
            param.origin_language = "en".to_string();
            if param.target_language == "auto" {
                param.target_language = "ru".to_string();
            }
        }
    }

    // Phase 2: Translate each segment's text
    let needs_translation = param.subtitle_result_type != SubtitleResultType::OriginOnly
        && !param.target_language.is_empty()
        && param.origin_language != param.target_language;

    if needs_translation {
        cli_art::step_translate_start(
            cli_art::lang_display_name(&param.origin_language),
            cli_art::lang_display_name(&param.target_language),
        );
    }

    let mut all_blocks: Vec<SrtBlock> = Vec::new();

    for (seg_idx, transcription, time_offset) in &transcription_results {
        if transcription.text.trim().is_empty() {
            continue;
        }

        let items = if needs_translation {
            translate_text(
                chat_completer,
                &translate_sem,
                &transcription.text,
                &param.origin_language,
                &param.target_language,
                max_sentence_len,
                config.app.translate_max_attempts as usize,
            )
            .await?
        } else {
            // No translation needed — just split into sentences
            split_into_sentences(&transcription.text, max_sentence_len)
                .into_iter()
                .map(|s| TranslatedItem {
                    origin_text: s.clone(),
                    translated_text: s,
                })
                .collect()
        };

        // Generate timestamp-aligned SRT blocks
        let blocks = generate_srt_with_timestamps(&items, &transcription.words, *time_offset);
        all_blocks.extend(blocks);

        // Save translation data
        let json_path = format!("{}/translation_data_{seg_idx:03}.json", param.task_base_path);
        if let Ok(json) = serde_json::to_string_pretty(&items) {
            let _ = tokio::fs::write(&json_path, json).await;
        }
    }

    // Re-index blocks
    for (i, block) in all_blocks.iter_mut().enumerate() {
        block.index = i + 1;
    }

    // Write SRT files
    write_srt_files(param, &all_blocks).await?;

    cli_art::step_transcribe_done(all_blocks.len());
    Ok(())
}

/// Translate text using the LLM with context-aware prompting
async fn translate_text(
    chat_completer: &Arc<dyn ChatCompleter>,
    sem: &Arc<Semaphore>,
    text: &str,
    origin_lang: &str,
    target_lang: &str,
    max_sentence_len: usize,
    max_attempts: usize,
) -> anyhow::Result<Vec<TranslatedItem>> {
    let sentences = split_into_sentences(text, max_sentence_len);
    let mut results = Vec::with_capacity(sentences.len());

    for (i, sentence) in sentences.iter().enumerate() {
        let _permit = sem.acquire().await?;

        // Build context (3 before + 3 after)
        let prev: Vec<&str> = sentences[i.saturating_sub(3)..i]
            .iter()
            .map(|s| s.as_str())
            .collect();
        let next: Vec<&str> = sentences[i + 1..sentences.len().min(i + 4)]
            .iter()
            .map(|s| s.as_str())
            .collect();

        let prompt = crate::types::prompts::SPLIT_TEXT_WITH_CONTEXT_PROMPT
            .replace("{origin_lang}", origin_lang)
            .replace("{target_lang}", target_lang)
            .replace("{prev_context}", &prev.join("\n"))
            .replace("{text}", sentence)
            .replace("{next_context}", &next.join("\n"));

        let mut last_err = None;
        for attempt in 0..max_attempts.max(1) {
            match chat_completer.chat_completion(&prompt).await {
                Ok(translated) => {
                    let translated = translated.trim().to_string();
                    results.push(TranslatedItem {
                        origin_text: sentence.clone(),
                        translated_text: translated,
                    });
                    last_err = None;
                    break;
                }
                Err(e) => {
                    tracing::warn!(
                        "   ⚠️  Translation attempt {}/{max_attempts} failed for sentence {i}: {e}",
                        attempt + 1
                    );
                    last_err = Some(e);
                }
            }
        }

        if let Some(e) = last_err {
            // Use original text as fallback
            tracing::error!("   ❌ Translation failed for sentence {i}, using original: {e}");
            results.push(TranslatedItem {
                origin_text: sentence.clone(),
                translated_text: sentence.clone(),
            });
        }
    }

    Ok(results)
}

/// Split text into sentences respecting max length
fn split_into_sentences(text: &str, max_chars: usize) -> Vec<String> {
    let delimiters: &[&str] = &[". ", "! ", "? ", "\u{3002}", "\u{FF01}", "\u{FF1F}", "\u{FF1B}"];
    let mut sentences = Vec::new();
    let mut current = String::new();

    for c in text.chars() {
        current.push(c);
        if delimiters.iter().any(|d| current.ends_with(d)) || current.len() >= max_chars {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current.clear();
        }
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        sentences.push(trimmed);
    }

    sentences
}

/// Write the various SRT output files using write! macro for efficiency
async fn write_srt_files(param: &mut StepParam, blocks: &[SrtBlock]) -> anyhow::Result<()> {
    let output_dir = param.output_dir();

    // Bilingual SRT
    let bilingual_path = format!("{output_dir}/{}", crate::types::task::BILINGUAL_SRT_FILE);
    let mut bilingual = String::with_capacity(blocks.len() * 120);
    for block in blocks {
        let text = match param.subtitle_result_type {
            SubtitleResultType::BilingualTranslationOnTop => {
                format!("{}\n{}", block.target_language_sentence, block.origin_language_sentence)
            }
            SubtitleResultType::BilingualTranslationOnBottom => {
                format!("{}\n{}", block.origin_language_sentence, block.target_language_sentence)
            }
            _ => block.origin_language_sentence.clone(),
        };
        let _ = write!(bilingual, "{}\n{}\n{}\n\n", block.index, block.timestamp, text);
    }
    tokio::fs::write(&bilingual_path, &bilingual).await?;
    param.bilingual_srt_file_path = bilingual_path.clone();

    // Origin-only SRT
    let origin_path = format!("{output_dir}/{}", crate::types::task::ORIGIN_LANG_SRT_FILE);
    let mut origin = String::with_capacity(blocks.len() * 80);
    for block in blocks {
        let _ = write!(
            origin, "{}\n{}\n{}\n\n",
            block.index, block.timestamp, block.origin_language_sentence
        );
    }
    tokio::fs::write(&origin_path, &origin).await?;

    // Target-only SRT
    let target_path = format!("{output_dir}/{}", crate::types::task::TARGET_LANG_SRT_FILE);
    let mut target = String::with_capacity(blocks.len() * 80);
    for block in blocks {
        let _ = write!(
            target, "{}\n{}\n{}\n\n",
            block.index, block.timestamp, block.target_language_sentence
        );
    }
    tokio::fs::write(&target_path, &target).await?;

    // Set the TTS source path based on result type
    param.tts_source_file_path = match param.subtitle_result_type {
        SubtitleResultType::TargetOnly
        | SubtitleResultType::BilingualTranslationOnTop
        | SubtitleResultType::BilingualTranslationOnBottom => target_path,
        SubtitleResultType::OriginOnly => origin_path,
    };

    Ok(())
}
