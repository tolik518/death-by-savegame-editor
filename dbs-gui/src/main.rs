use eframe::egui;
use dbs_core::{decrypt, encrypt, calc_checksum};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([518.0, 300.0])
            .with_max_inner_size([518.0, 800.0])
            .with_min_inner_size([518.0, 300.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Death by Savegame Editor",
        options,
        Box::new(|cc| {
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

#[derive(Default)]
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
}

#[derive(Default, PartialEq)]
enum OperationMode {
    #[default]
    Decrypt,
    Encrypt,
}

impl SaveEditorApp {
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

                        // Create payload preview (first 500 chars)
                        let preview_bytes = &unpacked.payload[..unpacked.payload.len().min(500)];
                        self.payload_preview = String::from_utf8_lossy(preview_bytes).to_string();

                        // Write decrypted payload
                        match std::fs::write(&self.plain_path, &unpacked.payload) {
                            Ok(_) => {
                                self.status_message = format!(
                                    "✓ Successfully decrypted to: {}\n\
                                     Checksum: {} ({})\n\
                                     Cipher length: {} bytes\n\
                                     Payload length: {} bytes",
                                    self.plain_path,
                                    if self.checksum_stored == self.checksum_calc { "OK" } else { "MISMATCH" },
                                    if self.checksum_stored == self.checksum_calc {
                                        format!("0x{:08x}", self.checksum_calc)
                                    } else {
                                        format!("stored=0x{:08x} calc=0x{:08x}", self.checksum_stored, self.checksum_calc)
                                    },
                                    self.cipher_len,
                                    unpacked.payload.len()
                                );
                                self.needs_resize = true;
                            }
                            Err(e) => {
                                self.error_message = format!("Failed to write plaintext file: {}", e);
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
                                    "✓ Successfully encrypted to: {}\n\
                                     Payload length: {} bytes\n\
                                     Cipher length: {} bytes",
                                    self.cipher_path,
                                    payload_len,
                                    cipher_len
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
        let version = env!("CARGO_PKG_VERSION");

        egui::CentralPanel::default().show(ctx, |ui| {
            // add Heading with a version with a small font right after the heading
            ui.heading(format!("Death by Savegame Editor v{}", version));

            ui.add_space(10.0);

            ui.separator();
            ui.add_space(10.0);

            // Operation mode selection
            ui.horizontal(|ui| {
                ui.label("Mode:");
                ui.radio_value(&mut self.operation_mode, OperationMode::Decrypt, "🔓 Decrypt");
                ui.radio_value(&mut self.operation_mode, OperationMode::Encrypt, "🔒 Encrypt");
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
                ui.collapsing("📄 Payload Preview (first 500 bytes)", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.payload_preview.as_str())
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
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
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                518.0,
                550.0
            )));
        }
    }
}

