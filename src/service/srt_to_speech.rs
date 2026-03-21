use crate::config::Config;
use crate::provider::Ttser;
use crate::storage::BinPaths;
use crate::types::subtitle::{SrtSentenceWithStrTime, parse_timestamp};
use crate::types::task::{self, StepParam};
use crate::util::{cli_art, cmd};
use crate::util::srt::parse_srt;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_FAILURE_RATIO: f64 = 0.5;

/// Step 3: Generate TTS audio for each subtitle and replace/add to video audio
pub async fn srt_to_speech(
    bins: &BinPaths,
    config: &Config,
    tts_client: &Arc<dyn Ttser>,
    param: &mut StepParam,
) -> anyhow::Result<()> {
    if !param.enable_tts {
        tracing::info!("   ⏭️  TTS disabled, skipping step 3");
        return Ok(());
    }

    // Parse SRT file
    let srt_content = tokio::fs::read_to_string(&param.tts_source_file_path).await?;
    let subtitles = parse_srt(&srt_content);

    if subtitles.is_empty() {
        tracing::warn!("   ⚠️  No subtitles found for TTS");
        return Ok(());
    }

    tracing::info!("   🎵 Generating TTS for {} entries", subtitles.len());

    // Generate TTS for each subtitle concurrently
    let tts_parallel = config.app.tts_parallel_num as usize;
    let sem = Arc::new(Semaphore::new(tts_parallel));
    let mut handles = Vec::new();
    let mut failed_count = 0usize;

    for (i, sub) in subtitles.iter().enumerate() {
        let sub_audio_path = format!("{}/subtitle_{i}.wav", param.task_base_path);
        let tts = tts_client.clone();
        let sem = sem.clone();
        let voice = param.tts_voice_code.clone();
        let text = sub.text.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await?;
            tts.text_to_speech(&text, &voice, Path::new(&sub_audio_path))
                .await?;
            Ok::<_, anyhow::Error>(sub_audio_path)
        }));
    }

    let mut audio_paths: Vec<Option<String>> = vec![None; subtitles.len()];
    let total = subtitles.len();
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await? {
            Ok(path) => {
                audio_paths[i] = Some(path);
                if (i + 1) % 5 == 0 || i + 1 == total {
                    cli_art::step_tts_progress(i + 1, total);
                }
            }
            Err(e) => {
                tracing::warn!("   ⚠️  TTS failed for subtitle {i}: {e}");
                failed_count += 1;
            }
        }
    }

    // Check failure threshold
    if failed_count as f64 / subtitles.len() as f64 > MAX_FAILURE_RATIO {
        anyhow::bail!(
            "Too many TTS failures: {failed_count}/{} exceeded {MAX_FAILURE_RATIO} threshold",
            subtitles.len()
        );
    }

    // Build the final audio by adjusting durations and concatenating
    let final_audio = format!("{}/{}", param.task_base_path, task::TTS_FINAL_AUDIO);
    build_final_audio(bins, &subtitles, &audio_paths, &final_audio).await?;

    param.tts_result_file_path = final_audio.clone();

    // Replace or add audio track to video
    if !param.input_video_path.is_empty() && Path::new(&param.input_video_path).exists() {
        let output_video = format!("{}/{}", param.task_base_path, task::VIDEO_WITH_TTS);

        if param.multi_track_audio {
            // Add as second audio track with language metadata
            let orig_lang = cli_art::lang_to_iso639_2(&param.origin_language);
            let target_lang = cli_art::lang_to_iso639_2(&param.target_language);
            tracing::info!(
                "   🔊 Adding audio track: {} ({}) → {} ({})",
                cli_art::lang_display_name(&param.origin_language), orig_lang,
                cli_art::lang_display_name(&param.target_language), target_lang,
            );
            crate::util::video::add_audio_track(
                &bins.ffmpeg,
                Path::new(&param.input_video_path),
                Path::new(&final_audio),
                Path::new(&output_video),
                orig_lang,
                target_lang,
            )
            .await?;
            tracing::info!("   🎬 Multi-track video: track 0 = {} (original), track 1 = {} (dubbed)", orig_lang, target_lang);
        } else {
            tracing::info!(
                "   🔊 Replacing audio track with {} dubbed audio",
                cli_art::lang_display_name(&param.target_language),
            );
            crate::util::video::replace_audio(
                &bins.ffmpeg,
                Path::new(&param.input_video_path),
                Path::new(&final_audio),
                Path::new(&output_video),
            )
            .await?;
        }
        param.video_with_tts_file_path = output_video;
        cli_art::step_tts_done();
    } else {
        cli_art::step_tts_done();
        tracing::info!("   ℹ️  TTS audio generated (no video to merge with)");
    }

    Ok(())
}

/// Build the final concatenated audio with correct timing.
/// Uses a duration cache to avoid duplicate ffprobe calls.
async fn build_final_audio(
    bins: &BinPaths,
    subtitles: &[SrtSentenceWithStrTime],
    audio_paths: &[Option<String>],
    output: &str,
) -> anyhow::Result<()> {
    let output_dir = Path::new(output).parent().unwrap();
    let concat_list = output_dir.join("concat_list.txt");
    let mut list_content = String::new();

    // Duration cache to avoid redundant ffprobe calls
    let mut duration_cache: HashMap<String, f64> = HashMap::new();

    for (i, sub) in subtitles.iter().enumerate() {
        let start = parse_timestamp(&sub.start).unwrap_or(0.0);
        let end = parse_timestamp(&sub.end).unwrap_or(start + 1.0);
        let target_duration = end - start;

        // Determine the audio file for this segment
        let segment_file = if let Some(path) = &audio_paths[i] {
            if Path::new(path).exists() {
                // Get actual duration (cached)
                let actual = get_cached_duration(&bins.ffprobe, path, &mut duration_cache).await;

                if actual > target_duration * 1.1 {
                    // Audio too long — speed up with atempo
                    let ratio = actual / target_duration;
                    let adjusted = output_dir.join(format!("subtitle_{i}_adjusted.wav"));
                    let adjusted_str = adjusted.to_string_lossy().to_string();
                    let atempo = ratio.min(4.0);
                    let atempo_str = format!("atempo={atempo}");

                    if cmd::run_cmd_status(
                        &bins.ffmpeg,
                        &["-y", "-i", path, "-af", &atempo_str, &adjusted_str],
                    )
                    .await
                    .is_ok()
                    {
                        adjusted_str
                    } else {
                        path.clone()
                    }
                } else {
                    path.clone()
                }
            } else {
                generate_silence(bins, output_dir, i, target_duration).await?
            }
        } else {
            generate_silence(bins, output_dir, i, target_duration).await?
        };

        // Add gap (silence) before the first subtitle or between subtitles
        if i == 0 {
            let gap_start = parse_timestamp(&subtitles[0].start).unwrap_or(0.0);
            if gap_start > 0.1 {
                let gap_file = generate_silence(bins, output_dir, 9000 + i, gap_start).await?;
                list_content.push_str(&format!("file '{}'\n", gap_file));
            }
        }

        list_content.push_str(&format!("file '{}'\n", segment_file));

        // Add silence padding if audio is shorter than target duration
        if let Some(path) = &audio_paths[i] {
            if Path::new(path).exists() {
                let actual = get_cached_duration(&bins.ffprobe, path, &mut duration_cache).await;
                let pad = target_duration - actual;
                if pad > 0.05 {
                    let pad_file = generate_silence(bins, output_dir, 8000 + i, pad).await?;
                    list_content.push_str(&format!("file '{}'\n", pad_file));
                }
            }
        }
    }

    tokio::fs::write(&concat_list, &list_content).await?;

    // Concatenate all files
    let concat_str = concat_list.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 path: {}", concat_list.display()))?;

    cmd::run_cmd_status(&bins.ffmpeg, &[
        "-y",
        "-f", "concat",
        "-safe", "0",
        "-i", concat_str,
        "-c", "copy",
        output,
    ]).await
}

/// Get duration with caching
async fn get_cached_duration(
    ffprobe: &str,
    path: &str,
    cache: &mut HashMap<String, f64>,
) -> f64 {
    if let Some(&d) = cache.get(path) {
        return d;
    }
    let d = crate::util::audio::get_duration(ffprobe, Path::new(path))
        .await
        .unwrap_or(0.0);
    cache.insert(path.to_string(), d);
    d
}

/// Generate a silence WAV file
async fn generate_silence(
    bins: &BinPaths,
    dir: &Path,
    index: usize,
    duration: f64,
) -> anyhow::Result<String> {
    let path = dir.join(format!("silence_{index}.wav"));
    let path_str = path.to_string_lossy().to_string();
    let dur_str = format!("{duration}");

    cmd::run_cmd_status(&bins.ffmpeg, &[
        "-y",
        "-f", "lavfi",
        "-i", "anullsrc=r=44100:cl=stereo",
        "-t", &dur_str,
        "-q:a", "9",
        &path_str,
    ]).await?;

    Ok(path_str)
}
