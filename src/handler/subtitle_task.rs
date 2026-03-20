use crate::dto::{
    ApiResponse, GetTaskRequest, GetTaskResponse, StartTaskRequest, StartTaskResponse,
    SubtitleInfoDto, VideoInfo,
};
use crate::types::task::{EmbedVideoType, SubtitleResultType, SubtitleTask};
use crate::util::cli_art;
use crate::AppState;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn start_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartTaskRequest>,
) -> impl IntoResponse {
    // Generate task ID
    let task_id = format!(
        "{}_{}",
        sanitize_for_id(&req.url),
        &uuid::Uuid::new_v4().to_string()[..8]
    );

    // Determine subtitle result type
    let subtitle_result_type = match (req.bilingual, req.translation_subtitle_pos) {
        (1, 1) => SubtitleResultType::BilingualTranslationOnTop,
        (1, _) => SubtitleResultType::BilingualTranslationOnBottom,
        (_, _) if !req.target_lang.is_empty() => SubtitleResultType::TargetOnly,
        _ => SubtitleResultType::OriginOnly,
    };

    // Parse replace words
    let replace_words_map: HashMap<String, String> = req
        .replace
        .iter()
        .filter_map(|s| {
            let parts: Vec<&str> = s.splitn(2, '|').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    // Create task directory
    let task_base_path = format!("./tasks/{task_id}");
    let output_dir = format!("{task_base_path}/output");
    if let Err(e) = tokio::fs::create_dir_all(&output_dir).await {
        return ApiResponse::<()>::error(&format!("Failed to create task directory: {e}"))
            .into_response();
    }

    // Auto-detect language mode: when origin_language is empty, use "auto"
    let config = state.config.read().await;
    let origin_language = if req.origin_language.is_empty() && config.app.auto_detect_language {
        "auto".to_string()
    } else if req.origin_language.is_empty() {
        "en".to_string()
    } else {
        req.origin_language.clone()
    };

    // Auto target language: when target is empty, will be determined after detection
    let target_language = if req.target_lang.is_empty() && config.app.auto_detect_language {
        "auto".to_string()
    } else if req.target_lang.is_empty() {
        config.app.default_target_language.clone()
    } else {
        req.target_lang.clone()
    };

    // Adjust subtitle result type for auto mode — default to bilingual
    let subtitle_result_type = if origin_language == "auto" && subtitle_result_type == SubtitleResultType::OriginOnly {
        SubtitleResultType::BilingualTranslationOnBottom
    } else {
        subtitle_result_type
    };
    drop(config);

    // Create and store task
    let task = SubtitleTask::new(
        task_id.clone(),
        req.url.clone(),
        origin_language.clone(),
        target_language.clone(),
    );
    state.task_store.insert(task);

    // Build step params
    let embed_type = EmbedVideoType::from(req.embed_subtitle_video_type.as_str());
    let max_word = if req.origin_language_word_one_line > 0 {
        req.origin_language_word_one_line
    } else {
        12
    };

    let step_param = crate::types::task::StepParam {
        task_id: task_id.clone(),
        task_base_path,
        link: req.url,
        audio_file_path: String::new(),
        input_video_path: String::new(),
        video_with_tts_file_path: String::new(),
        subtitle_result_type,
        enable_modal_filter: req.modal_filter == 1,
        enable_tts: req.tts == 1,
        tts_voice_code: req.tts_voice_code,
        voice_clone_audio_url: req.tts_voice_clone_src_file_url,
        origin_language,
        target_language,
        user_ui_language: req.language,
        replace_words_map,
        bilingual_srt_file_path: String::new(),
        short_origin_mixed_srt_file_path: String::new(),
        tts_source_file_path: String::new(),
        tts_result_file_path: String::new(),
        embed_subtitle_video_type: embed_type,
        vertical_video_major_title: req.vertical_major_title,
        vertical_video_minor_title: req.vertical_minor_title,
        max_word_one_line: max_word,
        subtitle_infos: Vec::new(),
        multi_track_audio: req.multi_track,
        detected_language: String::new(),
    };

    // Snapshot state once before spawning — zero lock contention during pipeline
    let state_clone = state.clone();
    let task_id_for_spawn = task_id.clone();
    tokio::spawn(async move {
        let tid = task_id_for_spawn;
        if let Err(e) = run_pipeline(state_clone.clone(), step_param).await {
            cli_art::pipeline_failed(&tid, &e.to_string());
            state_clone.task_store.update(&tid, |t| {
                t.set_failed(e.to_string());
            });
        }
    });

    ApiResponse::success(StartTaskResponse { task_id }).into_response()
}

pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Query(req): Query<GetTaskRequest>,
) -> impl IntoResponse {
    let Some(task) = state.task_store.get(&req.task_id) else {
        return ApiResponse::<()>::error("Task not found").into_response();
    };

    let video_info = if !task.title.is_empty() || !task.description.is_empty() {
        Some(VideoInfo {
            title: task.title.clone(),
            description: task.description.clone(),
            translated_title: task.translated_title.clone(),
            translated_description: task.translated_description.clone(),
            language: task.origin_language.clone(),
        })
    } else {
        None
    };

    let subtitle_info: Vec<SubtitleInfoDto> = task
        .subtitle_infos
        .iter()
        .map(|s| SubtitleInfoDto {
            name: s.name.clone(),
            download_url: s.download_url.clone(),
        })
        .collect();

    ApiResponse::success(GetTaskResponse {
        task_id: task.task_id.clone(),
        process_percent: task.process_pct,
        video_info,
        subtitle_info,
        target_language: task.target_language.clone(),
        speech_download_url: task.speech_download_url.clone(),
    })
    .into_response()
}

/// Run the full dubbing pipeline
async fn run_pipeline(
    state: Arc<AppState>,
    mut param: crate::types::task::StepParam,
) -> anyhow::Result<()> {
    let task_id = param.task_id.clone();

    // Snapshot config and bins once — zero lock contention during pipeline
    let config = state.config.read().await.clone();
    let bins = state.bin_paths.read().await.clone();
    let service = state.service.read().await;
    let transcriber = service.transcriber.clone();
    let chat_completer = service.chat_completer.clone();
    let tts_client = service.tts_client.clone();
    drop(service);

    state.task_store.update(&task_id, |t| t.set_progress(3));

    // Step 1: Download/extract audio
    cli_art::step_download_start(&param.link);
    crate::service::link_to_file::link_to_file(&bins, &mut param, &config.app.proxy).await?;
    cli_art::step_download_done();
    state.task_store.update(&task_id, |t| t.set_progress(10));

    // Step 2: Transcribe + translate → SRT (with auto language detection)
    cli_art::step_transcribe_start(config.transcribe.provider.as_str(), &param.origin_language);
    crate::service::audio_to_subtitle::audio_to_subtitle(
        &bins,
        &config,
        &transcriber,
        &chat_completer,
        &mut param,
    )
    .await?;
    state.task_store.update(&task_id, |t| {
        t.set_progress(90);
        // Update detected language info on the task
        if !param.detected_language.is_empty() {
            t.origin_language = param.origin_language.clone();
            t.target_language = param.target_language.clone();
        }
    });

    // Step 3: TTS dubbing
    if param.enable_tts {
        cli_art::step_tts_start(config.tts.provider.as_str(), &param.tts_voice_code);
    }
    crate::service::srt_to_speech::srt_to_speech(
        &bins,
        &config,
        &tts_client,
        &mut param,
    )
    .await?;
    state.task_store.update(&task_id, |t| t.set_progress(95));

    // Step 4: Embed subtitles into video
    if param.embed_subtitle_video_type != crate::types::task::EmbedVideoType::None {
        cli_art::step_embed_start(&format!("{:?}", param.embed_subtitle_video_type));
    }
    crate::service::srt_embed::embed_subtitles(&bins, &mut param).await?;
    state.task_store.update(&task_id, |t| t.set_progress(98));

    // Step 5: Finalize
    cli_art::step_finalize_start();
    crate::service::upload_subtitles::upload_subtitles(&mut param).await?;
    cli_art::step_finalize_done(param.subtitle_infos.len());

    // Update task with final results
    state.task_store.update(&task_id, |t| {
        t.subtitle_infos = param.subtitle_infos.clone();
        t.speech_download_url = param.tts_result_file_path.clone();
        t.set_success();
    });

    cli_art::pipeline_success(&task_id);
    Ok(())
}

fn sanitize_for_id(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(50)
        .collect()
}
