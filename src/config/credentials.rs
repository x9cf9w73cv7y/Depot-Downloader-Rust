use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: Option<String>,
    // Store encrypted password or refresh token
    pub encrypted_password: Option<Vec<u8>>,
    pub refresh_token: Option<String>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for Credentials {
    fn default() -> Self {
        Self {
            username: None,
            encrypted_password: None,
            refresh_token: None,
            last_login: None,
        }
    }
}

impl Credentials {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> Result<Self> {
        let credentials_path = Self::get_credentials_path();
        
        if !credentials_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&credentials_path)
            .context("Failed to read credentials file")?;
        
        let credentials: Credentials = serde_json::from_str(&content)
            .context("Failed to parse credentials file")?;

        Ok(credentials)
    }

    pub fn save(&self) -> Result<()> {
        let credentials_path = Self::get_credentials_path();
        
        std::fs::create_dir_all(&credentials_path.parent().unwrap())?;
        
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize credentials")?;
        
        std::fs::write(&credentials_path, content)
            .context("Failed to write credentials file")?;

        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        self.username = None;
        self.encrypted_password = None;
        self.refresh_token = None;
        self.last_login = None;
        self.save()
    }

    fn get_credentials_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("depot_downloader")
            .join("credentials.json")
    }

    pub fn has_saved_credentials(&self) -> bool {
        self.username.is_some() && 
            (self.encrypted_password.is_some() || self.refresh_token.is_some())
    }

    pub fn set_credentials(&mut self, username: String, password: String) {
        self.username = Some(username);
        // In a real implementation, you'd encrypt the password here
        // For now, we'll just store it as bytes (not secure!)
        self.encrypted_password = Some(password.into_bytes());
    }

    pub fn get_password(&self) -> Option<String> {
        self.encrypted_password.as_ref()
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
    }
}
