use eframe::egui;
use crate::gui::app::AppSettings;

pub struct DownloadDialog {
    app_id: u32,
    depot_id: String,
    install_dir: String,
    verify_download: bool,
}

impl DownloadDialog {
    pub fn new(app_id: u32) -> Self {
        // Default depot ID is app_id + 1 (for Windows games)
        Self {
            app_id,
            depot_id: (app_id + 1).to_string(),
            install_dir: format!("./downloads/{}", app_id),
            verify_download: true,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<DownloadConfig> {
        let mut confirmed = false;
        let mut cancelled = false;

        ui.label(format!("Download configuration for App ID: {}", self.app_id));
        ui.label("The manifest will be automatically determined from Steam servers.");
        ui.separator();

        // Depot ID input (REQUIRED - user must specify which depot to download)
        ui.horizontal(|ui| {
            ui.label("Depot ID:");
            ui.text_edit_singleline(&mut self.depot_id)
                .on_hover_text("The depot ID to download (check your depot keys to see available depots)");
        });
        ui.label("Tip: Common depot IDs:");
        ui.label(format!("  - {} (Linux/SteamOS)", self.app_id));
        ui.label(format!("  - {} (Windows)", self.app_id + 1));
        ui.label(format!("  - {} (macOS)", self.app_id + 2));

        // Install directory
        ui.horizontal(|ui| {
            ui.label("Install Directory:");
            ui.text_edit_singleline(&mut self.install_dir);
            if ui.button("📁 Browse").clicked() {
                // TODO: Implement directory picker
            }
        });

        // Options
        ui.checkbox(&mut self.verify_download, "Verify downloaded files");

        ui.separator();

        // Buttons
        ui.horizontal(|ui| {
            if ui.button("✓ Download").clicked() {
                if self.depot_id.parse::<u32>().is_ok() {
                    confirmed = true;
                } else {
                    // Show error if depot ID is invalid
                    ui.label("Invalid Depot ID");
                }
            }

            if ui.button("✗ Cancel").clicked() {
                cancelled = true;
            }
        });

        if cancelled {
            return None;
        }

        if confirmed {
            let depot_id = self.depot_id.parse::<u32>().ok()?;

            return Some(DownloadConfig {
                app_id: self.app_id,
                depot_id,
                install_dir: self.install_dir.clone(),
                verify: self.verify_download,
            });
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub app_id: u32,
    pub depot_id: u32,
    pub install_dir: String,
    pub verify: bool,
}

pub struct SettingsDialog {
    show: bool,
}

impl SettingsDialog {
    pub fn new(_settings: &AppSettings) -> Self {
        Self {
            show: true,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, settings: &mut AppSettings) {
        ui.heading("Settings");
        ui.separator();

        ui.collapsing("Manifest Repository", |ui| {
            ui.horizontal(|ui| {
                ui.label("Repository URL:");
                ui.text_edit_singleline(&mut settings.manifest_repo_url);
            });

            ui.horizontal(|ui| {
                ui.label("Local Path:");
                ui.text_edit_singleline(&mut settings.local_manifest_path);
                if ui.button("📁 Browse").clicked() {
                    // TODO: Implement directory picker
                }
            });
        });

        ui.collapsing("Steam Authentication", |ui| {
            ui.horizontal(|ui| {
                ui.label("Username:");
                ui.text_edit_singleline(&mut settings.steam_username);
            });

            ui.horizontal(|ui| {
                ui.label("Password:");
                ui.add(egui::TextEdit::singleline(&mut settings.steam_password)
                    .password(true));
            });

            ui.label("Note: Password is stored locally only");
        });

        ui.collapsing("Download Settings", |ui| {
            ui.horizontal(|ui| {
                ui.label("Output Directory:");
                ui.text_edit_singleline(&mut settings.output_directory);
                if ui.button("📁 Browse").clicked() {
                    // TODO: Implement directory picker
                }
            });
        });

        ui.separator();

        if ui.button("Save").clicked() {
            self.show = false;
        }

        if ui.button("Cancel").clicked() {
            self.show = false;
        }
    }

    pub fn should_show(&self) -> bool {
        self.show
    }
}

pub struct LoginDialog {
    username: String,
    password: String,
    use_qr_code: bool,
    remember_password: bool,
}

impl LoginDialog {
    pub fn new() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            use_qr_code: false,
            remember_password: false,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<LoginResult> {
        ui.heading("Steam Login");
        ui.separator();

        ui.checkbox(&mut self.use_qr_code, "Login with QR Code");

        if !self.use_qr_code {
            ui.horizontal(|ui| {
                ui.label("Username:");
                ui.text_edit_singleline(&mut self.username);
            });

            ui.horizontal(|ui| {
                ui.label("Password:");
                ui.add(egui::TextEdit::singleline(&mut self.password)
                    .password(true));
            });

            ui.checkbox(&mut self.remember_password, "Remember password");
        } else {
            ui.label("Scan the QR code with your Steam Mobile App:");
            // TODO: Generate and display QR code
            ui.label("[QR Code would be displayed here]");
        }

        ui.separator();

        let mut result = None;

        ui.horizontal(|ui| {
            if ui.button("Login").clicked() {
                if self.use_qr_code {
                    result = Some(LoginResult::QrCode);
                } else if !self.username.is_empty() && !self.password.is_empty() {
                    result = Some(LoginResult::Credentials {
                        username: self.username.clone(),
                        password: self.password.clone(),
                        remember: self.remember_password,
                    });
                }
            }

            if ui.button("Anonymous").clicked() {
                result = Some(LoginResult::Anonymous);
            }

            if ui.button("Cancel").clicked() {
                result = Some(LoginResult::Cancelled);
            }
        });

        result
    }
}

#[derive(Debug, Clone)]
pub enum LoginResult {
    Anonymous,
    Credentials { username: String, password: String, remember: bool },
    QrCode,
    Cancelled,
}

pub struct DepotKeyDialog {
    depot_keys: Vec<(u32, String)>,
    selected_depot: Option<usize>,
}

impl DepotKeyDialog {
    pub fn new(depot_keys: Vec<(u32, String)>) -> Self {
        Self {
            depot_keys,
            selected_depot: None,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<(u32, String)> {
        ui.heading("Select Depot");
        ui.separator();

        ui.label("Available depots:");
        
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for (idx, (depot_id, _key)) in self.depot_keys.iter().enumerate() {
                    let selected = self.selected_depot == Some(idx);
                    if ui.selectable_label(selected, format!("Depot ID: {}", depot_id))
                        .clicked() {
                        self.selected_depot = Some(idx);
                    }
                }
            });

        ui.separator();

        let mut result = None;

        ui.horizontal(|ui| {
            if ui.button("Select").clicked() {
                if let Some(idx) = self.selected_depot {
                    let (id, key) = self.depot_keys[idx].clone();
                    result = Some((id, key));
                }
            }

            if ui.button("Cancel").clicked() {
                result = Some((0, String::new()));
            }
        });

        result
    }
}
