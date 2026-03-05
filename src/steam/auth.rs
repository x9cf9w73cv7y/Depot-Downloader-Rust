use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum AuthMethod {
    Anonymous,
    Credentials { username: String, password: String },
    QrCode,
    Token { username: String, token: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamCredentials {
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

pub struct SteamAuth {
    credentials: SteamCredentials,
    config_path: PathBuf,
}

impl SteamAuth {
    pub fn new() -> Self {
        let config_path = Self::get_config_path();
        Self {
            credentials: SteamCredentials {
                username: None,
                password: None,
                token: None,
            },
            config_path,
        }
    }

    pub fn with_credentials(username: String, password: String) -> Self {
        let config_path = Self::get_config_path();
        Self {
            credentials: SteamCredentials {
                username: Some(username),
                password: Some(password),
                token: None,
            },
            config_path,
        }
    }

    fn get_config_path() -> PathBuf {
        let base = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        base.join("depot_downloader").join("account.config")
    }

    pub fn get_auth_method(&self) -> AuthMethod {
        if let (Some(username), Some(token)) = 
            (&self.credentials.username, &self.credentials.token) {
            AuthMethod::Token {
                username: username.clone(),
                token: token.clone(),
            }
        } else if let (Some(username), Some(password)) = 
            (&self.credentials.username, &self.credentials.password) {
            AuthMethod::Credentials {
                username: username.clone(),
                password: password.clone(),
            }
        } else {
            AuthMethod::Anonymous
        }
    }

    pub async fn authenticate(&self, method: AuthMethod) -> Result<AuthSession> {
        match method {
            AuthMethod::Anonymous => {
                tracing::info!("Authenticating anonymously");
                Ok(AuthSession::Anonymous)
            }
            AuthMethod::Credentials { username, password } => {
                tracing::info!("Authenticating with credentials for user: {}", username);
                // Steam authentication would go here
                // For now, return a placeholder
                Ok(AuthSession::Authenticated {
                    username,
                    token: None,
                })
            }
            AuthMethod::QrCode => {
                tracing::info!("Authenticating via QR code");
                // QR code authentication would go here
                Ok(AuthSession::QrPending)
            }
            AuthMethod::Token { username, token } => {
                tracing::info!("Authenticating with token for user: {}", username);
                Ok(AuthSession::Authenticated {
                    username,
                    token: Some(token),
                })
            }
        }
    }

    pub fn save_credentials(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_path.parent().unwrap())?;
        let content = serde_json::to_string_pretty(&self.credentials)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }

    pub fn load_credentials(&mut self) -> Result<()> {
        if self.config_path.exists() {
            let content = std::fs::read_to_string(&self.config_path)?;
            self.credentials = serde_json::from_str(&content)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum AuthSession {
    Anonymous,
    Authenticated { username: String, token: Option<String> },
    QrPending,
    Failed(String),
}

impl AuthSession {
    pub fn is_authenticated(&self) -> bool {
        matches!(self, AuthSession::Authenticated { .. })
    }

    pub fn get_username(&self) -> Option<&str> {
        match self {
            AuthSession::Authenticated { username, .. } => Some(username),
            _ => None,
        }
    }
}
