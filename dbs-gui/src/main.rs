use dbs_core::{calc_checksum, decrypt, encrypt};
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let icon = include_bytes!("../../data/images/logo-square.png");
    let icon_data = eframe::icon_data::from_png_bytes(icon).expect("Icon could not be loaded");
    let version = env!("CARGO_PKG_VERSION");
    let app_name = format!("Death by Savegame Editor v{}", version);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([518.0, 330.0])
            .with_max_inner_size([518.0, 800.0])
            .with_min_inner_size([518.0, 330.0])
            .with_resizable(true)
            .with_icon(icon_data)
            .with_app_id(app_name.clone())
            .with_title(app_name.clone()),
        ..Default::default()
    };

    eframe::run_native(
        &app_name,
        options,
        Box::new(|cc| {
            // Install image loaders to support PNG images
            egui_extras::install_image_loaders(&cc.egui_ctx);

            // Set up custom fonts if needed
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(SaveEditorApp::default()))
        }),
    )
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let fonts = egui::FontDefinitions::default();
    // You can add custom fonts here if needed
    ctx.set_fonts(fonts);
}

struct SaveEditorApp {
    // File paths
    cipher_path: String,
    plain_path: String,

    // Operation state
    operation_mode: OperationMode,
    status_message: String,
    error_message: String,

    // Decryption info
    checksum_stored: u32,
    checksum_calc: u32,
    extra4: u32,
    padlen: u8,
    cipher_len: usize,
    payload_preview: String,

    // UI state
    needs_resize: bool,
    is_dark_mode: bool,
}

impl Default for SaveEditorApp {
    fn default() -> Self {
        let (cipher_path, plain_path) = Self::get_default_paths();
        Self {
            cipher_path,
            plain_path,
            operation_mode: OperationMode::default(),
            status_message: String::new(),
            error_message: String::new(),
            checksum_stored: 0,
            checksum_calc: 0,
            extra4: 0,
            padlen: 0,
            cipher_len: 0,
            payload_preview: String::new(),
            needs_resize: false,
            is_dark_mode: true,
        }
    }
}

#[derive(Default, PartialEq)]
enum OperationMode {
    #[default]
    Decrypt,
    Encrypt,
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

    fn get_default_paths() -> (String, String) {
        let save_dir = match Self::get_save_directory() {
            Some(dir) => dir,
            None => return (String::new(), String::new()),
        };

        let cipher_path = save_dir.join("save.bin");

        // Only use the path if the file exists
        if cipher_path.exists() {
            let plain_path = save_dir.join("save.hocon");
            (
                cipher_path.to_string_lossy().to_string(),
                plain_path.to_string_lossy().to_string(),
            )
        } else {
            (String::new(), String::new())
        }
    }

    fn decrypt_file(&mut self) {
        self.error_message.clear();
        self.status_message.clear();

        match std::fs::read(&self.cipher_path) {
            Ok(enc) => {
                self.cipher_len = enc.len();

                match decrypt(&enc) {
                    Ok(unpacked) => {
                        self.checksum_stored = unpacked.checksum;
                        self.checksum_calc = calc_checksum(&unpacked.payload);
                        self.extra4 = unpacked.extra4;
                        self.padlen = unpacked.padlen;
                        self.payload_preview =
                            String::from_utf8_lossy(&unpacked.payload).to_string();

                        // Write decrypted payload
                        match std::fs::write(&self.plain_path, &unpacked.payload) {
                            Ok(_) => {
                                self.status_message = format!(
                                    "Successfully decrypted to: {}\n\
                                     Checksum: {} ({})\n\
                                     Cipher length: {} bytes\n\
                                     Payload length: {} bytes\
                                     Keys:\n\
                                        - extra4: 0x{:08x}\n\
                                        - key: {:?}",
                                    self.plain_path,
                                    if self.checksum_stored == self.checksum_calc {
                                        "OK"
                                    } else {
                                        "MISMATCH"
                                    },
                                    if self.checksum_stored == self.checksum_calc {
                                        format!("0x{:08x}", self.checksum_calc)
                                    } else {
                                        format!(
                                            "stored=0x{:08x} calc=0x{:08x}",
                                            self.checksum_stored, self.checksum_calc
                                        )
                                    },
                                    self.cipher_len,
                                    unpacked.payload.len(),
                                    self.extra4,
                                    dbs_core::KEY,
                                );
                                self.needs_resize = true;
                            }
                            Err(e) => {
                                self.error_message =
                                    format!("Failed to write plaintext file: {}", e);
                                self.needs_resize = true;
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = format!("Decryption failed: {:?}", e);
                        self.needs_resize = true;
                    }
                }
            }
            Err(e) => {
                self.error_message = format!("Failed to read cipher file: {}", e);
                self.needs_resize = true;
            }
        }
    }

    fn encrypt_file(&mut self) {
        self.error_message.clear();
        self.status_message.clear();

        match std::fs::read(&self.plain_path) {
            Ok(payload) => {
                let payload_len = payload.len();

                match encrypt(&payload) {
                    Ok(enc) => {
                        let cipher_len = enc.len();

                        match std::fs::write(&self.cipher_path, &enc) {
                            Ok(_) => {
                                self.status_message = format!(
                                    "Successfully encrypted to: {}\n\
                                     Checksum: {}\n\
                                     Payload length: {} bytes\n\
                                     Cipher length: {} bytes\
                                      Keys:\n\
                                        - extra4: 0x{:08x}\n\
                                        - padlen: {} bytes\n\
                                        - key: {:?}",
                                    self.cipher_path,
                                    format!("0x{:08x}", calc_checksum(&payload)),
                                    payload_len,
                                    cipher_len,
                                    dbs_core::EXTRA4,
                                    (8 - (payload_len % 8)) % 8,
                                    dbs_core::KEY,
                                );
                                self.needs_resize = true;
                            }
                            Err(e) => {
                                self.error_message = format!("Failed to write cipher file: {}", e);
                                self.needs_resize = true;
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = format!("Encryption failed: {:?}", e);
                        self.needs_resize = true;
                    }
                }
            }
            Err(e) => {
                self.error_message = format!("Failed to read plaintext file: {}", e);
                self.needs_resize = true;
            }
        }
    }
}

impl eframe::App for SaveEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set theme based on toggle
        if self.is_dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Theme toggle in top right corner
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                if ui
                    .button(if self.is_dark_mode { "🌙" } else { "☀" })
                    .clicked()
                {
                    self.is_dark_mode = !self.is_dark_mode;
                }
            });

            // Logo image
            ui.add(
                egui::Image::new(egui::include_image!("../../data/images/logo-long-x2.png"))
                    .fit_to_original_size(1.0)
                    .max_height(40.0),
            );

            ui.add_space(10.0);

            ui.separator();
            ui.add_space(10.0);

            // Operation mode selection
            ui.horizontal(|ui| {
                ui.label("Mode:");
                ui.radio_value(
                    &mut self.operation_mode,
                    OperationMode::Decrypt,
                    "🔓 Decrypt",
                );
                ui.radio_value(
                    &mut self.operation_mode,
                    OperationMode::Encrypt,
                    "🔒 Encrypt",
                );
            });

            ui.add_space(15.0);

            // File selection based on mode
            match self.operation_mode {
                OperationMode::Decrypt => {
                    ui.group(|ui| {
                        ui.heading("Decrypt Save File");
                        ui.add_space(5.0);

                        // Input file (encrypted)
                        ui.horizontal(|ui| {
                            ui.label("Encrypted save:");
                            ui.text_edit_singleline(&mut self.cipher_path);
                            if ui.button("Browse...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Save files", &["bin", "save"])
                                    .add_filter("All files", &["*"])
                                    .pick_file()
                                {
                                    self.cipher_path = path.display().to_string();
                                }
                            }
                        });

                        ui.add_space(5.0);

                        // Output file (plaintext)
                        ui.horizontal(|ui| {
                            ui.label("Output plaintext:");
                            ui.text_edit_singleline(&mut self.plain_path);
                            if ui.button("Browse...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("HOCON files", &["hocon", "conf"])
                                    .add_filter("Text files", &["txt"])
                                    .add_filter("All files", &["*"])
                                    .save_file()
                                {
                                    self.plain_path = path.display().to_string();
                                }
                            }
                        });

                        ui.add_space(10.0);

                        if ui.button("🔓 Decrypt Save File").clicked() {
                            self.decrypt_file();
                        }
                    });
                }
                OperationMode::Encrypt => {
                    ui.group(|ui| {
                        ui.heading("Encrypt Payload");
                        ui.add_space(5.0);

                        // Input file (plaintext)
                        ui.horizontal(|ui| {
                            ui.label("Plaintext payload:");
                            ui.text_edit_singleline(&mut self.plain_path);
                            if ui.button("Browse...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("HOCON files", &["hocon", "conf"])
                                    .add_filter("Text files", &["txt"])
                                    .add_filter("All files", &["*"])
                                    .pick_file()
                                {
                                    self.plain_path = path.display().to_string();
                                }
                            }
                        });

                        ui.add_space(5.0);

                        // Output file (encrypted)
                        ui.horizontal(|ui| {
                            ui.label("Output encrypted:");
                            ui.text_edit_singleline(&mut self.cipher_path);
                            if ui.button("Browse...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Save files", &["bin", "save"])
                                    .add_filter("All files", &["*"])
                                    .save_file()
                                {
                                    self.cipher_path = path.display().to_string();
                                }
                            }
                        });

                        ui.add_space(10.0);

                        if ui.button("🔒 Encrypt Payload").clicked() {
                            self.encrypt_file();
                        }
                    });
                }
            }

            ui.add_space(15.0);

            // Status messages
            if !self.error_message.is_empty() {
                ui.group(|ui| {
                    ui.colored_label(egui::Color32::RED, "Error");
                    ui.label(&self.error_message);
                });
            }

            if !self.status_message.is_empty() {
                ui.group(|ui| {
                    ui.colored_label(egui::Color32::GREEN, "Success");
                    ui.label(&self.status_message);
                });
            }

            // Payload preview for decrypt mode
            if self.operation_mode == OperationMode::Decrypt && !self.payload_preview.is_empty() {
                ui.add_space(10.0);
                ui.collapsing("Payload Preview:", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.payload_preview.as_str())
                                    .code_editor()
                                    .desired_width(f32::INFINITY),
                            );
                        });
                });
            }

            ui.add_space(15.0);
            ui.separator();

            // Footer with info
            ui.horizontal(|ui| {
                ui.label("by Anatolij <tolik518> Vasilev");
            });
        });

        // Auto-resize window when content changes
        if self.needs_resize {
            self.needs_resize = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(518.0, 550.0)));
        }
    }
}
