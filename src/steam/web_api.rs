use anyhow::{Result, Context};
use reqwest;
use serde::{Deserialize, Serialize};

const STEAM_STORE_API: &str = "https://store.steampowered.com/api/appdetails";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub app_id: u32,
    pub name: String,
    pub description: String,
    pub header_image: Option<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub release_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SteamApiResponse {
    success: bool,
    data: Option<GameData>,
}

#[derive(Debug, Clone, Deserialize)]
struct GameData {
    name: String,
    #[serde(rename = "short_description")]
    short_description: Option<String>,
    #[serde(rename = "header_image")]
    header_image: Option<String>,
    developers: Option<Vec<String>>,
    publishers: Option<Vec<String>>,
    #[serde(rename = "release_date")]
    release_date: Option<ReleaseDate>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReleaseDate {
    date: Option<String>,
}

pub struct SteamWebApi {
    client: reqwest::Client,
}

impl SteamWebApi {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client })
    }

    pub async fn get_game_info(&self, app_id: u32) -> Result<Option<GameInfo>> {
        let response = self.client
            .get(STEAM_STORE_API)
            .query(&[("appids", app_id.to_string())])
            .send()
            .await
            .context("Failed to fetch game info from Steam API")?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!("Steam API returned status: {}", status));
        }

        let text = response.text().await?;
        let api_response: std::collections::HashMap<String, SteamApiResponse> = 
            serde_json::from_str(&text)
                .context("Failed to parse Steam API response")?;

        let response = api_response.get(&app_id.to_string())
            .ok_or_else(|| anyhow::anyhow!("No data for app ID {} in response", app_id))?;

        if !response.success || response.data.is_none() {
            return Ok(None);
        }

        let data = response.data.as_ref().unwrap();
        
        Ok(Some(GameInfo {
            app_id,
            name: data.name.clone(),
            description: data.short_description.clone().unwrap_or_default(),
            header_image: data.header_image.clone(),
            developers: data.developers.clone().unwrap_or_default(),
            publishers: data.publishers.clone().unwrap_or_default(),
            release_date: data.release_date.as_ref().and_then(|d| d.date.clone()),
        }))
    }

    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to download image")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to download image: {}", response.status()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}

impl Default for SteamWebApi {
    fn default() -> Self {
        Self::new().expect("Failed to create SteamWebApi")
    }
}
