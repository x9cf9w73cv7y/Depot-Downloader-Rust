use eframe::egui;
use std::collections::VecDeque;

const MAX_LOG_LINES: usize = 1000;

#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

pub struct LogBox {
    entries: VecDeque<LogEntry>,
    scroll_to_bottom: bool,
    auto_scroll: bool,
}

impl LogBox {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_LOG_LINES),
            scroll_to_bottom: true,
            auto_scroll: true,
        }
    }

    pub fn add(&mut self, level: LogLevel, message: impl Into<String>) {
        let entry = LogEntry {
            level,
            message: message.into(),
            timestamp: chrono::Local::now(),
        };

        if self.entries.len() >= MAX_LOG_LINES {
            self.entries.pop_front();
        }

        self.entries.push_back(entry);
        
        if self.auto_scroll {
            self.scroll_to_bottom = true;
        }
    }

    pub fn info(&mut self, message: impl Into<String>) {
        self.add(LogLevel::Info, message);
    }

    pub fn warning(&mut self, message: impl Into<String>) {
        self.add(LogLevel::Warning, message);
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.add(LogLevel::Error, message);
    }

    pub fn success(&mut self, message: impl Into<String>) {
        self.add(LogLevel::Success, message);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn get_color_for_level(level: &LogLevel) -> egui::Color32 {
        match level {
            LogLevel::Info => egui::Color32::LIGHT_GRAY,
            LogLevel::Warning => egui::Color32::YELLOW,
            LogLevel::Error => egui::Color32::RED,
            LogLevel::Success => egui::Color32::GREEN,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.label("Log");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
                if ui.button("Clear").clicked() {
                    self.clear();
                }
            });
        });

        ui.separator();

        // Log display
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(self.scroll_to_bottom)
            .show(ui, |ui| {
                for entry in &self.entries {
                    let color = Self::get_color_for_level(&entry.level);
                    let timestamp = entry.timestamp.format("%H:%M:%S").to_string();
                    
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            egui::Color32::DARK_GRAY,
                            format!("[{}]", timestamp)
                        );
                        ui.colored_label(color, &entry.message);
                    });
                }

                if self.scroll_to_bottom {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    self.scroll_to_bottom = false;
                }
            });
    }
}

impl Default for LogBox {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GameInfoPanel {
    // Could be extended with additional game info display features
}

impl GameInfoPanel {
    pub fn new() -> Self {
        Self {}
    }

    pub fn ui(&self, ui: &mut egui::Ui, app_id: u32, name: &str) {
        ui.group(|ui| {
            ui.heading(name);
            ui.label(format!("App ID: {}", app_id));
        });
    }
}

impl Default for GameInfoPanel {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ProgressBar {
    value: f32,
    label: String,
}

impl ProgressBar {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            label: String::new(),
        }
    }

    pub fn set_progress(&mut self, value: f32) {
        self.value = value.clamp(0.0, 1.0);
    }

    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        let text = if self.label.is_empty() {
            format!("{:.1}%", self.value * 100.0)
        } else {
            format!("{} - {:.1}%", self.label, self.value * 100.0)
        };

        ui.add(egui::ProgressBar::new(self.value).text(text));
    }
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self::new()
    }
}
