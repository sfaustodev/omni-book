//! Bottom-right toast queue — CAD-25 Fase B Slice 4 §2.15. Transient feedback
//! (note saved, vault switched, error). Toasts expire after a few seconds and
//! anchor to the screen edge (not the editor pane), per Q-11.

use crate::app::OmniNoteApp;
use egui::RichText;
use std::time::{Duration, Instant};

/// Severity drives the leading glyph + accent.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

#[derive(Clone)]
pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub expires: Instant,
}

impl Toast {
    fn new(text: impl Into<String>, kind: ToastKind, secs: u64) -> Self {
        Self {
            text: text.into(),
            kind,
            expires: Instant::now() + Duration::from_secs(secs),
        }
    }
}

impl OmniNoteApp {
    pub fn toast_info(&mut self, text: impl Into<String>) {
        self.toasts.push(Toast::new(text, ToastKind::Info, 3));
    }
    pub fn toast_success(&mut self, text: impl Into<String>) {
        self.toasts.push(Toast::new(text, ToastKind::Success, 3));
    }
    pub fn toast_error(&mut self, text: impl Into<String>) {
        self.toasts.push(Toast::new(text, ToastKind::Error, 6));
    }

    pub fn show_toasts(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.toasts.retain(|t| t.expires > now);
        if self.toasts.is_empty() {
            return;
        }
        // Keep repainting while toasts are live so they disappear on time even idle.
        ctx.request_repaint_after(Duration::from_millis(250));

        // Severity glyph colors come from the live theme tokens, so they track the
        // active preset (and fold to the accent under high-contrast) instead of
        // hardcoded hues. Falls back to the dark preset on the no-vault screen.
        let theme = self
            .vault
            .as_ref()
            .map(|v| crate::app::theme_for_config(&v.config))
            .unwrap_or_else(crate::theme::Theme::obsidian_dark);

        let mut offset = 16.0;
        // Render newest-last, stacking upward from the bottom-right corner.
        for toast in self.toasts.clone().iter().rev() {
            let (glyph, color) = match toast.kind {
                ToastKind::Info => ("ℹ", theme.accent),
                ToastKind::Success => ("✓", theme.status_done()),
                ToastKind::Error => ("⚠", theme.status_blocked()),
            };
            let id = egui::Id::new(("toast", toast.expires));
            egui::Area::new(id)
                .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -offset])
                .interactable(false)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(glyph).color(color));
                            ui.label(&toast.text);
                        });
                    });
                });
            offset += 44.0;
        }
    }
}
