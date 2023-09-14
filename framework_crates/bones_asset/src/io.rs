use std::path::PathBuf;

use bones_utils::HashMap;

use crate::{AssetLoc, AssetLocRef};

/// [`AssetIo`] is a trait that is implemented for backends capable of loading all the games assets
/// and returning the raw bytes stored in asset files.
pub trait AssetIo: Sync + Send {
    /// List the names of the non-core asset pack folders that are installed.
    ///
    /// These names, are not necessarily the names of the pack, but the names of the folders that
    /// they are located in. These names can be used to load files from the pack in the
    /// [`load_file()`][Self::load_file] method.
    fn enumerate_packs(&self) -> anyhow::Result<Vec<String>>;
    /// Get the binary contents of an asset.
    ///
    /// The `pack_folder` is the name of a folder returned by
    /// [`enumerate_packs()`][Self::enumerate_packs], or [`None`] to refer to the core pack.
    fn load_file(&self, loc: AssetLocRef) -> anyhow::Result<Vec<u8>>;

    /// Subscribe to asset changes.
    fn watch(&self) -> Option<async_channel::Receiver<AssetLoc>>;
}

/// [`AssetIo`] implementation that loads from the filesystem.
#[cfg(not(target_arch = "wasm32"))]
pub struct FileAssetIo {
    /// The directory to load the core asset pack.
    pub core_dir: PathBuf,
    /// The directory to load the asset packs from.
    pub packs_dir: PathBuf,
    /// Receiver for asset changed events.
    pub change_events: Option<async_channel::Receiver<AssetLoc>>,
    /// Filesystem watcher if enabled.
    pub watcher: Option<Box<dyn notify::Watcher + Sync + Send>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileAssetIo {
    /// Create a new [`FileAssetIo`].
    pub fn new(core_dir: &std::path::Path, packs_dir: &std::path::Path, watch: bool) -> Self {
        let cwd = std::env::current_dir().unwrap();
        let core_dir = cwd.join(core_dir);
        let packs_dir = cwd.join(packs_dir);
        let mut watcher = None;
        let mut change_events = None;
        if watch {
            use notify::{RecursiveMode, Result, Watcher};
            let (sender, receiver) = async_channel::bounded(20);

            let core_dir_ = core_dir.clone();
            let packs_dir_ = packs_dir.clone();
            notify::recommended_watcher(move |res: Result<notify::Event>| {
                match res {
                    Ok(event) => match &event.kind {
                        notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                            for path in event.paths {
                                let (path, pack) = if let Ok(path) = path.strip_prefix(&core_dir_) {
                                    (path, None)
                                } else if let Ok(path) = path.strip_prefix(&packs_dir_) {
                                    let pack =
                                        path.iter().next().unwrap().to_str().unwrap().to_string();
                                    let path = path.strip_prefix(&pack).unwrap();
                                    (path, Some(pack))
                                } else {
                                    continue;
                                };
                                sender
                                    .send_blocking(AssetLoc {
                                        path: path.into(),
                                        pack,
                                    })
                                    .unwrap();
                            }
                        }
                        _ => (),
                    },
                    // TODO: Log asset errors with tracing.
                    // We should use the [`tracing`](https://docs.rs/tracing/latest/tracing/) crate
                    // to log an error message here instead of using `eprintln!()`.
                    Err(e) => eprintln!("watch error: {e:?}"),
                }
            })
            .and_then(|mut w| {
                if core_dir.exists() {
                    w.watch(&core_dir, RecursiveMode::Recursive)?;
                }
                if packs_dir.exists() {
                    w.watch(&packs_dir, RecursiveMode::Recursive)?;
                }

                watcher = Some(Box::new(w) as _);
                change_events = Some(receiver);
                Ok(())
            })
            .map_err(|e| {
                eprintln!("watch error: {e:?}");
                // TODO: Log asset errors with tracing.
            })
            .ok();
        }
        Self {
            core_dir: core_dir.clone(),
            packs_dir: packs_dir.clone(),
            change_events,
            watcher,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl AssetIo for FileAssetIo {
    fn enumerate_packs(&self) -> anyhow::Result<Vec<String>> {
        if !self.packs_dir.exists() {
            return Ok(Vec::new());
        }

        // List the folders in the asset packs dir.
        let dirs = std::fs::read_dir(&self.packs_dir)?
            .map(|entry| {
                let entry = entry?;
                let name = entry
                    .file_name()
                    .to_str()
                    .expect("non-unicode filename")
                    .to_owned();
                Ok::<_, std::io::Error>(name)
            })
            .filter(|x| {
                x.as_ref()
                    .map(|name| self.packs_dir.join(name).is_dir())
                    .unwrap_or(true)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(dirs)
    }

    fn load_file(&self, loc: AssetLocRef) -> anyhow::Result<Vec<u8>> {
        let base_dir = match loc.pack {
            Some(folder) => self.packs_dir.join(folder),
            None => self.core_dir.clone(),
        };
        // Make sure absolute paths are relative to pack.
        let path = if loc.path.is_absolute() {
            loc.path.strip_prefix("/").unwrap()
        } else {
            loc.path
        };
        let path = base_dir.join(path);
        Ok(std::fs::read(path)?)
    }

    fn watch(&self) -> Option<async_channel::Receiver<AssetLoc>> {
        self.change_events.clone()
    }
}

/// Dummy [`AssetIo`] implementation used for debugging or as a placeholder.
pub struct DummyIo {
    core: HashMap<PathBuf, Vec<u8>>,
    packs: HashMap<String, HashMap<PathBuf, Vec<u8>>>,
}

impl DummyIo {
    /// Initialize a new [`DummyIo`] from an iterator of `(string_path, byte_data)` items.
    pub fn new<'a, I: IntoIterator<Item = (&'a str, Vec<u8>)>>(core: I) -> Self {
        Self {
            core: core
                .into_iter()
                .map(|(p, d)| (PathBuf::from(p), d))
                .collect(),
            packs: Default::default(),
        }
    }
}

impl AssetIo for DummyIo {
    fn enumerate_packs(&self) -> anyhow::Result<Vec<String>> {
        Ok(self.packs.keys().cloned().collect())
    }

    fn load_file(&self, loc: AssetLocRef) -> anyhow::Result<Vec<u8>> {
        let err = || {
            anyhow::format_err!(
                "File not found: `{:?}` in pack `{:?}`",
                loc.path,
                loc.pack.unwrap_or("[core]")
            )
        };
        if let Some(pack_folder) = loc.pack {
            self.packs
                .get(pack_folder)
                .ok_or_else(err)?
                .get(loc.path)
                .cloned()
                .ok_or_else(err)
        } else {
            self.core.get(loc.path).cloned().ok_or_else(err)
        }
    }

    fn watch(&self) -> Option<async_channel::Receiver<AssetLoc>> {
        None
    }
}
