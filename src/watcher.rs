use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};

/// Wraps a notify watcher + its receiver. Keeping the watcher alive in
/// the struct is important — dropping it stops events.
pub struct VaultWatcher {
    _watcher: RecommendedWatcher,
    pub rx: Receiver<notify::Result<notify::Event>>,
}

impl VaultWatcher {
    pub fn new(root: &Path) -> Result<Self, String> {
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .map_err(|e| e.to_string())?;
        watcher
            .watch(root, RecursiveMode::Recursive)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Drain pending events. Returns Vec of paths that changed (Create/Modify/Remove)
    /// filtered to .md files only, excluding paths inside .omninote/ and _attachments/.
    pub fn drain_md_changes(&self) -> Vec<std::path::PathBuf> {
        let mut paths = Vec::new();
        while let Ok(res) = self.rx.try_recv() {
            if let Ok(event) = res {
                use notify::EventKind;
                let interesting = matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                );
                if !interesting {
                    continue;
                }
                for p in event.paths {
                    let s = p.to_string_lossy();
                    if s.contains("/.omninote/") || s.contains("\\.omninote\\") {
                        continue;
                    }
                    if s.contains("/_attachments/") || s.contains("\\_attachments\\") {
                        continue;
                    }
                    if p.extension().and_then(|e| e.to_str()) != Some("md") {
                        continue;
                    }
                    paths.push(p);
                }
            }
        }
        paths
    }
}
