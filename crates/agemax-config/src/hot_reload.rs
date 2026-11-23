use crate::schema::AgeMaxConfig;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<AgeMaxConfig>,
}

impl ConfigWatcher {
    pub fn watch(config_path: PathBuf) -> notify::Result<Self> {
        let (tx, rx) = channel();
        let path_clone = config_path.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() {
                        std::thread::sleep(Duration::from_millis(50));
                        if let Ok(cfg) = AgeMaxConfig::load_from_path(&path_clone) {
                            let _ = tx.send(cfg);
                        }
                    }
                }
            },
            notify::Config::default(),
        )?;

        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
            let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
        }

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    pub fn try_recv(&self) -> Option<AgeMaxConfig> {
        self.rx.try_recv().ok()
    }
}
