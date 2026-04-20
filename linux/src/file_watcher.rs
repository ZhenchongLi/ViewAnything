use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::time::Duration;

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    pub fn new<F>(path: &Path, callback: F) -> Option<Self>
    where
        F: Fn() + 'static,
    {
        let (tx, rx) = channel::<notify::Event>();

        let mut watcher: RecommendedWatcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            })
            .ok()?;

        let watch_path = path.parent().unwrap_or(path);
        watcher.watch(watch_path, RecursiveMode::NonRecursive).ok()?;

        let target_path: PathBuf = path.to_path_buf();
        let callback: Rc<dyn Fn()> = Rc::new(callback);
        let pending = Rc::new(std::cell::Cell::new(false));

        glib::source::timeout_add_local(Duration::from_millis(100), move || {
            let mut relevant = false;
            while let Ok(event) = rx.try_recv() {
                if event.paths.iter().any(|p| p == &target_path) {
                    relevant = true;
                }
            }
            if relevant && !pending.get() {
                pending.set(true);
                let cb = callback.clone();
                let pending_clone = pending.clone();
                glib::timeout_add_local_once(Duration::from_millis(250), move || {
                    pending_clone.set(false);
                    cb();
                });
            }
            glib::ControlFlow::Continue
        });

        Some(Self { _watcher: watcher })
    }
}
