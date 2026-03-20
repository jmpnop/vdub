use crate::types::task::{self, StepParam, SubtitleInfo};
use std::path::Path;

/// Step 5: Finalize task — apply word replacements, set download URLs
pub async fn upload_subtitles(param: &mut StepParam) -> anyhow::Result<()> {
    let output_dir = param.output_dir();

    // Apply word replacements if configured
    if !param.replace_words_map.is_empty() {
        apply_replacements(&output_dir, &param.replace_words_map).await?;
    }

    // Build subtitle info list
    let mut infos = Vec::new();

    let bilingual_path = format!("{output_dir}/{}", task::BILINGUAL_SRT_FILE);
    if Path::new(&bilingual_path).exists() {
        infos.push(SubtitleInfo {
            name: "Bilingual Subtitles".to_string(),
            download_url: format!("/api/file/{bilingual_path}"),
        });
    }

    let origin_path = format!("{output_dir}/{}", task::ORIGIN_LANG_SRT_FILE);
    if Path::new(&origin_path).exists() {
        infos.push(SubtitleInfo {
            name: "Original Language Subtitles".to_string(),
            download_url: format!("/api/file/{origin_path}"),
        });
    }

    let target_path = format!("{output_dir}/{}", task::TARGET_LANG_SRT_FILE);
    if Path::new(&target_path).exists() {
        infos.push(SubtitleInfo {
            name: "Translated Subtitles".to_string(),
            download_url: format!("/api/file/{target_path}"),
        });
    }

    // Video files
    let h_embed = format!("{output_dir}/{}", task::HORIZONTAL_EMBED);
    if Path::new(&h_embed).exists() {
        infos.push(SubtitleInfo {
            name: "Horizontal Video with Subtitles".to_string(),
            download_url: format!("/api/file/{h_embed}"),
        });
    }

    let v_embed = format!("{output_dir}/{}", task::VERTICAL_EMBED);
    if Path::new(&v_embed).exists() {
        infos.push(SubtitleInfo {
            name: "Vertical Video with Subtitles".to_string(),
            download_url: format!("/api/file/{v_embed}"),
        });
    }

    // TTS result
    let tts_path = format!("{}/{}", param.task_base_path, task::TTS_FINAL_AUDIO);
    let speech_url = if Path::new(&tts_path).exists() {
        format!("/api/file/{tts_path}")
    } else {
        String::new()
    };

    // Video with TTS
    let tts_video = format!("{}/{}", param.task_base_path, task::VIDEO_WITH_TTS);
    if Path::new(&tts_video).exists() {
        infos.push(SubtitleInfo {
            name: "Video with Dubbed Audio".to_string(),
            download_url: format!("/api/file/{tts_video}"),
        });
    }

    param.subtitle_infos = infos;
    param.tts_result_file_path = speech_url;

    tracing::info!("Step 5 complete: {} output files ready", param.subtitle_infos.len());
    Ok(())
}

async fn apply_replacements(
    output_dir: &str,
    replacements: &std::collections::HashMap<String, String>,
) -> anyhow::Result<()> {
    let srt_files = [
        task::BILINGUAL_SRT_FILE,
        task::ORIGIN_LANG_SRT_FILE,
        task::TARGET_LANG_SRT_FILE,
    ];

    for file_name in &srt_files {
        let path = format!("{output_dir}/{file_name}");
        if Path::new(&path).exists() {
            let mut content = tokio::fs::read_to_string(&path).await?;
            for (from, to) in replacements {
                content = content.replace(from, to);
            }
            tokio::fs::write(&path, content).await?;
        }
    }

    Ok(())
}
