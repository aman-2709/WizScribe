pub mod audio;
pub mod db;
pub mod whisper;
pub mod ai;
pub mod config;
pub mod dual_audio;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;

use dual_audio::{DualAudioRecorder, DualRecordingStatus, DualRecordingResult, SpeakerTranscript, SpeakerSegment};

// Application state
pub struct AppState {
    pub db: Arc<Mutex<Option<db::Database>>>,
    pub audio: Arc<Mutex<audio::AudioRecorder>>,
    pub dual_audio: Arc<Mutex<DualAudioRecorder>>,
    pub whisper: Arc<Mutex<whisper::WhisperTranscriber>>,
    pub ai: Arc<Mutex<ai::AIClient>>,
    pub config: Arc<Mutex<config::AppConfig>>,
}

// ===== Meeting Commands =====

#[tauri::command]
async fn create_meeting(state: tauri::State<'_, AppState>, title: String) -> Result<db::Meeting, String> {
    let db = state.db.lock().await;
    let db = db.as_ref().ok_or("Database not initialized")?;
    db.create_meeting(&title).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_meeting(state: tauri::State<'_, AppState>, id: String) -> Result<Option<db::Meeting>, String> {
    let db = state.db.lock().await;
    let db = db.as_ref().ok_or("Database not initialized")?;
    db.get_meeting(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_meetings(state: tauri::State<'_, AppState>) -> Result<Vec<db::Meeting>, String> {
    let db = state.db.lock().await;
    let db = db.as_ref().ok_or("Database not initialized")?;
    db.list_meetings().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_meeting(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().await;
    let db = db.as_ref().ok_or("Database not initialized")?;
    db.delete_meeting(&id).await.map_err(|e| e.to_string())
}

// ===== Audio Commands =====

#[tauri::command]
async fn start_recording(state: tauri::State<'_, AppState>, meeting_id: String) -> Result<String, String> {
    let mut audio = state.audio.lock().await;
    audio.start_recording(&meeting_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_recording(state: tauri::State<'_, AppState>) -> Result<(String, u64), String> {
    let (meeting_id, duration) = {
        let mut audio = state.audio.lock().await;
        audio.stop_recording().await.map_err(|e| e.to_string())?
    };

    // Get the audio path
    let app_data_dir = get_app_data_dir();
    let audio_path = app_data_dir.join("audio").join(format!("{}.wav", meeting_id));
    let audio_path_str = audio_path.to_string_lossy().to_string();

    // Update meeting with audio path and duration
    let db = state.db.lock().await;
    if let Some(db) = db.as_ref() {
        if let Err(e) = db.update_meeting_audio(&meeting_id, &audio_path_str, duration as i64).await {
            eprintln!("Failed to update meeting audio: {}", e);
        }
    }

    Ok((meeting_id, duration))
}

#[tauri::command]
async fn pause_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut audio = state.audio.lock().await;
    audio.pause_recording().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn resume_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut audio = state.audio.lock().await;
    audio.resume_recording().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_recording_state(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    // Check dual audio state first
    let dual_audio = state.dual_audio.lock().await;
    let dual_status = dual_audio.get_status();

    // If dual audio is recording, return that state
    if dual_status.get("is_recording").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Ok(serde_json::json!({
            "state": "recording",
            "meeting_id": dual_status.get("meeting_id"),
            "is_dual_mode": true,
            "mic_active": dual_status.get("mic_active"),
            "system_active": dual_status.get("system_active"),
            "mic_device": dual_status.get("mic_device"),
            "system_device": dual_status.get("system_device"),
        }));
    }
    drop(dual_audio);

    // Fall back to legacy single audio state
    let audio = state.audio.lock().await;
    let legacy_state = audio.get_state();
    let state_str = legacy_state.get("state").and_then(|v| v.as_str()).unwrap_or("idle");

    Ok(serde_json::json!({
        "state": state_str,
        "meeting_id": legacy_state.get("meeting_id"),
        "is_dual_mode": false,
        "mic_active": state_str == "recording",
        "system_active": false,
        "mic_device": null,
        "system_device": null,
    }))
}

#[tauri::command]
async fn list_audio_devices() -> Result<Vec<audio::AudioDevice>, String> {
    audio::list_audio_devices().map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_recording_device(
    state: tauri::State<'_, AppState>,
    device_index: Option<usize>,
) -> Result<(), String> {
    let mut audio = state.audio.lock().await;
    audio.set_device(device_index);

    // Persist to config
    let mut config = state.config.lock().await;
    config.selected_audio_device = device_index;
    config.save().await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_selected_audio_device(state: tauri::State<'_, AppState>) -> Result<Option<usize>, String> {
    let audio = state.audio.lock().await;
    Ok(audio.get_selected_device())
}

// ===== Dual Audio Commands =====

#[tauri::command]
async fn start_dual_recording(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<DualRecordingStatus, String> {
    let config = state.config.lock().await;
    let mic_device_index = config.mic_device_index;
    let system_device_index = config.system_device_index;
    drop(config);

    let mut dual_audio = state.dual_audio.lock().await;
    dual_audio.start(&meeting_id, mic_device_index, system_device_index)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_dual_recording(
    state: tauri::State<'_, AppState>,
) -> Result<DualRecordingResult, String> {
    let result = {
        let mut dual_audio = state.dual_audio.lock().await;
        dual_audio.stop().await.map_err(|e| e.to_string())?
    };

    // Get the audio path
    let app_data_dir = get_app_data_dir();
    let audio_path = app_data_dir.join("audio").join(format!("{}.wav", result.meeting_id));
    let audio_path_str = audio_path.to_string_lossy().to_string();

    // Update meeting with audio path and duration
    let db = state.db.lock().await;
    if let Some(db) = db.as_ref() {
        if let Err(e) = db.update_meeting_audio(&result.meeting_id, &audio_path_str, result.duration_secs as i64).await {
            eprintln!("Failed to update meeting audio: {}", e);
        }
    }

    Ok(result)
}

#[tauri::command]
async fn get_dual_audio_config(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().await;
    Ok(serde_json::json!({
        "mic_device_index": config.mic_device_index,
        "system_device_index": config.system_device_index,
    }))
}

#[tauri::command]
async fn set_dual_audio_config(
    state: tauri::State<'_, AppState>,
    mic_device_index: Option<usize>,
    system_device_index: Option<usize>,
) -> Result<(), String> {
    // Validate devices if provided
    let devices = audio::list_audio_devices().map_err(|e| e.to_string())?;

    if let Some(mic_idx) = mic_device_index {
        if mic_idx >= devices.len() {
            return Err("Invalid mic device index".to_string());
        }
    }

    if let Some(system_idx) = system_device_index {
        if system_idx >= devices.len() {
            return Err("Invalid system device index".to_string());
        }
        // Verify it's a monitor source
        if !devices[system_idx].is_monitor {
            return Err("Device is not a monitor source".to_string());
        }
    }

    // Update config
    let mut config = state.config.lock().await;
    config.mic_device_index = mic_device_index;
    config.system_device_index = system_device_index;
    config.save().await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_audio_devices_by_type(
    device_type: String,
) -> Result<Vec<audio::AudioDevice>, String> {
    let devices = audio::list_audio_devices().map_err(|e| e.to_string())?;

    let filtered: Vec<audio::AudioDevice> = match device_type.as_str() {
        "microphone" => devices.into_iter().filter(|d| !d.is_monitor).collect(),
        "monitor" => devices.into_iter().filter(|d| d.is_monitor).collect(),
        _ => return Err("Invalid device type. Use 'microphone' or 'monitor'".to_string()),
    };

    Ok(filtered)
}

#[tauri::command]
async fn transcribe_dual_audio(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    audio_path: String,
) -> Result<SpeakerTranscript, String> {
    // Check if file exists
    if !std::path::Path::new(&audio_path).exists() {
        return Err("Audio file not found".to_string());
    }

    // Check if stereo
    if !whisper::is_stereo_file(&audio_path) {
        return Err("Not a stereo file".to_string());
    }

    // Split stereo into mono channels
    let (left_path, right_path) = whisper::split_stereo_channels(&audio_path)
        .map_err(|e| format!("Failed to split channels: {}", e))?;

    let whisper = state.whisper.lock().await;

    // Transcribe both channels (sequentially for now, could be parallelized)
    let left_transcript = whisper.transcribe(left_path.to_string_lossy().as_ref())
        .await
        .map_err(|e| format!("Failed to transcribe mic channel: {}", e))?;

    let right_transcript = whisper.transcribe(right_path.to_string_lossy().as_ref())
        .await
        .map_err(|e| format!("Failed to transcribe system channel: {}", e))?;

    // Parse transcripts into segments
    let mic_segments: Vec<SpeakerSegment> = whisper::parse_transcript_to_segments(&left_transcript)
        .into_iter()
        .map(|s| SpeakerSegment {
            speaker: "Me".to_string(),
            text: s.text,
            start_ms: s.start_ms as u64,
            end_ms: s.end_ms as u64,
            is_overlapping: false,
        })
        .collect();

    let system_segments: Vec<SpeakerSegment> = whisper::parse_transcript_to_segments(&right_transcript)
        .into_iter()
        .map(|s| SpeakerSegment {
            speaker: "Them".to_string(),
            text: s.text,
            start_ms: s.start_ms as u64,
            end_ms: s.end_ms as u64,
            is_overlapping: false,
        })
        .collect();

    // Get device names from config
    let config = state.config.lock().await;
    let devices = audio::list_audio_devices().unwrap_or_default();

    let mic_name = config.mic_device_index
        .and_then(|idx| devices.get(idx))
        .map(|d| d.name.clone())
        .unwrap_or_else(|| "Microphone".to_string());

    let system_name = config.system_device_index
        .and_then(|idx| devices.get(idx))
        .map(|d| d.name.clone())
        .unwrap_or_else(|| "System Audio".to_string());

    // Merge segments with overlap detection
    let merged_segments = SpeakerTranscript::merge(mic_segments, system_segments);

    let transcript = SpeakerTranscript {
        version: 1,
        mic_device: mic_name,
        system_device: system_name,
        has_dual_audio: true,
        segments: merged_segments,
    };

    // Store as JSON in database
    let transcript_json = serde_json::to_string(&transcript)
        .map_err(|e| format!("Failed to serialize transcript: {}", e))?;

    let db = state.db.lock().await;
    if let Some(db) = db.as_ref() {
        let _ = db.update_meeting_transcript(&meeting_id, &transcript_json).await;
    }

    // Cleanup temp files
    let _ = std::fs::remove_file(&left_path);
    let _ = std::fs::remove_file(&right_path);

    Ok(transcript)
}

#[tauri::command]
async fn get_dual_recording_state(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let dual_audio = state.dual_audio.lock().await;
    Ok(dual_audio.get_status())
}

// ===== Transcription Commands =====

#[tauri::command]
async fn transcribe_audio(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    audio_path: String,
) -> Result<String, String> {
    let whisper = state.whisper.lock().await;
    let transcript = whisper.transcribe(&audio_path).await.map_err(|e| e.to_string())?;

    // Update meeting with transcript
    let db = state.db.lock().await;
    if let Some(db) = db.as_ref() {
        let _ = db.update_meeting_transcript(&meeting_id, &transcript).await;
    }

    Ok(transcript)
}

#[tauri::command]
async fn is_whisper_model_available(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let whisper = state.whisper.lock().await;
    Ok(whisper.is_model_available())
}

#[tauri::command]
async fn get_whisper_model_path(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let whisper = state.whisper.lock().await;
    Ok(whisper.get_model_path().to_string())
}

// ===== AI Commands =====

#[tauri::command]
async fn set_ai_api_key(
    state: tauri::State<'_, AppState>,
    api_key: String,
    provider: String,
) -> Result<(), String> {
    // Set in AI client
    let ai = state.ai.lock().await;
    ai.set_api_key(&api_key, &provider).await.map_err(|e| e.to_string())?;

    // Persist to config file
    let mut config = state.config.lock().await;
    config.set_ai_credentials(&api_key, &provider).await.map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_ai_config(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config = state.config.lock().await;
    Ok(serde_json::json!({
        "has_api_key": config.ai_api_key.is_some(),
        "provider": config.ai_provider.clone().unwrap_or_else(|| "openai".to_string())
    }))
}

#[tauri::command]
async fn generate_summary(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    transcript: String,
) -> Result<String, String> {
    let ai = state.ai.lock().await;
    let summary = ai.generate_summary(&transcript).await.map_err(|e| e.to_string())?;

    // Update meeting with summary
    let db = state.db.lock().await;
    if let Some(db) = db.as_ref() {
        let _ = db.update_meeting_summary(&meeting_id, &summary).await;
    }

    Ok(summary)
}

#[tauri::command]
async fn chat_with_ai(
    state: tauri::State<'_, AppState>,
    transcript: String,
    question: String,
) -> Result<String, String> {
    let ai = state.ai.lock().await;
    ai.chat(&transcript, &question).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn extract_action_items(
    state: tauri::State<'_, AppState>,
    transcript: String,
) -> Result<Vec<String>, String> {
    let ai = state.ai.lock().await;
    ai.extract_action_items(&transcript).await.map_err(|e| e.to_string())
}

// ===== Note Commands =====

#[tauri::command]
async fn get_note(state: tauri::State<'_, AppState>, meeting_id: String) -> Result<Option<db::Note>, String> {
    let db = state.db.lock().await;
    let db = db.as_ref().ok_or("Database not initialized")?;
    db.get_note(&meeting_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_note(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    content: String,
    timestamps: Vec<f64>,
) -> Result<(), String> {
    let db = state.db.lock().await;
    let db = db.as_ref().ok_or("Database not initialized")?;
    db.update_note(&meeting_id, &content, timestamps).await.map_err(|e| e.to_string())
}

// ===== Template Commands =====

#[tauri::command]
async fn list_templates(state: tauri::State<'_, AppState>) -> Result<Vec<db::Template>, String> {
    let db = state.db.lock().await;
    let db = db.as_ref().ok_or("Database not initialized")?;
    db.list_templates().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_template(state: tauri::State<'_, AppState>, id: String) -> Result<Option<db::Template>, String> {
    let db = state.db.lock().await;
    let db = db.as_ref().ok_or("Database not initialized")?;
    db.get_template(&id).await.map_err(|e| e.to_string())
}

// ===== App initialization =====

fn get_app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("wizscribe")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_data_dir = get_app_data_dir();

            // Ensure directories exist
            std::fs::create_dir_all(&app_data_dir).ok();
            std::fs::create_dir_all(app_data_dir.join("audio")).ok();
            std::fs::create_dir_all(app_data_dir.join("models")).ok();

            let db_path = app_data_dir.join("wizscribe.db");
            let audio_dir = app_data_dir.join("audio");

            // Initialize state
            let state = AppState {
                db: Arc::new(Mutex::new(None)),
                audio: Arc::new(Mutex::new(audio::AudioRecorder::new(audio_dir.clone()))),
                dual_audio: Arc::new(Mutex::new(dual_audio::DualAudioRecorder::new(audio_dir))),
                whisper: Arc::new(Mutex::new(
                    whisper::WhisperTranscriber::new().expect("Failed to create WhisperTranscriber")
                )),
                ai: Arc::new(Mutex::new(
                    ai::AIClient::new().expect("Failed to create AIClient")
                )),
                config: Arc::new(Mutex::new(config::AppConfig::default())),
            };

            app.manage(state);

            // Initialize database and load config asynchronously
            let app_handle = app.handle().clone();
            let db_path_str = format!("sqlite:{}?mode=rwc", db_path.display());

            tauri::async_runtime::spawn(async move {
                // Load config
                let loaded_config = config::AppConfig::load().await;

                let state = app_handle.state::<AppState>();

                // Set config
                {
                    let mut cfg = state.config.lock().await;
                    *cfg = loaded_config.clone();
                }

                // If we have saved API credentials, set them in the AI client
                if let (Some(api_key), Some(provider)) = (&loaded_config.ai_api_key, &loaded_config.ai_provider) {
                    let ai = state.ai.lock().await;
                    if let Err(e) = ai.set_api_key(api_key, provider).await {
                        eprintln!("Failed to restore API key: {}", e);
                    } else {
                        println!("API key restored from config");
                    }
                }

                // Restore audio device selection
                if let Some(device_index) = loaded_config.selected_audio_device {
                    let mut audio = state.audio.lock().await;
                    audio.set_device(Some(device_index));
                    println!("Audio device restored from config: index {}", device_index);
                }

                // Initialize database
                match db::Database::new(&db_path_str).await {
                    Ok(database) => {
                        let mut db = state.db.lock().await;
                        *db = Some(database);
                        println!("Database initialized successfully");
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize database: {}", e);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Meeting commands
            create_meeting,
            get_meeting,
            list_meetings,
            delete_meeting,
            // Audio commands
            start_recording,
            stop_recording,
            pause_recording,
            resume_recording,
            get_recording_state,
            list_audio_devices,
            set_recording_device,
            get_selected_audio_device,
            // Dual audio commands
            start_dual_recording,
            stop_dual_recording,
            get_dual_audio_config,
            set_dual_audio_config,
            get_audio_devices_by_type,
            transcribe_dual_audio,
            get_dual_recording_state,
            // Transcription commands
            transcribe_audio,
            is_whisper_model_available,
            get_whisper_model_path,
            // AI commands
            set_ai_api_key,
            get_ai_config,
            generate_summary,
            chat_with_ai,
            extract_action_items,
            // Note commands
            get_note,
            update_note,
            // Template commands
            list_templates,
            get_template,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
