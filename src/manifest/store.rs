use anyhow::{Result, Context};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::manifest::parser::Manifest;

pub struct ManifestStore {
    base_path: PathBuf,
    manifest_cache: HashMap<(u32, u64), Manifest>,
}

impl ManifestStore {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        let base_path = base_path.as_ref().to_path_buf();
        Self {
            base_path,
            manifest_cache: HashMap::new(),
        }
    }

    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("depot_downloader")
            .join("manifests")
    }

    pub fn get_manifest_path(&self, depot_id: u32, manifest_id: u64) -> PathBuf {
        self.base_path
            .join(format!("{}_{}.manifest", depot_id, manifest_id))
    }

    pub fn get_depot_key_path(&self, app_id: u32) -> PathBuf {
        self.base_path.join(format!("{}.key", app_id))
    }

    pub async fn save_manifest(
        &self,
        depot_id: u32,
        manifest_id: u64,
        data: &[u8],
    ) -> Result<PathBuf> {
        let path = self.get_manifest_path(depot_id, manifest_id);
        
        // Create directory if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&path, data).await?;
        tracing::info!("Saved manifest to: {:?}", path);
        
        Ok(path)
    }

    pub async fn load_manifest(
        &mut self,
        depot_id: u32,
        manifest_id: u64,
    ) -> Result<Option<Manifest>> {
        // Check cache first
        if let Some(manifest) = self.manifest_cache.get(&(depot_id, manifest_id)) {
            return Ok(Some(manifest.clone()));
        }

        let path = self.get_manifest_path(depot_id, manifest_id);
        
        if !path.exists() {
            return Ok(None);
        }

        let data = tokio::fs::read(&path).await
            .context("Failed to read manifest file")?;

        // Parse manifest
        let manifest = Manifest::from_bytes(&data)?;
        
        // Cache it
        self.manifest_cache.insert((depot_id, manifest_id), manifest.clone());
        
        Ok(Some(manifest))
    }

    pub fn manifest_exists(&self, depot_id: u32, manifest_id: u64) -> bool {
        self.get_manifest_path(depot_id, manifest_id).exists()
    }

    pub async fn save_depot_keys(
        &self,
        app_id: u32,
        keys: &HashMap<u32, String>,
    ) -> Result<()> {
        let path = self.get_depot_key_path(app_id);
        
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content: String = keys.iter()
            .map(|(depot_id, key)| format!("{};{}", depot_id, key))
            .collect::<Vec<_>>()
            .join("\n");

        tokio::fs::write(&path, content).await?;
        tracing::info!("Saved depot keys for app {} to: {:?}", app_id, path);
        
        Ok(())
    }

    pub async fn load_depot_keys(
        &self,
        app_id: u32,
    ) -> Result<HashMap<u32, String>> {
        let path = self.get_depot_key_path(app_id);
        
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let content = tokio::fs::read_to_string(&path).await?;
        
        let mut keys = HashMap::new();
        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(2, ';').collect();
            if parts.len() == 2 {
                if let Ok(depot_id) = parts[0].parse::<u32>() {
                    keys.insert(depot_id, parts[1].to_string());
                }
            }
        }

        Ok(keys)
    }

    pub fn clear_cache(&mut self) {
        self.manifest_cache.clear();
    }

    pub fn cache_manifest(&mut self, depot_id: u32, manifest_id: u64, manifest: Manifest) {
        self.manifest_cache.insert((depot_id, manifest_id), manifest);
    }

    pub async fn list_manifests(&self) -> Result<Vec<(u32, u64)>> {
        let mut manifests = Vec::new();
        
        if !self.base_path.exists() {
            return Ok(manifests);
        }

        let mut entries = tokio::fs::read_dir(&self.base_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            
            if file_name.ends_with(".manifest") {
                // Parse filename: depotId_manifestId.manifest
                let parts: Vec<&str> = file_name
                    .trim_end_matches(".manifest")
                    .split('_')
                    .collect();
                
                if parts.len() == 2 {
                    if let (Ok(depot_id), Ok(manifest_id)) = 
                        (parts[0].parse::<u32>(), parts[1].parse::<u64>()) {
                        manifests.push((depot_id, manifest_id));
                    }
                }
            }
        }

        Ok(manifests)
    }

    pub async fn delete_manifest(&self, depot_id: u32, manifest_id: u64) -> Result<()> {
        let path = self.get_manifest_path(depot_id, manifest_id);
        
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
            tracing::info!("Deleted manifest: {:?}", path);
        }

        Ok(())
    }
}

impl Default for ManifestStore {
    fn default() -> Self {
        Self::new(Self::default_path())
    }
}
