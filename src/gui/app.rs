use eframe::{egui, CreationContext};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedSender, UnboundedReceiver};

use crate::steam::web_api::{SteamWebApi, GameInfo};
use crate::gui::components::LogBox;
use crate::gui::dialogs::{DownloadDialog, SettingsDialog, DownloadConfig};
use crate::downloader::{DownloadManager, DownloadProgress};
use crate::manifest::ManifestHubFetcher;

#[derive(Debug, Clone)]
pub enum AppMessage {
    GameFound(GameInfo),
    GameNotFound,
    SearchError(String),
    DownloadProgress(DownloadProgress),
    DepotKeysFetched(HashMap<u32, String>),
    DepotKeysError(String),
}

#[derive(Debug, Clone)]
enum AppState {
    Idle,
    Searching,
    Downloading,
    Error(String),
}

pub struct DepotDownloaderApp {
    state: AppState,
    app_id_input: String,
    current_game: Option<GameInfo>,
    log_box: LogBox,
    download_dialog: Option<DownloadDialog>,
    settings_dialog: Option<SettingsDialog>,
    steam_api: Arc<SteamWebApi>,
    progress: f32,
    settings: AppSettings,
    message_tx: UnboundedSender<AppMessage>,
    message_rx: UnboundedReceiver<AppMessage>,
    depot_keys: std::collections::HashMap<u32, String>, // depot_id -> key
}

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub manifest_repo_url: String,
    pub local_manifest_path: String,
    pub steam_username: String,
    pub steam_password: String,
    pub output_directory: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            manifest_repo_url: "https://github.com/SteamAutoCracks/ManifestHub".to_string(),
            local_manifest_path: "./manifests".to_string(),
            steam_username: String::new(),
            steam_password: String::new(),
            output_directory: "./downloads".to_string(),
        }
    }
}

impl DepotDownloaderApp {
    pub fn new(cc: &CreationContext) -> Self {
        // Customize fonts if needed
        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(20.0, egui::FontFamily::Proportional),
        );
        cc.egui_ctx.set_style(style);

        let steam_api = Arc::new(SteamWebApi::new().expect("Failed to create Steam API client"));
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        Self {
            state: AppState::Idle,
            app_id_input: String::new(),
            current_game: None,
            log_box: LogBox::new(),
            download_dialog: None,
            settings_dialog: None,
            steam_api,
            progress: 0.0,
            settings: AppSettings::default(),
            message_tx,
            message_rx,
            depot_keys: HashMap::new(),
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Depot Downloader");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⚙ Settings").clicked() {
                    self.settings_dialog = Some(SettingsDialog::new(&self.settings));
                }
                if ui.button("📁 Open Downloads").clicked() {
                    if let Err(e) = open::that(&self.settings.output_directory) {
                        self.log_box.error(format!("Failed to open downloads folder: {}", e));
                    }
                }
            });
        });
        ui.separator();
    }

    fn render_search_section(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("Search Game");
            
            ui.horizontal(|ui| {
                ui.label("App ID:");
                ui.text_edit_singleline(&mut self.app_id_input)
                    .on_hover_text("Enter the Steam App ID");
                
                let search_button = ui.button("🔍 Search");
                let search_enabled = matches!(self.state, AppState::Idle) 
                    && !self.app_id_input.is_empty();
                
                if search_button.clicked() && search_enabled {
                    self.handle_search();
                }
                
                if matches!(self.state, AppState::Searching) {
                    ui.spinner();
                }
            });

            // Game info display
            if let Some(game) = &self.current_game {
                ui.separator();
                
                ui.vertical(|ui| {
                    ui.heading(&game.name);
                    ui.label(format!("App ID: {}", game.app_id));
                    
                    if !game.developers.is_empty() {
                        ui.label(format!("Developer: {}", game.developers.join(", ")));
                    }
                    
                    if let Some(date) = &game.release_date {
                        ui.label(format!("Release Date: {}", date));
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .show(ui, |ui| {
                        ui.label(&game.description);
                    });

                ui.separator();

                // Show available depot keys
                if !self.depot_keys.is_empty() {
                    ui.label("Available Depot Keys:");
                    for (depot_id, key) in &self.depot_keys {
                        ui.label(format!("  Depot {}: {}...", depot_id, &key[..20.min(key.len())]));
                    }
                } else {
                    ui.label("No depot keys loaded. Click 'Get Depot Keys' first.");
                }

                ui.separator();

                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("📥 Get Depot Keys").clicked() {
                        self.handle_get_depot_keys();
                    }

                    if ui.button("⬇️ Download").clicked() && !self.depot_keys.is_empty() {
                        self.show_download_dialog();
                    }
                });
            }
        });
    }

    fn render_log_section(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            self.log_box.ui(ui);
        });
    }

    fn render_progress(&mut self, ui: &mut egui::Ui) {
        if matches!(self.state, AppState::Downloading) {
            ui.add(
                egui::ProgressBar::new(self.progress)
                    .text(format!("{:.1}%", self.progress * 100.0))
            );
        }
    }

    fn handle_search(&mut self) {
        if let Ok(app_id) = self.app_id_input.parse::<u32>() {
            self.state = AppState::Searching;
            self.current_game = None;
            self.log_box.info(format!("Searching for App ID: {}", app_id));

            let api = self.steam_api.clone();
            let tx = self.message_tx.clone();

            tokio::spawn(async move {
                match api.get_game_info(app_id).await {
                    Ok(Some(game)) => {
                        let _ = tx.send(AppMessage::GameFound(game));
                    }
                    Ok(None) => {
                        let _ = tx.send(AppMessage::GameNotFound);
                    }
                    Err(e) => {
                        let _ = tx.send(AppMessage::SearchError(e.to_string()));
                    }
                }
            });
        } else {
            self.state = AppState::Error("Invalid App ID".to_string());
            self.log_box.error("Please enter a valid numeric App ID");
        }
    }

    fn handle_messages(&mut self) {
        while let Ok(message) = self.message_rx.try_recv() {
            match message {
                AppMessage::GameFound(game) => {
                    self.current_game = Some(game.clone());
                    self.state = AppState::Idle;
                    self.log_box.success(format!("Found game: {}", game.name));
                }
                AppMessage::GameNotFound => {
                    self.state = AppState::Error("Game not found".to_string());
                    self.log_box.error("Game not found for the provided App ID");
                }
                AppMessage::SearchError(error) => {
                    self.state = AppState::Error(error.clone());
                    self.log_box.error(format!("Search error: {}", error));
                }
                AppMessage::DownloadProgress(progress) => {
                    if progress.complete {
                        self.state = AppState::Idle;
                        self.progress = 1.0;
                        self.log_box.success("Download completed!");
                    } else {
                        self.progress = progress.progress_percent();
                        self.log_box.info(progress.message.clone());
                    }
                }
                AppMessage::DepotKeysFetched(keys) => {
                    self.state = AppState::Idle;
                    self.depot_keys = keys.clone();
                    self.log_box.success(format!("Fetched {} depot keys", keys.len()));
                    for (depot_id, key) in &keys {
                        tracing::info!("Depot {}: {}...", depot_id, &key[..20.min(key.len())]);
                    }
                }
                AppMessage::DepotKeysError(error) => {
                    self.state = AppState::Error(error.clone());
                    self.log_box.error(format!("Failed to fetch depot keys: {}", error));
                }
            }
        }
    }

    fn handle_get_depot_keys(&mut self) {
        if let Some(game) = &self.current_game {
            self.log_box.info(format!("Fetching depot keys for App ID: {}", game.app_id));
            self.state = AppState::Searching;
            
            let tx = self.message_tx.clone();
            let app_id = game.app_id;
            
            tokio::spawn(async move {
                let mut fetcher = ManifestHubFetcher::default();
                match fetcher.fetch_depot_keys(app_id).await {
                    Ok(keys) => {
                        tracing::info!("Fetched {} depot keys", keys.len());
                        let _ = tx.send(AppMessage::DepotKeysFetched(keys));
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch depot keys: {}", e);
                        let _ = tx.send(AppMessage::DepotKeysError(e.to_string()));
                    }
                }
            });
        }
    }

    fn show_download_dialog(&mut self) {
        if let Some(game) = &self.current_game {
            self.download_dialog = Some(DownloadDialog::new(game.app_id));
        }
    }

    fn start_download(&mut self, config: DownloadConfig) {
        // Check if we have depot keys
        if self.depot_keys.is_empty() {
            self.state = AppState::Error("No depot keys found. Please click 'Get Depot Keys' first.".to_string());
            self.log_box.error("No depot keys found. Please click 'Get Depot Keys' first.");
            return;
        }

        self.state = AppState::Downloading;
        self.progress = 0.0;
        self.log_box.info(format!(
            "Starting download for App ID: {}, Depot ID: {}",
            config.app_id, config.depot_id
        ));
        self.log_box.info(format!("Install directory: {}", config.install_dir));
        self.log_box.info(format!("Using {} depot keys", self.depot_keys.len()));

        // Clone depot keys for the async task
        let depot_keys = self.depot_keys.clone();

        // Create download manager
        let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(100);
        let message_tx = self.message_tx.clone();
        let message_tx2 = message_tx.clone();

        // Spawn a task to forward progress messages
        tokio::spawn(async move {
            while let Some(progress) = progress_rx.recv().await {
                let _ = message_tx.send(AppMessage::DownloadProgress(progress));
            }
        });

        // Start the actual download
        let depot_id = config.depot_id;
        let app_id = config.app_id;
        
        tokio::spawn(async move {
            match DownloadManager::new(progress_tx) {
                Ok(mut manager) => {
                    // Add depot keys to the manager
                    manager.set_depot_keys(depot_keys);
                    
                    let install_dir = PathBuf::from(config.install_dir);
                    // Pass None for manifest_id - it will be fetched from Steam
                    match manager.download_depot(
                        app_id,
                        depot_id,
                        None, // Manifest ID will be fetched from Steam
                        &install_dir,
                    ).await {
                        Ok(_) => {
                            tracing::info!("Download completed successfully");
                        }
                        Err(e) => {
                            tracing::error!("Download failed: {}", e);
                            let _ = message_tx2.send(AppMessage::SearchError(format!("Download failed: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create download manager: {}", e);
                    let _ = message_tx2.send(AppMessage::SearchError(format!("Failed to create download manager: {}", e)));
                }
            }
        });
    }
}

impl eframe::App for DepotDownloaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process any async messages
        self.handle_messages();

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_header(ui);
            self.render_search_section(ui);
            ui.add_space(10.0);
            self.render_progress(ui);
            ui.add_space(10.0);
            self.render_log_section(ui);
        });

        // Render modal dialogs
        let mut download_config: Option<DownloadConfig> = None;
        
        if let Some(dialog) = &mut self.download_dialog {
            let mut open = true;
            
            egui::Window::new("Download")
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    if let Some(config) = dialog.ui(ui) {
                        download_config = Some(config);
                    }
                });
            
            if download_config.is_some() || !open {
                self.download_dialog = None;
            }
        }
        
        // Start download outside of the closure to avoid borrow issues
        if let Some(config) = download_config {
            self.start_download(config);
        }

        if let Some(dialog) = &mut self.settings_dialog {
            let mut open = true;
            egui::Window::new("Settings")
                .open(&mut open)
                .resizable(true)
                .collapsible(false)
                .show(ctx, |ui| {
                    dialog.ui(ui, &mut self.settings);
                });
            
            if !open {
                self.settings_dialog = None;
            }
        }

        // Handle error state
        if let AppState::Error(msg) = &self.state {
            let msg = msg.clone();
            let mut close = false;
            
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.colored_label(egui::Color32::RED, &msg);
                    if ui.button("OK").clicked() {
                        close = true;
                    }
                });
            
            if close {
                self.state = AppState::Idle;
            }
        }
    }

    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        // Cleanup if needed
    }
}
