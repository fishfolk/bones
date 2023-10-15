use std::path::PathBuf;

use anyhow::Context;
use async_channel::Sender;
use bones_utils::{default, futures::future::Boxed as BoxedFuture, HashMap};

use crate::{AssetLocRef, ChangedAsset};

/// [`AssetIo`] is a trait that is implemented for backends capable of loading all the games assets
/// and returning the raw bytes stored in asset files.
pub trait AssetIo: Sync + Send {
    /// List the names of the non-core asset pack folders that are installed.
    ///
    /// These names, are not necessarily the names of the pack, but the names of the folders that
    /// they are located in. These names can be used to load files from the pack in the
    /// [`load_file()`][Self::load_file] method.
    fn enumerate_packs(&self) -> BoxedFuture<anyhow::Result<Vec<String>>>;

    /// Get the binary contents of an asset.
    ///
    /// The `pack_folder` is the name of a folder returned by
    /// [`enumerate_packs()`][Self::enumerate_packs], or [`None`] to refer to the core pack.
    fn load_file(&self, loc: AssetLocRef) -> BoxedFuture<anyhow::Result<Vec<u8>>>;

    /// Subscribe to asset changes.
    ///
    /// Returns `true` if this [`AssetIo`] implementation supports watching for changes.
    fn watch(&self, change_sender: Sender<ChangedAsset>) -> bool {
        let _ = change_sender;
        false
    }
}

/// [`AssetIo`] implementation that loads from the filesystem.
#[cfg(not(target_arch = "wasm32"))]
pub struct FileAssetIo {
    /// The directory to load the core asset pack.
    pub core_dir: PathBuf,
    /// The directory to load the asset packs from.
    pub packs_dir: PathBuf,
    /// Filesystem watcher if enabled.
    pub watcher: bones_utils::parking_lot::Mutex<Option<Box<dyn notify::Watcher + Sync + Send>>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileAssetIo {
    /// Create a new [`FileAssetIo`].
    pub fn new(core_dir: &std::path::Path, packs_dir: &std::path::Path) -> Self {
        let cwd = std::env::current_dir().unwrap();
        let core_dir = cwd.join(core_dir);
        let packs_dir = cwd.join(packs_dir);
        Self {
            core_dir: core_dir.clone(),
            packs_dir: packs_dir.clone(),
            watcher: bones_utils::parking_lot::Mutex::new(None),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl AssetIo for FileAssetIo {
    fn enumerate_packs(&self) -> BoxedFuture<anyhow::Result<Vec<String>>> {
        if !self.packs_dir.exists() {
            return Box::pin(async { Ok(Vec::new()) });
        }

        let packs_dir = self.packs_dir.clone();
        Box::pin(async move {
            // List the folders in the asset packs dir.
            let dirs = std::fs::read_dir(&packs_dir)?
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
                        .map(|name| packs_dir.join(name).is_dir())
                        .unwrap_or(true)
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok(dirs)
        })
    }

    fn load_file(&self, loc: AssetLocRef) -> BoxedFuture<anyhow::Result<Vec<u8>>> {
        let packs_dir = self.packs_dir.clone();
        let core_dir = self.core_dir.clone();
        let loc = loc.to_owned();

        // TODO: Load files asynchronously.
        Box::pin(async move {
            let base_dir = match loc.pack {
                Some(folder) => packs_dir.join(folder),
                None => core_dir.clone(),
            };
            // Make sure absolute paths are relative to pack.
            let path = if loc.path.is_absolute() {
                loc.path.strip_prefix("/").unwrap().to_owned()
            } else {
                loc.path
            };
            let path = base_dir.join(path);
            std::fs::read(&path).with_context(|| format!("Could not load file: {path:?}"))
        })
    }

    fn watch(&self, sender: Sender<ChangedAsset>) -> bool {
        use notify::{RecursiveMode, Result, Watcher};

        let core_dir_ = self.core_dir.clone();
        let packs_dir_ = self.packs_dir.clone();
        notify::recommended_watcher(move |res: Result<notify::Event>| match res {
            Ok(event) => match &event.kind {
                notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                    for path in event.paths {
                        let (path, pack) = if let Ok(path) = path.strip_prefix(&core_dir_) {
                            (path, None)
                        } else if let Ok(path) = path.strip_prefix(&packs_dir_) {
                            let pack = path.iter().next().unwrap().to_str().unwrap().to_string();
                            let path = path.strip_prefix(&pack).unwrap();
                            (path, Some(pack))
                        } else {
                            continue;
                        };
                        sender
                            .send_blocking(ChangedAsset::Loc(crate::AssetLoc {
                                path: path.into(),
                                pack,
                            }))
                            .unwrap();
                    }
                }
                _ => (),
            },
            Err(e) => tracing::error!("watch error: {e:?}"),
        })
        .and_then(|mut w| {
            if self.core_dir.exists() {
                w.watch(&self.core_dir, RecursiveMode::Recursive)?;
            }
            if self.packs_dir.exists() {
                w.watch(&self.packs_dir, RecursiveMode::Recursive)?;
            }

            *self.watcher.lock() = Some(Box::new(w) as _);
            Ok(())
        })
        .map_err(|e| {
            tracing::error!("watch error: {e:?}");
        })
        .map(|_| true)
        .unwrap_or(false)
    }
}

/// Asset IO implementation that loads assets from a URL.
pub struct WebAssetIo {
    /// The base URL to load assets from.
    pub asset_url: String,
}

impl WebAssetIo {
    /// Create a new [`WebAssetIo`] with the given URL as the core pack root URL.
    pub fn new(asset_url: &str) -> Self {
        let mut asset_url = asset_url.to_string();
        if !asset_url.ends_with('/') {
            asset_url.push('/');
        }
        Self { asset_url }
    }
}

impl AssetIo for WebAssetIo {
    fn enumerate_packs(&self) -> BoxedFuture<anyhow::Result<Vec<String>>> {
        Box::pin(async move { Ok(default()) })
    }

    fn load_file(&self, loc: AssetLocRef) -> BoxedFuture<anyhow::Result<Vec<u8>>> {
        let loc = loc.to_owned();
        let asset_url = self.asset_url.clone();
        Box::pin(async move {
            tracing::info!(?loc, "Loading asset in WebAssetIo");
            if loc.pack.is_some() {
                return Err(anyhow::format_err!("Cannot load asset packs on WASM yet"));
            }
            let url = format!("{}{}", asset_url, loc.path.to_str().unwrap());
            let (sender, receiver) = async_channel::bounded(1);
            let req = ehttp::Request::get(&url);
            ehttp::fetch(req, move |resp| {
                sender.send_blocking(resp.map(|resp| resp.bytes)).unwrap();
            });
            let result = receiver
                .recv()
                .await
                .unwrap()
                .map_err(|e| anyhow::format_err!("{e}"))
                .with_context(|| format!("Could not download file: {url}"))?;

            Ok(result)
        })
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
    fn enumerate_packs(&self) -> BoxedFuture<anyhow::Result<Vec<String>>> {
        let packs = self.packs.keys().cloned().collect();
        Box::pin(async { Ok(packs) })
    }

    fn load_file(&self, loc: AssetLocRef) -> BoxedFuture<anyhow::Result<Vec<u8>>> {
        let err = || {
            anyhow::format_err!(
                "File not found: `{:?}` in pack `{:?}`",
                loc.path,
                loc.pack.unwrap_or("[core]")
            )
        };
        let data = (|| {
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
        })();
        Box::pin(async move { data })
    }
}
