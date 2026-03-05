use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnServer {
    pub host: String,
    pub port: u16,
    pub https: bool,
    pub weight: u32,
    pub proxy_server: Option<String>,
}

impl CdnServer {
    pub fn new(host: String, port: u16, https: bool) -> Self {
        Self {
            host,
            port,
            https,
            weight: 1,
            proxy_server: None,
        }
    }

    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_proxy(mut self, proxy: String) -> Self {
        self.proxy_server = Some(proxy);
        self
    }

    pub fn url(&self) -> String {
        let scheme = if self.https { "https" } else { "http" };
        format!("{}://{}:{}", scheme, self.host, self.port)
    }

    pub fn base_url(&self) -> String {
        self.url()
    }

    pub fn depot_url(&self, depot_id: u32, manifest_id: u64) -> String {
        format!("{}/depot/{}/manifest/{}/chunk",
            self.url(), depot_id, manifest_id)
    }

    pub fn chunk_url(&self, depot_id: u32, chunk_id: &str) -> String {
        format!("{}/depot/{}/chunk/{}_001",
            self.url(), depot_id, chunk_id)
    }
}

#[derive(Debug, Clone)]
pub struct CdnServerPool {
    servers: Vec<CdnServer>,
    current_index: usize,
}

impl CdnServerPool {
    pub fn new(servers: Vec<CdnServer>) -> Self {
        Self {
            servers,
            current_index: 0,
        }
    }

    pub fn get_server(&mut self) -> Option<&CdnServer> {
        if self.servers.is_empty() {
            return None;
        }

        // Simple round-robin for now
        let server = &self.servers[self.current_index];
        self.current_index = (self.current_index + 1) % self.servers.len();
        Some(server)
    }

    pub fn get_server_by_weight(&mut self) -> Option<&CdnServer> {
        if self.servers.is_empty() {
            return None;
        }

        // Weighted random selection
        let total_weight: u32 = self.servers.iter().map(|s| s.weight).sum();
        if total_weight == 0 {
            return self.get_server();
        }

        let mut rng = rand::random::<u32>() % total_weight;
        for server in &self.servers {
            if rng < server.weight {
                return Some(server);
            }
            rng -= server.weight;
        }

        self.servers.first()
    }

    pub fn add_server(&mut self, server: CdnServer) {
        self.servers.push(server);
    }

    pub fn remove_server(&mut self, host: &str) {
        self.servers.retain(|s| s.host != host);
    }

    pub fn server_count(&self) -> usize {
        self.servers.len()
    }
}
