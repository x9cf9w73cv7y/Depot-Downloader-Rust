use anyhow::{Result, Context};
use std::collections::HashMap;
use std::time::Duration;

use crate::steam::auth::{AuthMethod, AuthSession, SteamAuth};

pub struct SteamSession {
    auth: SteamAuth,
    session: Option<AuthSession>,
    connected: bool,
    depot_keys: HashMap<u32, Vec<u8>>,
}

impl SteamSession {
    pub fn new() -> Self {
        Self {
            auth: SteamAuth::new(),
            session: None,
            connected: false,
            depot_keys: HashMap::new(),
        }
    }

    pub fn new_with_auth(auth: SteamAuth) -> Self {
        Self {
            auth,
            session: None,
            connected: false,
            depot_keys: HashMap::new(),
        }
    }

    pub async fn connect(&mut self, method: AuthMethod) -> Result<()> {
        tracing::info!("Connecting to Steam...");
        
        self.session = Some(self.auth.authenticate(method).await?);
        self.connected = true;
        
        tracing::info!("Connected to Steam");
        Ok(())
    }

    pub async fn connect_anonymous(&mut self) -> Result<()> {
        self.connect(AuthMethod::Anonymous).await
    }

    pub fn disconnect(&mut self) {
        tracing::info!("Disconnecting from Steam");
        self.session = None;
        self.connected = false;
        self.depot_keys.clear();
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn is_authenticated(&self) -> bool {
        self.session.as_ref().map_or(false, |s| s.is_authenticated())
    }

    pub fn get_session(&self) -> Option<&AuthSession> {
        self.session.as_ref()
    }

    pub fn request_depot_key(&mut self, depot_id: u32) -> Result<Vec<u8>> {
        if let Some(key) = self.depot_keys.get(&depot_id) {
            return Ok(key.clone());
        }

        // Request from Steam
        // This would normally use SteamKit2 or similar
        // For now, return error
        Err(anyhow::anyhow!("Depot key not available"))
    }

    pub fn add_depot_key(&mut self, depot_id: u32, key: Vec<u8>) {
        self.depot_keys.insert(depot_id, key);
    }

    pub fn has_depot_key(&self, depot_id: u32) -> bool {
        self.depot_keys.contains_key(&depot_id)
    }

    pub fn tick(&mut self) {
        // Process callbacks, heartbeats, etc.
        // This is called periodically
    }
}

impl Default for SteamSession {
    fn default() -> Self {
        Self::new()
    }
}
