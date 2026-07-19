// Filesystem watcher (feature #8): reflect edits made to the vault outside the
// app (Obsidian, a text editor, `git pull`). A background `notify` watcher flips
// an atomic flag; the app drains it each frame and offers a reload.

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct VaultWatcher {
    _watcher: RecommendedWatcher,
    flag: Arc<AtomicBool>,
}

impl VaultWatcher {
    pub fn spawn(root: &Path) -> Option<Self> {
        let flag = Arc::new(AtomicBool::new(false));
        let sink = flag.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                let touches_md = ev
                    .paths
                    .iter()
                    .any(|p| p.extension().map(|e| e == "md").unwrap_or(false));
                let relevant = matches!(
                    ev.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                );
                if touches_md && relevant {
                    sink.store(true, Ordering::Relaxed);
                }
            }
        })
        .ok()?;
        watcher.watch(root, RecursiveMode::Recursive).ok()?;
        Some(Self {
            _watcher: watcher,
            flag,
        })
    }

    /// True if ≥1 `.md` change landed since the last call; resets the flag.
    pub fn drain(&self) -> bool {
        self.flag.swap(false, Ordering::Relaxed)
    }
}
