//! Filesystem watcher: wraps `notify` and forwards relevant change events over a
//! channel. The callback also pokes egui from the watcher thread so external edits
//! repaint promptly without busy-looping the UI.

use std::path::Path;
use std::sync::mpsc::{channel, Receiver};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

pub struct VaultWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<Event>,
}

impl VaultWatcher {
    /// Start watching `root` recursively. Repaints `ctx` on every event.
    pub fn new(root: &Path, ctx: egui::Context) -> Option<Self> {
        let (tx, rx) = channel::<Event>();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                // Ignore send errors: a closed receiver just means the app is gone.
                let _ = tx.send(event);
                ctx.request_repaint();
            }
        })
        .ok()?;
        watcher.watch(root, RecursiveMode::Recursive).ok()?;
        Some(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Drain pending events; returns true if any touched a note we care about.
    pub fn drain_relevant(&self) -> bool {
        let mut relevant = false;
        while let Ok(event) = self.rx.try_recv() {
            if event.paths.iter().any(|p| is_relevant(p)) {
                relevant = true;
            }
        }
        relevant
    }
}

/// A path is relevant when it's a loadable note outside the metadata/build dirs.
fn is_relevant(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if s.contains("/.omninote/")
        || s.contains("/_attachments/")
        || s.contains("/.git/")
        || s.contains("/target/")
        || s.contains("/node_modules/")
    {
        return false;
    }
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md") | Some("txt") | Some("env")
    )
}

#[cfg(test)]
mod tests {
    use super::is_relevant;
    use std::path::Path;

    #[test]
    fn markdown_note_is_relevant() {
        assert!(is_relevant(Path::new("/vault/Notes/idea.md")));
    }

    #[test]
    fn metadata_and_build_dirs_ignored() {
        assert!(!is_relevant(Path::new("/vault/.omninote/config.json")));
        assert!(!is_relevant(Path::new("/vault/_attachments/x.png")));
        assert!(!is_relevant(Path::new("/vault/target/debug/foo.md")));
        assert!(!is_relevant(Path::new("/vault/.git/COMMIT_EDITMSG")));
    }

    #[test]
    fn non_note_extension_ignored() {
        assert!(!is_relevant(Path::new("/vault/Notes/diagram.png")));
    }
}
