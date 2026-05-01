use eframe::egui;
use egui::RichText;
use std::path::PathBuf;
use crate::vault::Vault;

pub struct CadernoApp {
    pub vault: Option<Vault>,
    pub active_idx: Option<usize>,
    pub editing: bool,
    pub query: String,
    pub show_settings: bool,
    pub show_new: bool,
    pub show_import: bool,
    pub rename_idx: Option<usize>,
    pub rename_value: String,
    pub dirty: bool,
    pub last_save: std::time::Instant,
    pub error_msg: Option<String>,
}

impl CadernoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let last_vault = dirs::config_dir()
            .map(|d| d.join("caderno").join("last_vault"))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(PathBuf::from)
            .filter(|p| p.exists());

        let vault = last_vault.and_then(|p| Vault::open(p).ok());
        if let Some(v) = &vault {
            cc.egui_ctx.set_visuals(if v.config.dark_mode {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
        }

        Self {
            vault, active_idx: None, editing: false,
            query: String::new(), show_settings: false,
            show_new: false, show_import: false,
            rename_idx: None, rename_value: String::new(),
            dirty: false, last_save: std::time::Instant::now(),
            error_msg: None,
        }
    }

    pub fn save_last_vault(&self) {
        if let Some(v) = &self.vault {
            if let Some(d) = dirs::config_dir() {
                let dir = d.join("caderno");
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(dir.join("last_vault"), v.root.to_string_lossy().as_bytes());
            }
        }
    }

    pub fn pick_vault(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Escolha (ou crie) uma pasta pra ser seu vault")
            .pick_folder() {
            match Vault::open(path) {
                Ok(v) => {
                    self.vault = Some(v);
                    self.active_idx = None;
                    self.save_last_vault();
                }
                Err(e) => self.error_msg = Some(e),
            }
        }
    }
}

impl eframe::App for CadernoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.vault.is_none() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(120.0);
                    ui.heading("📓 Caderno");
                    ui.add_space(8.0);
                    ui.label("Escolha uma pasta pra ser seu vault.");
                    ui.label(RichText::new("Mesma pasta funciona no Obsidian — formatos compatíveis.").size(11.0).weak());
                    ui.add_space(20.0);
                    if ui.button("📂 Abrir / Criar Vault").clicked() {
                        self.pick_vault();
                    }
                });
            });
            return;
        }

        // TODO Claude Code: implementar conforme SPEC.md
        // - Sidebar com árvore (vault.list_folders + vault.notes)
        // - Editor com auto-save (dirty + last_save.elapsed > 600ms -> vault.save_note)
        // - Modais: settings, new, import, confirm
        // - Atalhos: Ctrl+N, Ctrl+E, Ctrl+K, Ctrl+,, Ctrl+Shift+D, Ctrl+= (autoformat)
        // - PDF preview (lopdf::extract_text + open::that pra abrir)
        // - Backlinks: notas com linked_note == active.frontmatter.id || content contém [[active.title]]

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Vault aberto. UI em construção — ver SPEC.md.");
            if let Some(v) = &self.vault {
                ui.label(format!("Notas: {}", v.notes.len()));
                ui.label(format!("Pastas: {}", v.list_folders().len()));
            }
        });

        if let Some(err) = self.error_msg.clone() {
            egui::Window::new("Erro").collapsible(false).show(ctx, |ui| {
                ui.label(&err);
                if ui.button("OK").clicked() { self.error_msg = None; }
            });
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(v) = &self.vault { let _ = v.save_config(); }
    }
}
