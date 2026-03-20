use crate::provider::Ttser;
use crate::storage::BinPaths;
use crate::types::subtitle::{SrtSentenceWithStrTime, parse_timestamp};
use crate::types::task::{self, StepParam};
use crate::util::srt::parse_srt;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_TTS_CONCURRENT: usize = 3;
const MAX_FAILURE_RATIO: f64 = 0.5;

/// Step 3: Generate TTS audio for each subtitle and replace video audio
pub async fn srt_to_speech(
    bins: &BinPaths,
    tts_client: &Arc<dyn Ttser>,
    param: &mut StepParam,
) -> anyhow::Result<()> {
    if !param.enable_tts {
        tracing::info!("TTS disabled, skipping step 3");
        return Ok(());
    }

    // Parse SRT file
    let srt_content = tokio::fs::read_to_string(&param.tts_source_file_path).await?;
    let subtitles = parse_srt(&srt_content);

    if subtitles.is_empty() {
        tracing::warn!("No subtitles found for TTS");
        return Ok(());
    }

    tracing::info!("Generating TTS for {} subtitle entries", subtitles.len());

    // Generate TTS for each subtitle concurrently
    let sem = Arc::new(Semaphore::new(MAX_TTS_CONCURRENT));
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
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await? {
            Ok(path) => audio_paths[i] = Some(path),
            Err(e) => {
                tracing::warn!("TTS failed for subtitle {i}: {e}");
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

    // Replace video audio if video exists
    if !param.input_video_path.is_empty() && Path::new(&param.input_video_path).exists() {
        let output_video = format!("{}/{}", param.task_base_path, task::VIDEO_WITH_TTS);
        crate::util::video::replace_audio(
            &bins.ffmpeg,
            Path::new(&param.input_video_path),
            Path::new(&final_audio),
            Path::new(&output_video),
        )
        .await?;
        param.video_with_tts_file_path = output_video;
        tracing::info!("Step 3 complete: video with TTS audio created");
    } else {
        tracing::info!("Step 3 complete: TTS audio generated (no video to replace)");
    }

    Ok(())
}

/// Build the final concatenated audio with correct timing
async fn build_final_audio(
    bins: &BinPaths,
    subtitles: &[SrtSentenceWithStrTime],
    audio_paths: &[Option<String>],
    output: &str,
) -> anyhow::Result<()> {
    let output_dir = Path::new(output).parent().unwrap();
    let concat_list = output_dir.join("concat_list.txt");
    let mut list_content = String::new();

    for (i, sub) in subtitles.iter().enumerate() {
        let start = parse_timestamp(&sub.start).unwrap_or(0.0);
        let end = parse_timestamp(&sub.end).unwrap_or(start + 1.0);
        let target_duration = end - start;

        // Determine the audio file for this segment
        let segment_file = if let Some(path) = &audio_paths[i] {
            if Path::new(path).exists() {
                // Get actual duration and adjust if needed
                let actual = crate::util::audio::get_duration(&bins.ffprobe, Path::new(path))
                    .await
                    .unwrap_or(target_duration);

                if actual > target_duration * 1.1 {
                    // Audio too long — speed up with atempo
                    let ratio = actual / target_duration;
                    let adjusted = output_dir.join(format!("subtitle_{i}_adjusted.wav"));
                    let atempo = ratio.min(4.0); // atempo max is 4.0
                    let status = tokio::process::Command::new(&bins.ffmpeg)
                        .args([
                            "-y",
                            "-i", path,
                            "-af", &format!("atempo={atempo}"),
                            adjusted.to_str().unwrap(),
                        ])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .await?;
                    if status.success() {
                        adjusted.to_string_lossy().to_string()
                    } else {
                        path.clone()
                    }
                } else {
                    path.clone()
                }
            } else {
                // File missing, generate silence
                generate_silence(bins, output_dir, i, target_duration).await?
            }
        } else {
            // TTS failed, generate silence
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
                let actual = crate::util::audio::get_duration(&bins.ffprobe, Path::new(path))
                    .await
                    .unwrap_or(0.0);
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
    let status = tokio::process::Command::new(&bins.ffmpeg)
        .args([
            "-y",
            "-f", "concat",
            "-safe", "0",
            "-i", concat_list.to_str().unwrap(),
            "-c", "copy",
            output,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("Failed to concatenate TTS audio files");
    }

    Ok(())
}

/// Generate a silence WAV file
async fn generate_silence(
    bins: &BinPaths,
    dir: &Path,
    index: usize,
    duration: f64,
) -> anyhow::Result<String> {
    let path = dir.join(format!("silence_{index}.wav"));
    let status = tokio::process::Command::new(&bins.ffmpeg)
        .args([
            "-y",
            "-f", "lavfi",
            "-i", &format!("anullsrc=r=44100:cl=stereo"),
            "-t", &format!("{duration}"),
            "-q:a", "9",
            path.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("Failed to generate silence");
    }

    Ok(path.to_string_lossy().to_string())
}
