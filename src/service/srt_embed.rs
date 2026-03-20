use crate::storage::BinPaths;
use crate::types::ass::{ASS_HEADER_HORIZONTAL, ASS_HEADER_VERTICAL};
use crate::types::subtitle::SrtSentenceWithStrTime;
use crate::types::task::{self, EmbedVideoType, StepParam, SubtitleResultType};
use crate::util::srt::parse_srt;
use std::path::Path;
use std::process::Stdio;

/// Step 4: Embed subtitles into video
pub async fn embed_subtitles(
    bins: &BinPaths,
    param: &mut StepParam,
) -> anyhow::Result<()> {
    if param.embed_subtitle_video_type == EmbedVideoType::None {
        tracing::info!("Subtitle embedding disabled, skipping step 4");
        return Ok(());
    }

    // Determine the video source (with TTS audio or original)
    let video_source = if !param.video_with_tts_file_path.is_empty()
        && Path::new(&param.video_with_tts_file_path).exists()
    {
        &param.video_with_tts_file_path
    } else if !param.input_video_path.is_empty()
        && Path::new(&param.input_video_path).exists()
    {
        &param.input_video_path
    } else {
        tracing::warn!("No video file available for subtitle embedding");
        return Ok(());
    };

    let srt_content = tokio::fs::read_to_string(&param.bilingual_srt_file_path).await?;
    let subtitles = parse_srt(&srt_content);

    match param.embed_subtitle_video_type {
        EmbedVideoType::Horizontal => {
            embed_horizontal(bins, param, video_source, &subtitles).await?;
        }
        EmbedVideoType::Vertical => {
            embed_vertical(bins, param, video_source, &subtitles).await?;
        }
        EmbedVideoType::All => {
            embed_horizontal(bins, param, video_source, &subtitles).await?;
            embed_vertical(bins, param, video_source, &subtitles).await?;
        }
        EmbedVideoType::None => {}
    }

    tracing::info!("Step 4 complete: subtitles embedded into video");
    Ok(())
}

async fn embed_horizontal(
    bins: &BinPaths,
    param: &StepParam,
    video: &str,
    subtitles: &[SrtSentenceWithStrTime],
) -> anyhow::Result<()> {
    let ass_path = format!("{}/horizontal.ass", param.task_base_path);
    let output = format!("{}/{}", param.output_dir(), task::HORIZONTAL_EMBED);

    // Generate ASS file
    let ass_content = generate_ass(subtitles, param, true);
    tokio::fs::write(&ass_path, &ass_content).await?;

    // Burn subtitles
    burn_ass_into_video(bins, video, &ass_path, &output).await
}

async fn embed_vertical(
    bins: &BinPaths,
    param: &StepParam,
    video: &str,
    subtitles: &[SrtSentenceWithStrTime],
) -> anyhow::Result<()> {
    let output_dir = param.output_dir();

    // First convert to vertical if source is horizontal
    let (w, h) = crate::util::video::get_resolution(&bins.ffprobe, Path::new(video)).await?;
    let vertical_video = if w > h {
        let vert = format!("{}/vertical_base.mp4", param.task_base_path);
        convert_to_vertical(bins, video, &vert).await?;
        vert
    } else {
        video.to_string()
    };

    let ass_path = format!("{}/vertical.ass", param.task_base_path);
    let output = format!("{output_dir}/{}", task::VERTICAL_EMBED);

    let mut ass_content = generate_ass(subtitles, param, false);

    // Add title overlays for vertical video
    if !param.vertical_video_major_title.is_empty() {
        let duration = crate::util::audio::get_duration(&bins.ffprobe, Path::new(&vertical_video))
            .await
            .unwrap_or(0.0);
        let end_ts = format_ass_time(duration);
        ass_content.push_str(&format!(
            "Dialogue: 0,0:00:00.00,{end_ts},Title,,0,0,0,,{}\n",
            param.vertical_video_major_title
        ));
        if !param.vertical_video_minor_title.is_empty() {
            ass_content.push_str(&format!(
                "Dialogue: 0,0:00:00.00,{end_ts},SubTitle,,0,0,0,,{}\n",
                param.vertical_video_minor_title
            ));
        }
    }

    tokio::fs::write(&ass_path, &ass_content).await?;
    burn_ass_into_video(bins, &vertical_video, &ass_path, &output).await
}

fn generate_ass(
    subtitles: &[SrtSentenceWithStrTime],
    param: &StepParam,
    horizontal: bool,
) -> String {
    let header = if horizontal {
        ASS_HEADER_HORIZONTAL
    } else {
        ASS_HEADER_VERTICAL
    };

    let mut content = header.to_string();

    for sub in subtitles {
        let start = format_srt_to_ass_time(&sub.start);
        let end = format_srt_to_ass_time(&sub.end);

        // Split text into lines (bilingual has two lines)
        let lines: Vec<&str> = sub.text.lines().collect();

        match param.subtitle_result_type {
            SubtitleResultType::BilingualTranslationOnTop
            | SubtitleResultType::BilingualTranslationOnBottom => {
                if lines.len() >= 2 {
                    // Major style for primary language
                    let major_text = split_long_text(lines[0], param.max_word_one_line);
                    content.push_str(&format!(
                        "Dialogue: 0,{start},{end},Major,,0,0,0,,{major_text}\n"
                    ));
                    // Minor style for secondary language
                    let minor_text = split_long_text(lines[1], param.max_word_one_line);
                    content.push_str(&format!(
                        "Dialogue: 0,{start},{end},Minor,,0,0,0,,{minor_text}\n"
                    ));
                } else if !lines.is_empty() {
                    let text = split_long_text(lines[0], param.max_word_one_line);
                    content.push_str(&format!(
                        "Dialogue: 0,{start},{end},Major,,0,0,0,,{text}\n"
                    ));
                }
            }
            _ => {
                if !lines.is_empty() {
                    let text = split_long_text(lines[0], param.max_word_one_line);
                    content.push_str(&format!(
                        "Dialogue: 0,{start},{end},Major,,0,0,0,,{text}\n"
                    ));
                }
            }
        }
    }

    content
}

/// Convert SRT timestamp (HH:MM:SS,mmm) to ASS format (H:MM:SS.cc)
fn format_srt_to_ass_time(srt_time: &str) -> String {
    let cleaned = srt_time.replace(',', ".");
    if let Some((hms, frac)) = cleaned.split_once('.') {
        let centis: u32 = frac.parse::<u32>().unwrap_or(0) / 10;
        let parts: Vec<&str> = hms.split(':').collect();
        if parts.len() == 3 {
            let h: u32 = parts[0].parse().unwrap_or(0);
            return format!("{}:{:02}:{:02}.{:02}", h,
                parts[1].parse::<u32>().unwrap_or(0),
                parts[2].parse::<u32>().unwrap_or(0),
                centis);
        }
    }
    srt_time.to_string()
}

fn format_ass_time(seconds: f64) -> String {
    let total_cs = (seconds * 100.0) as u64;
    let h = total_cs / 360000;
    let m = (total_cs % 360000) / 6000;
    let s = (total_cs % 6000) / 100;
    let cs = total_cs % 100;
    format!("{h}:{m:02}:{s:02}.{cs:02}")
}

fn split_long_text(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return text.to_string();
    }
    // Split roughly in the middle
    let mid = words.len() / 2;
    let line1 = words[..mid].join(" ");
    let line2 = words[mid..].join(" ");
    format!("{}\\N{}", line1, line2)
}

async fn burn_ass_into_video(
    bins: &BinPaths,
    video: &str,
    ass_path: &str,
    output: &str,
) -> anyhow::Result<()> {
    let vf = format!("ass={ass_path}");
    let status = tokio::process::Command::new(&bins.ffmpeg)
        .args(["-y", "-i", video, "-vf", &vf, output])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("Failed to burn subtitles into video");
    }
    Ok(())
}

async fn convert_to_vertical(
    bins: &BinPaths,
    input: &str,
    output: &str,
) -> anyhow::Result<()> {
    let status = tokio::process::Command::new(&bins.ffmpeg)
        .args([
            "-y", "-i", input,
            "-vf", "scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2",
            output,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("Failed to convert video to vertical");
    }
    Ok(())
}
