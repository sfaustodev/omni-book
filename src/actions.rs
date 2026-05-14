//! Pure action handlers for UI buttons. Decoupled from egui so each handler
//! can be exercised under `cargo test` without an `eframe::CreationContext`.
//!
//! Each `pub fn` here is the same logic that the corresponding UI closure used
//! to inline. UI code in `ui_*.rs` calls these directly so the action surface
//! stays single-source-of-truth.

use crate::types::{ConfirmAction, Note, NoteType};
use crate::vault::Vault;
use std::path::{Path, PathBuf};

// Confirmation flow ---------------------------------------------------------

pub fn request_delete_note(confirm: &mut Option<ConfirmAction>, id: String) {
    *confirm = Some(ConfirmAction::DeleteNote(id));
}

pub fn request_delete_folder(confirm: &mut Option<ConfirmAction>, rel: PathBuf) {
    *confirm = Some(ConfirmAction::DeleteFolder(rel));
}

pub fn cancel_confirm(confirm: &mut Option<ConfirmAction>) {
    *confirm = None;
}

/// Apply a pending DeleteNote confirmation. Removes the note from disk + memory
/// and clears `active_note` if the deletion targets it. Returns Ok even when the
/// id is unknown (the confirmation is consumed regardless).
pub fn confirm_delete_note(
    vault: &mut Vault,
    active: &mut Option<Note>,
    id: &str,
) -> Result<(), String> {
    if let Some(idx) = vault.notes.iter().position(|n| n.frontmatter.id == id) {
        vault.delete_note(idx)?;
        if active.as_ref().is_some_and(|n| n.frontmatter.id == id) {
            *active = None;
        }
    }
    Ok(())
}

/// Apply a pending DeleteFolder confirmation. Removes the folder recursively
/// from disk + memory. Drops `active_note` if it lived under the deleted folder.
pub fn confirm_delete_folder(
    vault: &mut Vault,
    active: &mut Option<Note>,
    rel: &Path,
) -> Result<(), String> {
    vault.delete_folder(rel)?;
    if let Some(n) = active {
        if n.rel_path.starts_with(rel) {
            *active = None;
        }
    }
    Ok(())
}

// Filter + search -----------------------------------------------------------

pub fn set_type_filter(filter: &mut Option<NoteType>, t: Option<NoteType>) {
    *filter = t;
}

pub fn set_query(query: &mut String, q: String) {
    *query = q;
}

/// Pure filter: notes matching the query (case-insensitive, title or tag) and
/// optional type filter. Returns indices into `vault.notes` so the caller can
/// borrow them however it likes.
///
/// Wired into `ui_sidebar` in a follow-up PR — the current sidebar inlines an
/// equivalent filter while iterating the tree.
#[allow(dead_code)]
pub fn filtered_note_indices(
    vault: &Vault,
    query: &str,
    type_filter: Option<NoteType>,
) -> Vec<usize> {
    let q = query.trim().to_lowercase();
    vault
        .notes
        .iter()
        .enumerate()
        .filter(|(_, n)| match type_filter {
            Some(t) => n.frontmatter.note_type == t,
            None => true,
        })
        .filter(|(_, n)| {
            if q.is_empty() {
                return true;
            }
            n.title.to_lowercase().contains(&q)
                || n.frontmatter
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&q))
        })
        .map(|(i, _)| i)
        .collect()
}

// Editor toggles ------------------------------------------------------------

pub fn toggle_edit(editing: &mut bool, active: &Option<Note>) {
    if active.is_some() {
        *editing = !*editing;
    }
}

// External-change conflict resolution --------------------------------------

/// User chose "Recarregar" — drop local edits, reload from disk via the watcher
/// pipeline. Caller is expected to call `vault.reload_notes()` and refresh
/// active_note from disk; this fn just resets the flags.
pub fn external_change_reload(
    vault: &mut Vault,
    active: &mut Option<Note>,
    dirty: &mut bool,
    pending: &mut bool,
) {
    vault.reload_notes();
    if let Some(n) = active {
        if let Some(fresh) = vault
            .notes
            .iter()
            .find(|x| x.frontmatter.id == n.frontmatter.id)
            .cloned()
        {
            *active = Some(fresh);
        } else {
            *active = None;
        }
    }
    *dirty = false;
    *pending = false;
}

/// User chose "Manter edits" — keep current buffer, mark dirty so next debounce
/// flush overwrites the external change.
pub fn external_change_keep(dirty: &mut bool, pending: &mut bool) {
    *dirty = true;
    *pending = false;
}

// Settings ------------------------------------------------------------------
//
// Wired into the Settings modal in a follow-up PR — the current modal mutates
// `vault.config` inline while holding a `&mut self.vault` borrow and defers
// `save_config` until on_exit, so swapping in these handlers needs a tiny
// borrow refactor.

#[allow(dead_code)]
pub fn reset_settings(vault: &mut Vault) -> Result<(), String> {
    vault.config = crate::types::AppConfig::default();
    vault.save_config()
}

#[allow(dead_code)]
pub fn set_font_family(vault: &mut Vault, f: crate::types::FontFamily) -> Result<(), String> {
    vault.config.font_family = f;
    vault.save_config()
}

// Imports -------------------------------------------------------------------

/// Import a PDF: extract text, create a Resumo note with that text, also copy
/// the original PDF into _attachments and reference it. Selects the new note.
pub fn import_pdf(
    vault: &mut Vault,
    active: &mut Option<Note>,
    editing: &mut bool,
    error_msg: &mut Option<String>,
    src: &Path,
) {
    let content = match crate::pdf::extract_text(src) {
        Ok(t) => t,
        Err(e) => {
            *error_msg = Some(e);
            return;
        }
    };
    let title = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("PDF")
        .to_string();
    match vault.create_note(None, &title, NoteType::Resumo) {
        Ok(mut note) => {
            note.content = content;
            if let Ok(name) = vault.import_attachment(src) {
                note.frontmatter.attachments.push(name);
            }
            let _ = vault.save_note(&note);
            sync_in_memory(vault, &note);
            *active = Some(note);
            *editing = false;
        }
        Err(e) => *error_msg = Some(e),
    }
}

/// Import a Claude chat (`.json` or `.md`) as a Resumo note.
pub fn import_chat(
    vault: &mut Vault,
    active: &mut Option<Note>,
    editing: &mut bool,
    error_msg: &mut Option<String>,
    src: &Path,
) {
    let content = match crate::import::import_claude_chat(src) {
        Ok(c) => c,
        Err(e) => {
            *error_msg = Some(e);
            return;
        }
    };
    let title = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Chat")
        .to_string();
    match vault.create_note(None, &title, NoteType::Resumo) {
        Ok(mut note) => {
            note.content = content;
            let _ = vault.save_note(&note);
            sync_in_memory(vault, &note);
            *active = Some(note);
            *editing = false;
        }
        Err(e) => *error_msg = Some(e),
    }
}

/// Import a Claude artifact (code file) as a Codigo note with a fenced code block.
pub fn import_artifact(
    vault: &mut Vault,
    active: &mut Option<Note>,
    editing: &mut bool,
    error_msg: &mut Option<String>,
    src: &Path,
) {
    let content = match crate::import::import_claude_artifact(src) {
        Ok(c) => c,
        Err(e) => {
            *error_msg = Some(e);
            return;
        }
    };
    let title = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Artefato")
        .to_string();
    match vault.create_note(None, &title, NoteType::Codigo) {
        Ok(mut note) => {
            note.content = content;
            let _ = vault.save_note(&note);
            sync_in_memory(vault, &note);
            *active = Some(note);
            *editing = false;
        }
        Err(e) => *error_msg = Some(e),
    }
}

// Attachments ---------------------------------------------------------------

/// Copy `src` into `_attachments/`, append the resulting name to the active
/// note's frontmatter, and return the wikilink string the editor should insert
/// at the cursor (image embed for image extensions, file embed otherwise).
pub fn attach_file_to_active(
    vault: &mut Vault,
    active: &mut Option<Note>,
    src: &Path,
) -> Result<String, String> {
    let name = vault.import_attachment(src)?;
    let wikilink = format!("![[{name}]]");
    if let Some(note) = active {
        note.frontmatter.attachments.push(name);
    }
    Ok(wikilink)
}

// Backlinks -----------------------------------------------------------------

/// Pure scan: indices of notes whose `content` contains a wikilink to `title`
/// (case-insensitive). The caller borrows the notes themselves.
///
/// Wired into `ui_editor` in a follow-up PR — the current backlinks panel uses
/// a slightly different scan that also matches on `frontmatter.linked_note`.
#[allow(dead_code)]
pub fn backlinks_to(vault: &Vault, title: &str) -> Vec<usize> {
    let needle = title.to_lowercase();
    vault
        .notes
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            crate::wikilinks::extract(&n.content)
                .into_iter()
                .any(|w| match w {
                    crate::wikilinks::Wikilink::Note(t) => t.to_lowercase() == needle,
                    _ => false,
                })
        })
        .map(|(i, _)| i)
        .collect()
}

/// Create a new linked note titled `title` and return it. Used by the
/// "➕ criar" button on the backlinks panel for unresolved links.
pub fn create_link_to_new(vault: &mut Vault, title: &str) -> Result<Note, String> {
    vault.create_note(None, title, NoteType::Resumo)
}

// Internal helpers ----------------------------------------------------------

fn sync_in_memory(vault: &mut Vault, note: &Note) {
    if let Some(slot) = vault
        .notes
        .iter_mut()
        .find(|n| n.frontmatter.id == note.frontmatter.id)
    {
        *slot = note.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AppConfig, FontFamily};
    use std::io::Write;

    fn temp_vault() -> (Vault, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let v = Vault::open(dir.path().to_path_buf()).unwrap();
        (v, dir)
    }

    fn tmp_file_with(content: &str, ext: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(ext).tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    // ----- Confirm flow -----

    #[test]
    fn request_delete_note_sets_confirm() {
        let mut confirm: Option<ConfirmAction> = None;
        request_delete_note(&mut confirm, "abc".into());
        assert!(matches!(confirm, Some(ConfirmAction::DeleteNote(ref s)) if s == "abc"));
    }

    #[test]
    fn request_delete_folder_sets_confirm() {
        let mut confirm: Option<ConfirmAction> = None;
        request_delete_folder(&mut confirm, PathBuf::from("Sub"));
        assert!(
            matches!(confirm, Some(ConfirmAction::DeleteFolder(ref p)) if p == &PathBuf::from("Sub"))
        );
    }

    #[test]
    fn cancel_confirm_clears() {
        let mut confirm = Some(ConfirmAction::DeleteNote("x".into()));
        cancel_confirm(&mut confirm);
        assert!(confirm.is_none());
    }

    #[test]
    fn confirm_delete_note_removes_and_clears_active() {
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "Bye", NoteType::Resumo).unwrap();
        let mut active = Some(note.clone());
        confirm_delete_note(&mut v, &mut active, &note.frontmatter.id).unwrap();
        assert_eq!(v.notes.len(), 0);
        assert!(active.is_none());
    }

    #[test]
    fn confirm_delete_note_keeps_active_when_different() {
        let (mut v, _d) = temp_vault();
        let kept = v.create_note(None, "Keep", NoteType::Resumo).unwrap();
        let target = v.create_note(None, "Drop", NoteType::Resumo).unwrap();
        let mut active = Some(kept.clone());
        confirm_delete_note(&mut v, &mut active, &target.frontmatter.id).unwrap();
        assert_eq!(v.notes.len(), 1);
        assert_eq!(active.as_ref().unwrap().frontmatter.id, kept.frontmatter.id);
    }

    #[test]
    fn confirm_delete_note_unknown_id_is_noop() {
        let (mut v, _d) = temp_vault();
        v.create_note(None, "Stay", NoteType::Resumo).unwrap();
        let mut active = None;
        confirm_delete_note(&mut v, &mut active, "n_unknown").unwrap();
        assert_eq!(v.notes.len(), 1);
    }

    #[test]
    fn confirm_delete_folder_clears_active_when_inside() {
        let (mut v, _d) = temp_vault();
        let abs = v.create_folder(None, "Pasta").unwrap();
        let rel = abs.strip_prefix(&v.root).unwrap().to_path_buf();
        let inside = v
            .create_note(Some(&rel), "Inside", NoteType::Resumo)
            .unwrap();
        let mut active = Some(inside);
        confirm_delete_folder(&mut v, &mut active, &rel).unwrap();
        assert!(active.is_none());
    }

    #[test]
    fn confirm_delete_folder_keeps_active_when_outside() {
        let (mut v, _d) = temp_vault();
        let abs = v.create_folder(None, "Pasta").unwrap();
        let rel = abs.strip_prefix(&v.root).unwrap().to_path_buf();
        let outside = v.create_note(None, "Outside", NoteType::Resumo).unwrap();
        let mut active = Some(outside.clone());
        confirm_delete_folder(&mut v, &mut active, &rel).unwrap();
        assert_eq!(active.unwrap().frontmatter.id, outside.frontmatter.id);
    }

    // ----- Filters + search -----

    #[test]
    fn set_type_filter_round_trip() {
        let mut filter = None;
        set_type_filter(&mut filter, Some(NoteType::Codigo));
        assert_eq!(filter, Some(NoteType::Codigo));
        set_type_filter(&mut filter, None);
        assert_eq!(filter, None);
    }

    #[test]
    fn set_query_replaces_value() {
        let mut q = String::new();
        set_query(&mut q, "rust".into());
        assert_eq!(q, "rust");
        set_query(&mut q, String::new());
        assert!(q.is_empty());
    }

    #[test]
    fn filtered_indices_match_title_substring_case_insensitive() {
        let (mut v, _d) = temp_vault();
        v.create_note(None, "Rust Notes", NoteType::Resumo).unwrap();
        v.create_note(None, "Python", NoteType::Codigo).unwrap();
        let result = filtered_note_indices(&v, "rust", None);
        assert_eq!(result.len(), 1);
        assert!(v.notes[result[0]].title.contains("Rust"));
    }

    #[test]
    fn filtered_indices_match_tag_substring() {
        let (mut v, _d) = temp_vault();
        let mut n = v.create_note(None, "X", NoteType::Resumo).unwrap();
        n.frontmatter.tags = vec!["rust".into()];
        v.save_note(&n).unwrap();
        v.reload_notes();
        let result = filtered_note_indices(&v, "rust", None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filtered_indices_respect_type_filter() {
        let (mut v, _d) = temp_vault();
        v.create_note(None, "A", NoteType::Resumo).unwrap();
        v.create_note(None, "B", NoteType::Codigo).unwrap();
        let result = filtered_note_indices(&v, "", Some(NoteType::Codigo));
        assert_eq!(result.len(), 1);
        assert_eq!(v.notes[result[0]].title, "B");
    }

    #[test]
    fn filtered_indices_empty_query_returns_all() {
        let (mut v, _d) = temp_vault();
        v.create_note(None, "A", NoteType::Resumo).unwrap();
        v.create_note(None, "B", NoteType::Resumo).unwrap();
        let result = filtered_note_indices(&v, "", None);
        assert_eq!(result.len(), 2);
    }

    // ----- Editor toggles -----

    #[test]
    fn toggle_edit_flips_when_active_present() {
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "X", NoteType::Resumo).unwrap();
        let mut editing = false;
        let active = Some(note);
        toggle_edit(&mut editing, &active);
        assert!(editing);
        toggle_edit(&mut editing, &active);
        assert!(!editing);
    }

    #[test]
    fn toggle_edit_noop_when_no_active() {
        let mut editing = false;
        toggle_edit(&mut editing, &None);
        assert!(!editing);
    }

    // ----- External change conflict -----

    #[test]
    fn external_change_reload_refreshes_active_from_disk() {
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "A", NoteType::Resumo).unwrap();
        // Mutate disk content out from under us
        let mut updated = note.clone();
        updated.content = "external edit".to_string();
        v.save_note(&updated).unwrap();
        let mut active = Some(note);
        let mut dirty = true;
        let mut pending = true;
        external_change_reload(&mut v, &mut active, &mut dirty, &mut pending);
        assert!(!dirty);
        assert!(!pending);
        assert_eq!(active.unwrap().content, "external edit");
    }

    #[test]
    fn external_change_reload_drops_active_when_deleted_externally() {
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "Gone", NoteType::Resumo).unwrap();
        std::fs::remove_file(&note.path).unwrap();
        let mut active = Some(note);
        let mut dirty = true;
        let mut pending = true;
        external_change_reload(&mut v, &mut active, &mut dirty, &mut pending);
        assert!(active.is_none());
    }

    #[test]
    fn external_change_keep_marks_dirty_clears_pending() {
        let mut dirty = false;
        let mut pending = true;
        external_change_keep(&mut dirty, &mut pending);
        assert!(dirty);
        assert!(!pending);
    }

    // ----- Settings -----

    #[test]
    fn reset_settings_restores_defaults_and_persists() {
        let (mut v, _d) = temp_vault();
        v.config.dark_mode = true;
        v.config.font_size = 22.0;
        reset_settings(&mut v).unwrap();
        let default = AppConfig::default();
        assert_eq!(v.config.dark_mode, default.dark_mode);
        assert!((v.config.font_size - default.font_size).abs() < f32::EPSILON);
        // persisted to disk
        let reopened = Vault::open(v.root.clone()).unwrap();
        assert!(!reopened.config.dark_mode);
    }

    #[test]
    fn set_font_family_writes_through() {
        let (mut v, _d) = temp_vault();
        set_font_family(&mut v, FontFamily::Monospace).unwrap();
        let reopened = Vault::open(v.root.clone()).unwrap();
        assert_eq!(reopened.config.font_family, FontFamily::Monospace);
    }

    // ----- Imports -----

    #[test]
    fn import_chat_creates_note_and_selects() {
        let (mut v, _d) = temp_vault();
        let json = r#"{"name":"Conv","chat_messages":[{"sender":"human","text":"oi"}]}"#;
        let f = tmp_file_with(json, ".json");
        let mut active = None;
        let mut editing = true;
        let mut err = None;
        import_chat(&mut v, &mut active, &mut editing, &mut err, f.path());
        assert!(err.is_none());
        assert_eq!(v.notes.len(), 1);
        assert!(active.is_some());
        assert!(!editing);
        assert!(active.unwrap().content.contains("oi"));
    }

    #[test]
    fn import_chat_records_error_on_bad_json() {
        let (mut v, _d) = temp_vault();
        let f = tmp_file_with("not json", ".json");
        let mut active = None;
        let mut editing = false;
        let mut err = None;
        import_chat(&mut v, &mut active, &mut editing, &mut err, f.path());
        assert!(err.is_some());
        assert_eq!(v.notes.len(), 0);
        assert!(active.is_none());
    }

    #[test]
    fn import_artifact_creates_codigo_note_with_fence() {
        let (mut v, _d) = temp_vault();
        let f = tmp_file_with("fn x() {}", ".rs");
        let mut active = None;
        let mut editing = true;
        let mut err = None;
        import_artifact(&mut v, &mut active, &mut editing, &mut err, f.path());
        assert!(err.is_none());
        let note = active.unwrap();
        assert_eq!(note.frontmatter.note_type, NoteType::Codigo);
        assert!(note.content.contains("```rust"));
    }

    #[test]
    fn import_pdf_records_error_on_bad_pdf() {
        let (mut v, _d) = temp_vault();
        let f = tmp_file_with("not a pdf", ".pdf");
        let mut active = None;
        let mut editing = false;
        let mut err = None;
        import_pdf(&mut v, &mut active, &mut editing, &mut err, f.path());
        assert!(err.is_some());
        assert_eq!(v.notes.len(), 0);
    }

    // ----- Attachments -----

    #[test]
    fn attach_file_returns_image_embed_and_records_attachment() {
        let (mut v, dir) = temp_vault();
        let src = dir.path().join("photo.png");
        std::fs::write(&src, b"img").unwrap();
        let mut active = Some(v.create_note(None, "X", NoteType::Resumo).unwrap());
        let wl = attach_file_to_active(&mut v, &mut active, &src).unwrap();
        assert_eq!(wl, "![[photo.png]]");
        assert_eq!(active.as_ref().unwrap().frontmatter.attachments.len(), 1);
    }

    #[test]
    fn attach_file_propagates_vault_error_when_src_missing() {
        let (mut v, dir) = temp_vault();
        let src = dir.path().join("missing.bin");
        let mut active = None;
        let res = attach_file_to_active(&mut v, &mut active, &src);
        assert!(res.is_err());
    }

    // ----- Backlinks + create_link_to_new -----

    #[test]
    fn backlinks_match_case_insensitive_wikilink() {
        let (mut v, _d) = temp_vault();
        let target = v.create_note(None, "Target", NoteType::Resumo).unwrap();
        let mut linker = v.create_note(None, "Linker", NoteType::Resumo).unwrap();
        linker.content = "see [[target]] here".to_string();
        v.save_note(&linker).unwrap();
        v.reload_notes();
        let result = backlinks_to(&v, &target.title);
        assert_eq!(result.len(), 1);
        assert_eq!(v.notes[result[0]].title, "Linker");
    }

    #[test]
    fn backlinks_skip_image_and_file_embeds() {
        let (mut v, _d) = temp_vault();
        v.create_note(None, "Photo", NoteType::Resumo).unwrap();
        let mut linker = v.create_note(None, "Linker", NoteType::Resumo).unwrap();
        // Embed (not link) — should not count as backlink to a note titled "photo.png"
        linker.content = "![[photo.png]]".to_string();
        v.save_note(&linker).unwrap();
        v.reload_notes();
        let result = backlinks_to(&v, "photo.png");
        assert!(result.is_empty());
    }

    #[test]
    fn create_link_to_new_returns_resumo() {
        let (mut v, _d) = temp_vault();
        let note = create_link_to_new(&mut v, "Brand New").unwrap();
        assert_eq!(note.frontmatter.note_type, NoteType::Resumo);
        assert!(v
            .notes
            .iter()
            .any(|n| n.frontmatter.id == note.frontmatter.id));
    }
}
