use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub manifest_repo_url: String,
    pub local_manifest_path: PathBuf,
    pub output_directory: PathBuf,
    pub max_concurrent_downloads: usize,
    pub verify_downloads: bool,
    pub save_credentials: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let default_manifest_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("depot_downloader")
            .join("manifests");
        
        let default_output = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("depot_downloader");

        Self {
            manifest_repo_url: "https://github.com/SteamAutoCracks/ManifestHub".to_string(),
            local_manifest_path: default_manifest_path,
            output_directory: default_output,
            max_concurrent_downloads: 8,
            verify_downloads: true,
            save_credentials: false,
        }
    }
}

impl Settings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path();
        
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)
            .context("Failed to read settings file")?;
        
        let settings: Settings = serde_json::from_str(&content)
            .context("Failed to parse settings file")?;

        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path();
        
        std::fs::create_dir_all(&config_path.parent().unwrap())?;
        
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize settings")?;
        
        std::fs::write(&config_path, content)
            .context("Failed to write settings file")?;

        Ok(())
    }

    fn get_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("depot_downloader")
            .join("settings.json")
    }

    pub fn ensure_directories(&self) -> Result<()> {
        std::fs::create_dir_all(&self.local_manifest_path)?;
        std::fs::create_dir_all(&self.output_directory)?;
        Ok(())
    }
}
