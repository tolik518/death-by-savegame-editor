mod backup;
mod editor_state;
mod hocon_editor;
mod hocon_parser;

use backup::{BackupInfo, BackupManager};
use dbs_core::{decrypt, encrypt};
use editor_state::{EditMode, EditorState};
use eframe::egui;
use hocon_parser::HoconDocument;

fn main() -> Result<(), eframe::Error> {
    let icon = include_bytes!("../../data/images/logo-square.png");
    let icon_data = eframe::icon_data::from_png_bytes(icon).expect("Icon could not be loaded");
    let version = env!("CARGO_PKG_VERSION");
    let app_name = format!("Death by Savegame Editor v{}", version);

    // Get initial window size from single source of truth
    let initial_size = SaveEditorApp::ideal_window_size_for_state(&EditorState::Welcome);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([initial_size.x, initial_size.y])
            .with_min_inner_size([400.0, 200.0])
            .with_resizable(true)
            .with_icon(icon_data)
            .with_app_id(app_name.clone())
            .with_title(app_name.clone()),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        &app_name,
        options,
        Box::new(|cc| {
            // Install image loaders to support PNG images
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(SaveEditorApp::default()))
        }),
    )
}

struct SaveEditorApp {
    // Application services
    backup_manager: Option<BackupManager>,

    // UI state
    editor_state: EditorState,
    is_dark_mode: bool,
    search_query: String,

    // Transient UI state
    status_message: String,

    // Track state changes for auto-resize
    previous_state_variant: String,
}

impl Default for SaveEditorApp {
    fn default() -> Self {
        let save_dir = Self::get_save_directory();
        let backup_manager = save_dir.map(BackupManager::new);

        Self {
            search_query: String::new(),
            backup_manager,
            editor_state: EditorState::Welcome,
            is_dark_mode: true,
            status_message: String::new(),
            previous_state_variant: String::new(), // Empty so first frame triggers resize
        }
    }
}

impl SaveEditorApp {
    fn get_save_directory() -> Option<std::path::PathBuf> {
        use std::path::PathBuf;

        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()?;

        let mut path = PathBuf::from(home);

        if cfg!(target_os = "linux") {
            path.push(".local");
            path.push("share");
        } else if cfg!(target_os = "windows") {
            path.push("AppData");
            path.push("Local");
        } else if cfg!(target_os = "macos") {
            path.push("Library");
            path.push("Application Support");
        } else {
            return None;
        }

        path.push("Terrible Toybox");
        path.push("Death by Scrolling");

        Some(path)
    }

    /// Returns ideal window size for a given state
    fn ideal_window_size_for_state(state: &EditorState) -> egui::Vec2 {
        match state {
            EditorState::Welcome => egui::vec2(500.0, 450.0),
            EditorState::Editing { .. } => egui::vec2(575.0, 700.0),
            EditorState::EditingParsed { .. } => egui::vec2(575.0, 700.0),
            EditorState::SelectingBackup { .. } => egui::vec2(500.0, 480.0),
            EditorState::ConfirmRestore { .. } => egui::vec2(400.0, 350.0),
            EditorState::Error { .. } => egui::vec2(600.0, 350.0),
            EditorState::LoadingSave => egui::vec2(400.0, 200.0),
        }
    }

    /// Returns ideal window size for current state
    fn ideal_window_size(&self) -> egui::Vec2 {
        Self::ideal_window_size_for_state(&self.editor_state)
    }

    /// Handles loading a savegame
    fn handle_load_savegame(&mut self) {
        let save_path = match &self.backup_manager {
            Some(bm) if bm.save_exists() => bm.get_save_path(),
            _ => {
                // Show file picker
                if let Some(path) = self.show_file_picker() {
                    path
                } else {
                    return; // User cancelled
                }
            }
        };

        self.load_and_edit_file(save_path);
    }

    fn load_and_edit_file(&mut self, save_path: std::path::PathBuf) {
        // Create backup
        let backup_path = match &self.backup_manager {
            Some(bm) => match bm.create_backup(&save_path) {
                Ok(path) => path,
                Err(e) => {
                    self.show_error(format!("Failed to create backup: {}", e));
                    return;
                }
            },
            None => {
                self.show_error("Backup manager not initialized".to_string());
                return;
            }
        };

        // Read and decrypt
        match self.decrypt_file(&save_path) {
            Ok(content) => {
                // Try to parse as HOCON
                match HoconDocument::parse(&content) {
                    Ok(document) => {
                        // Successfully parsed - use form editor
                        self.editor_state = EditorState::EditingParsed {
                            original_path: save_path,
                            backup_path,
                            document,
                            is_modified: false,
                            edit_mode: EditMode::Form,
                        };
                    }
                    Err(e) => {
                        // Failed to parse - fall back to raw text editor
                        eprintln!("Failed to parse HOCON: {}, falling back to raw editor", e);
                        self.editor_state = EditorState::Editing {
                            original_path: save_path,
                            backup_path,
                            content: content.clone(),
                            is_modified: false,
                            original_content: content, // Preserve original
                        };
                    }
                }
            }
            Err(e) => {
                self.show_error(format!("Failed to decrypt: {}", e));
            }
        }
    }

    fn decrypt_file(&self, path: &std::path::PathBuf) -> anyhow::Result<String> {
        let cipher = std::fs::read(path)?;
        let unpacked = decrypt(&cipher)?;
        let content = String::from_utf8_lossy(&unpacked.payload).to_string();

        Ok(content)
    }

    fn handle_save_changes(&mut self) {
        if let EditorState::Editing {
            original_path,
            content,
            ..
        } = &self.editor_state
        {
            match self.encrypt_and_save(content, original_path) {
                Ok(_) => {
                    self.status_message = "Save file updated successfully!".to_string();
                    self.editor_state = EditorState::Welcome;
                }
                Err(e) => {
                    self.show_error(format!("Failed to save: {}", e));
                }
            }
        }
    }

    fn handle_save_changes_parsed(&mut self) {
        if let EditorState::EditingParsed {
            original_path,
            document,
            ..
        } = &self.editor_state
        {
            // Serialize document to HOCON string
            let hocon_string = document.to_hocon_string();

            match self.encrypt_and_save(&hocon_string, original_path) {
                Ok(_) => {
                    self.status_message = "Save file updated successfully!".to_string();
                    self.editor_state = EditorState::Welcome;
                }
                Err(e) => {
                    self.show_error(format!("Failed to save: {}", e));
                }
            }
        }
    }

    fn encrypt_and_save(&self, content: &str, path: &std::path::PathBuf) -> anyhow::Result<()> {
        let cipher = encrypt(content.as_bytes())?;
        std::fs::write(path, cipher)?;

        Ok(())
    }

    fn handle_recover_backup(&mut self) {
        let backups = match &self.backup_manager {
            Some(bm) => match bm.list_backups() {
                Ok(backups) => backups,
                Err(e) => {
                    self.show_error(format!("Failed to list backups: {}", e));
                    return;
                }
            },
            None => {
                self.show_error("Backup manager not initialized".to_string());
                return;
            }
        };

        if backups.is_empty() {
            self.show_error("No backup files found".to_string());
            return;
        }

        self.editor_state = EditorState::SelectingBackup { backups };
    }

    fn handle_select_backup(&mut self, backup_info: BackupInfo) {
        self.editor_state = EditorState::ConfirmRestore {
            backup_info: backup_info.clone(),
        };
    }

    fn handle_confirm_restore(&mut self) {
        if let EditorState::ConfirmRestore { backup_info } = &self.editor_state {
            match &self.backup_manager {
                Some(bm) => match bm.restore_backup(&backup_info.path) {
                    Ok(_) => {
                        self.status_message =
                            format!("Backup restored successfully: {}", backup_info.filename);
                        self.editor_state = EditorState::Welcome;
                    }
                    Err(e) => {
                        self.show_error(format!("Failed to restore backup: {}", e));
                    }
                },
                None => {
                    self.show_error("Backup manager not initialized".to_string());
                }
            }
        }
    }

    fn show_error(&mut self, message: String) {
        let previous_state = Box::new(self.editor_state.clone());
        self.editor_state = EditorState::Error {
            message,
            previous_state,
        };
    }

    fn show_file_picker(&self) -> Option<std::path::PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Save files", &["bin", "save"])
            .add_filter("All files", &["*"])
            .pick_file()
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // Ctrl+S to save (only in editing states)
        if ctx.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command) {
            match &self.editor_state {
                EditorState::Editing { .. } => {
                    self.handle_save_changes();
                }
                EditorState::EditingParsed { .. } => {
                    self.handle_save_changes_parsed();
                }
                _ => {}
            }
        }

        // Escape to clear search or cancel
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if !self.search_query.is_empty() {
                self.search_query.clear();
            }
        }
    }
}

impl eframe::App for SaveEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme
        if self.is_dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ctx);

        // Auto-resize window when state changes
        let current_state_variant = match &self.editor_state {
            EditorState::Welcome => "Welcome",
            EditorState::LoadingSave => "LoadingSave",
            EditorState::Editing { .. } => "Editing",
            EditorState::EditingParsed { .. } => "EditingParsed",
            EditorState::SelectingBackup { .. } => "SelectingBackup",
            EditorState::ConfirmRestore { .. } => "ConfirmRestore",
            EditorState::Error { .. } => "Error",
        };

        if current_state_variant != self.previous_state_variant {
            let ideal_size = self.ideal_window_size();
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(ideal_size));
            self.previous_state_variant = current_state_variant.to_string();
        }

        // Footer as bottom panel (always visible, like a status bar)
        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("by Anatolij <tolik518> Vasilev");

                // Status message on the right
                if !self.status_message.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(egui::Color32::GREEN, &self.status_message);
                    });
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Theme toggle (always visible)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                if ui
                    .button(if self.is_dark_mode { "🌙" } else { "☀" })
                    .clicked()
                {
                    self.is_dark_mode = !self.is_dark_mode;
                }
            });

            // Render based on state
            match &self.editor_state.clone() {
                EditorState::Welcome => self.render_welcome_screen(ui),
                EditorState::LoadingSave => self.render_loading_screen(ui),
                EditorState::Editing { .. } => self.render_editing_screen(ui),
                EditorState::EditingParsed { .. } => self.render_form_editing_screen(ui),
                EditorState::SelectingBackup { backups } => self.render_backup_list(ui, backups),
                EditorState::ConfirmRestore { backup_info } => {
                    self.render_restore_confirmation(ui, backup_info)
                }
                EditorState::Error { message, .. } => self.render_error_screen(ui, message),
            }
        });
    }
}

impl SaveEditorApp {
    fn render_welcome_screen(&mut self, ui: &mut egui::Ui) {
        // Logo
        ui.add_space(15.0);
        ui.vertical_centered(|ui| {
            ui.add(
                egui::Image::new(egui::include_image!("../../data/images/logo-long-x2.png"))
                    .fit_to_original_size(1.0)
                    .max_height(80.0),
            );
        });
        ui.add_space(30.0);

        // Main action buttons
        ui.vertical_centered(|ui| {
            if ui
                .add_sized([300.0, 50.0], egui::Button::new("📂 Load Savegame"))
                .clicked()
            {
                self.handle_load_savegame();
            }

            ui.add_space(15.0);

            if ui
                .add_sized([300.0, 50.0], egui::Button::new("🔄 Recover Backup"))
                .clicked()
            {
                self.handle_recover_backup();
            }
        });

        ui.add_space(30.0);

        // Info box
        ui.group(|ui| {
            ui.label("ℹ Information");
            ui.separator();
            ui.label("• Load Savegame: Edit your current save file");
            ui.label("• Recover Backup: Restore from a previous backup");
            ui.add_space(5.0);
            ui.colored_label(
                egui::Color32::from_rgb(255, 165, 0),
                "⚠ A backup is created automatically before editing",
            );
        });
    }

    fn render_editing_screen(&mut self, ui: &mut egui::Ui) {
        let (original_path, backup_path, mut content, is_modified, original_content) =
            if let EditorState::Editing {
                original_path,
                backup_path,
                content,
                is_modified,
                original_content,
            } = &self.editor_state
            {
                (
                    original_path.clone(),
                    backup_path.clone(),
                    content.clone(),
                    *is_modified,
                    original_content.clone(),
                )
            } else {
                return;
            };

        ui.heading("Edit Savegame");
        ui.add_space(10.0);

        // Mode toggle - try to switch to Form mode
        ui.horizontal(|ui| {
            if ui
                .selectable_label(false, "📝 Form")
                .on_hover_text("Switch to form-based editor")
                .clicked()
            {
                // Try to parse the CURRENT content (with any edits made in Raw mode)
                match HoconDocument::parse(&content) {
                    Ok(document) => {
                        self.editor_state = EditorState::EditingParsed {
                            original_path: original_path.clone(),
                            backup_path: backup_path.clone(),
                            document,
                            is_modified,
                            edit_mode: EditMode::Form,
                        };
                        return;
                    }
                    Err(e) => {
                        self.status_message =
                            format!("Cannot switch to Form mode: Parse error: {}", e);
                    }
                }
            }
            if ui
                .selectable_label(true, "📄 Raw")
                .on_hover_text("Raw text editor (current mode)")
                .clicked()
            {
                // Already in raw mode
            }
        });

        ui.add_space(5.0);

        // File info
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("File:");
                ui.monospace(original_path.display().to_string());
            });
            ui.horizontal(|ui| {
                ui.label("Backup:");
                ui.monospace(
                    backup_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown"),
                );
            });
            if is_modified {
                ui.colored_label(egui::Color32::YELLOW, "• Modified");
            } else {
                ui.label("• Unmodified");
            }
        });

        ui.add_space(10.0);

        // Calculate available height for text editor
        // Reserve space for: buttons (45px) + spacing (45px) = ~90px
        let available_height = ui.available_height() - 93.0;

        // Text editor with scroll area - use fixed height
        ui.label("Content:");
        ui.add_space(5.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(available_height.max(200.0))
            .show(ui, |ui| {
                let text_edit = egui::TextEdit::multiline(&mut content)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace);

                if ui.add(text_edit).changed() {
                    self.editor_state = EditorState::Editing {
                        original_path,
                        backup_path,
                        content,
                        is_modified: true,
                        original_content,
                    };
                }
            });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(5.0);

        // Action buttons - always visible at bottom
        ui.horizontal(|ui| {
            // Make Save button more prominent with green color when there are changes
            let save_button = if is_modified {
                egui::Button::new("💾 Save Changes").fill(egui::Color32::from_rgb(0, 120, 0))
            } else {
                egui::Button::new("💾 Save Changes")
            };

            if ui
                .add_sized([180.0, 40.0], save_button)
                .on_hover_text("Encrypt and save changes to save.bin")
                .clicked()
            {
                self.handle_save_changes();
            }

            if ui
                .add_sized([120.0, 40.0], egui::Button::new("❌ Cancel"))
                .on_hover_text("Discard changes and return to welcome screen (Esc)")
                .clicked()
            {
                self.editor_state = EditorState::Welcome;
                self.status_message.clear();
            }
        });

        ui.add_space(10.0); // Space before footer
    }

    fn render_backup_list(&mut self, ui: &mut egui::Ui, backups: &[BackupInfo]) {
        ui.heading("Available Backups");
        ui.add_space(10.0);

        ui.label(format!("Found {} backup file(s):", backups.len()));
        ui.add_space(10.0);

        let available_height = ui.available_height() - 50.0; // Reserve space for cancel button

        egui::ScrollArea::vertical()
            .max_height(available_height.max(200.0))
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for backup in backups {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.strong(&backup.filename);
                                ui.label(format!(
                                    "Date: {}",
                                    backup.timestamp.format("%Y-%m-%d %H:%M:%S")
                                ));
                                ui.label(format!("Size: {} bytes", backup.size));
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Restore").clicked() {
                                        self.handle_select_backup(backup.clone());
                                    }
                                },
                            );
                        });
                    });
                    ui.add_space(5.0);
                }
            });

        ui.add_space(10.0);

        if ui
            .add_sized([120.0, 40.0], egui::Button::new("❌ Cancel"))
            .on_hover_text("Discard changes and return to welcome screen (Esc)")
            .clicked()
        {
            self.editor_state = EditorState::Welcome;
            self.status_message.clear();
        }
    }

    fn render_restore_confirmation(&mut self, ui: &mut egui::Ui, backup_info: &BackupInfo) {
        ui.heading("⚠ Confirm Restore");
        ui.add_space(20.0);

        ui.label("Are you sure you want to restore this backup?");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.strong("Backup Details:");
            ui.label(format!("File: {}", backup_info.filename));
            ui.label(format!(
                "Date: {}",
                backup_info.timestamp.format("%Y-%m-%d %H:%M:%S")
            ));
            ui.label(format!("Size: {} bytes", backup_info.size));
        });

        ui.add_space(10.0);

        ui.colored_label(
            egui::Color32::from_rgb(255, 165, 0),
            "⚠ This will overwrite your current save.bin file!",
        );
        ui.label("(An emergency backup will be created first)");

        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui
                .add_sized([150.0, 40.0], egui::Button::new("✅ Confirm Restore"))
                .clicked()
            {
                self.handle_confirm_restore();
            }

            ui.add_space(10.0);

            if ui
                .add_sized([150.0, 40.0], egui::Button::new("❌ Cancel"))
                .clicked()
            {
                self.editor_state = EditorState::Welcome;
            }
        });
    }

    fn render_error_screen(&mut self, ui: &mut egui::Ui, message: &str) {
        ui.heading("❌ Error");
        ui.add_space(20.0);

        ui.group(|ui| {
            ui.colored_label(egui::Color32::RED, message);
        });

        ui.add_space(20.0);

        if ui.button("OK").clicked() {
            self.editor_state = EditorState::Welcome;
            self.status_message.clear();
        }
    }

    fn render_loading_screen(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.spinner();
            ui.add_space(20.0);
            ui.label("Loading...");
        });
    }

    fn render_form_editing_screen(&mut self, ui: &mut egui::Ui) {
        let (original_path, backup_path, mut document, is_modified, mut edit_mode) =
            if let EditorState::EditingParsed {
                original_path,
                backup_path,
                document,
                is_modified,
                edit_mode,
            } = &self.editor_state
            {
                (
                    original_path.clone(),
                    backup_path.clone(),
                    document.clone(),
                    *is_modified,
                    edit_mode.clone(),
                )
            } else {
                return;
            };

        ui.heading("Edit Savegame");
        ui.add_space(10.0);

        // Mode toggle
        ui.horizontal(|ui| {
            if ui
                .selectable_label(edit_mode == EditMode::Form, "📝 Form")
                .clicked()
            {
                edit_mode = EditMode::Form;
            }
            if ui
                .selectable_label(edit_mode == EditMode::Raw, "📄 Raw")
                .clicked()
            {
                // Convert to raw text - use current (possibly modified) serialized content
                // If user wants original, they should reload from backup
                let content = document.to_hocon_string();
                self.editor_state = EditorState::Editing {
                    original_path: original_path.clone(),
                    backup_path: backup_path.clone(),
                    content: content.clone(),
                    is_modified,
                    original_content: content, // When switching from Form, this is the "original" for Raw mode
                };
                return;
            }
        });

        ui.add_space(5.0);
        // File info
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("File:");
                ui.monospace(original_path.display().to_string());
            });
            ui.horizontal(|ui| {
                ui.label("Backup:");
                ui.monospace(
                    backup_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown"),
                );
            });
            if is_modified {
                ui.colored_label(egui::Color32::YELLOW, "• Modified");
            } else {
                ui.label("• Unmodified");
            }
        });

        ui.add_space(10.0);

        // Search bar with statistics
        ui.horizontal(|ui| {
            ui.label("🔍 Search:");
            ui.text_edit_singleline(&mut self.search_query);

            if !self.search_query.is_empty() {
                let (total, key_matches, value_matches) =
                    hocon_editor::count_search_matches(&document, &self.search_query);

                ui.label(format!("({} matches)", total));
                if key_matches > 0 {
                    ui.colored_label(egui::Color32::YELLOW, format!("{}k", key_matches));
                }
                if value_matches > 0 {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, format!("{}v", value_matches));
                }
            }

            if ui.button("Clear").clicked() {
                self.search_query.clear();
            }
        });

        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);

        // Calculate available height for editor
        let available_height = ui.available_height() - 65.0;

        // Scrollable content area with categories
        let mut changed = false;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .max_height(available_height.max(200.0))
            .show(ui, |ui| {
                let categories = hocon_editor::get_categories();

                for category in &categories {
                    changed |= hocon_editor::render_category(
                        ui,
                        category,
                        &mut document,
                        &self.search_query,
                    );
                }

                // Render uncategorized fields
                changed |= hocon_editor::render_uncategorized(
                    ui,
                    &mut document,
                    &categories,
                    &self.search_query,
                );
            });

        if changed {
            // Update state with modified document
            self.editor_state = EditorState::EditingParsed {
                original_path: original_path.clone(),
                backup_path: backup_path.clone(),
                document,
                is_modified: true,
                edit_mode,
            };
        }

        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);

        // Action buttons - always visible at bottom
        ui.horizontal(|ui| {
            // Make Save button more prominent with green color when there are changes
            let save_button = if is_modified {
                egui::Button::new("💾 Save Changes").fill(egui::Color32::from_rgb(0, 120, 0))
            } else {
                egui::Button::new("💾 Save Changes")
            };

            if ui
                .add_sized([180.0, 40.0], save_button)
                .on_hover_text("Encrypt and save changes to save.bin (Ctrl+S)")
                .clicked()
            {
                self.handle_save_changes_parsed();
            }

            if ui
                .add_sized([120.0, 40.0], egui::Button::new("❌ Cancel"))
                .on_hover_text("Discard changes and return to welcome screen (Esc)")
                .clicked()
            {
                self.editor_state = EditorState::Welcome;
                self.status_message.clear();
            }
        });

        ui.add_space(10.0); // Space before footer
    }
}
