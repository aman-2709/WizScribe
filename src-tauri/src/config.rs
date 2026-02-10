use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub ai_api_key: Option<String>,
    pub ai_provider: Option<String>,
    /// DEPRECATED: Use mic_device_index instead
    pub selected_audio_device: Option<usize>,
    /// Dual audio: microphone device index
    pub mic_device_index: Option<usize>,
    /// Dual audio: system audio monitor device index
    pub system_device_index: Option<usize>,
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("wizscribe")
            .join("config.json")
    }

    pub async fn load() -> Self {
        let path = Self::config_path();

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path).await {
            Ok(content) => {
                let mut config: Self = serde_json::from_str(&content).unwrap_or_default();
                // Migrate legacy selected_audio_device to mic_device_index
                if config.selected_audio_device.is_some() && config.mic_device_index.is_none() {
                    config.mic_device_index = config.selected_audio_device;
                    // Save the migrated config
                    if let Err(e) = config.save().await {
                        eprintln!("Failed to save migrated config: {}", e);
                    }
                }
                config
            }
            Err(_) => Self::default(),
        }
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content).await?;

        Ok(())
    }

    pub async fn set_ai_credentials(&mut self, api_key: &str, provider: &str) -> anyhow::Result<()> {
        self.ai_api_key = Some(api_key.to_string());
        self.ai_provider = Some(provider.to_string());
        self.save().await
    }
}
