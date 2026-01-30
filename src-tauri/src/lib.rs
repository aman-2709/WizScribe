pub mod audio;
pub mod db;
pub mod whisper;
pub mod ai;
pub mod config;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;

// Application state
pub struct AppState {
    pub db: Arc<Mutex<Option<db::Database>>>,
    pub audio: Arc<Mutex<audio::AudioRecorder>>,
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
    let audio = state.audio.lock().await;
    Ok(audio.get_state())
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
                audio: Arc::new(Mutex::new(audio::AudioRecorder::new(audio_dir))),
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
