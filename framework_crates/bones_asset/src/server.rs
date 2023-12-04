use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::Context;
use append_only_vec::AppendOnlyVec;
use async_channel::{Receiver, Sender};
use bevy_tasks::IoTaskPool;
use dashmap::{
    mapref::one::{
        MappedRef as MappedMapRef, MappedRefMut as MappedMapRefMut, Ref as MapRef,
        RefMut as MapRefMut,
    },
    DashMap,
};
use once_cell::sync::Lazy;
use semver::VersionReq;
use serde::{de::DeserializeSeed, Deserialize};
use ulid::Ulid;

use crate::prelude::*;
use bones_utils::{
    parking_lot::{MappedMutexGuard, MutexGuard},
    *,
};

mod schema_loader;

/// Struct responsible for loading assets into it's contained [`AssetStore`], using an [`AssetIo`]
/// implementation.
#[derive(HasSchema, Deref, DerefMut, Clone)]
pub struct AssetServer {
    #[deref]
    /// The asset server inner state.
    pub inner: Arc<AssetServerInner>,
    /// The [`AssetIo`] implementation used to load assets.
    pub io: Arc<dyn AssetIo>,
}

/// The inner state of the asset server.
pub struct AssetServerInner {
    /// The version of the game. This is used to evaluate whether asset packs are compatible with
    /// the game.
    pub game_version: Mutex<Version>,
    /// The asset store.
    pub store: AssetStore,
    /// Sender for asset changes, used by the [`AssetIo`] implementation or other tasks to trigger
    /// hot reloads.
    pub asset_change_send: Sender<ChangedAsset>,
    /// Receiver for asset changes, used to implement hot reloads.
    pub asset_change_recv: Receiver<ChangedAsset>,
    /// The asset load progress.
    pub load_progress: AssetLoadProgress,
}

/// An ID for an asset that has changed.
pub enum ChangedAsset {
    /// The location of an asset that has changed.
    Loc(AssetLoc),
    /// The [`Cid`] of an asset that has changed.
    Handle(UntypedHandle),
}

impl Default for AssetServer {
    fn default() -> Self {
        Self {
            inner: default(),
            io: Arc::new(DummyIo::new([])),
        }
    }
}

impl Default for AssetServerInner {
    fn default() -> Self {
        let (asset_change_send, asset_change_recv) = async_channel::unbounded();
        Self {
            game_version: Mutex::new(Version::new(0, 0, 0)),
            store: default(),
            load_progress: default(),
            asset_change_send,
            asset_change_recv,
        }
    }
}

/// YAML format for the core asset pack's `pack.yaml` file.
#[derive(Debug, Clone, Deserialize)]
pub struct CorePackfileMeta {
    /// The path to the root asset for the pack.
    pub root: PathBuf,
    /// The paths to schema definitions to be loaded from this pack.
    #[serde(default)]
    pub schemas: Vec<PathBuf>,
}

/// YAML format for asset packs' `pack.yaml` file.
#[derive(Debug, Clone, Deserialize)]
pub struct PackfileMeta {
    /// User friendly pack name.
    pub name: String,
    /// The unique ID of the asset pack.
    pub id: AssetPackId,
    /// The version of the asset pack.
    pub version: Version,
    /// The required game version to be compatible with this asset pack.
    pub game_version: VersionReq,
    /// The paths to schema definitions to be loaded from this pack.
    #[serde(default)]
    pub schemas: Vec<PathBuf>,
    /// The path to the root asset for the pack.
    pub root: PathBuf,
}

/// The [`AssetPackId`] of the core pack.
pub static CORE_PACK_ID: Lazy<AssetPackId> =
    Lazy::new(|| AssetPackId::new_with_ulid("core", Ulid(0)).unwrap());

/// An error returned when an asset pack does not support the game version.
#[derive(Debug, Clone)]
pub struct IncompatibleGameVersionError {
    /// The version of the game that the pack is not compatible with.
    pub game_version: Version,
    /// The directory of the pack that
    pub pack_dir: String,
    /// The metadata of the pack that could not be loaded.
    pub pack_meta: PackfileMeta,
}

impl std::error::Error for IncompatibleGameVersionError {}
impl std::fmt::Display for IncompatibleGameVersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Asset pack `{}` v{} from folder `{}` is only compatible with game versions matching {}, not {}",
            self.pack_meta.id,
            self.pack_meta.version,
            self.pack_dir,
            self.pack_meta.game_version,
            self.game_version
        )
    }
}

#[derive(Debug)]
struct LoaderNotFound {
    name: String,
}
impl std::fmt::Display for LoaderNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Schema/loader not found for schema/extension: {}\n\
            You may need to register the asset by calling `MyAsset::schema()`",
            self.name
        )
    }
}
impl std::error::Error for LoaderNotFound {}

impl AssetServer {
    /// Initialize a new [`AssetServer`].
    pub fn new<Io: AssetIo + 'static>(io: Io, version: Version) -> Self {
        Self {
            inner: Arc::new(AssetServerInner {
                game_version: Mutex::new(version),
                ..default()
            }),
            io: Arc::new(io),
        }
    }

    /// Load the bytes of the asset at the given path, but return only the [`Cid`].
    ///
    /// The loaded bytes can be accessed from [`asset_server.store.asset_data`][AssetStore::asset_data]
    /// using the [`Cid`].
    ///
    /// If `force` is false, the bytes will be loaded from cache if possible.
    pub async fn load_asset_bytes(&self, loc: AssetLoc, force: bool) -> anyhow::Result<Cid> {
        // Load asset data from cache if present
        if !force {
            let cid = self
                .store
                .path_handles
                .get(&loc)
                .and_then(|handle| self.store.asset_ids.get(&handle));
            if let Some(cid) = cid {
                return Ok(*cid);
            }
        }

        // Load the data for the asset path
        let data = self.io.load_file(loc.as_ref()).await?;

        // Compute the Cid
        let mut cid = Cid::default();
        cid.update(&data);

        // Insert the data into the cache
        self.store.asset_data.insert(cid, data);

        // Return the Cid
        Ok(cid)
    }

    /// Tell the asset backend to watch for changes and trigger hot reloads for changed assets.
    pub fn watch_for_changes(&self) {
        self.io.watch(self.asset_change_send.clone());
    }

    /// Set the [`AssetIo`] implementation.
    ///
    /// This should almost always be called before calling [`load_assets()`][Self::load_assets].
    pub fn set_io<Io: AssetIo + 'static>(&mut self, io: Io) {
        self.io = Arc::new(io);
    }

    /// Responds to any asset changes reported by the [`AssetIo`] implementation.
    ///
    /// This must be called or asset changes will be ignored. Additionally, the [`AssetIo`]
    /// implementation must be able to detect asset changes or this will do nothing.
    pub fn handle_asset_changes<F: FnMut(&mut AssetServer, UntypedHandle)>(
        &mut self,
        mut handle_change: F,
    ) {
        let mut pending_asset_changes = Vec::new();
        while let Ok(changed) = self.asset_change_recv.try_recv() {
            match changed {
                ChangedAsset::Loc(loc) => {
                    let handle = self.load_asset_forced(loc.as_ref());
                    pending_asset_changes.push(handle);
                }
                ChangedAsset::Handle(handle) => {
                    let entry = self
                        .store
                        .path_handles
                        .iter()
                        .find(|entry| *entry.value() == handle)
                        .unwrap();
                    let loc = entry.key().to_owned();
                    drop(entry);
                    self.load_asset_forced(loc.as_ref());
                    pending_asset_changes.push(handle);
                }
            }
        }

        for handle in pending_asset_changes {
            handle_change(self, handle)
        }
    }

    /// Load all assets. This is usually done in an async task.
    pub async fn load_assets(&self) -> anyhow::Result<()> {
        // Load the core asset pack
        self.load_pack(None).await?;

        // Load the user asset packs
        for pack_dir in self.io.enumerate_packs().await? {
            // Load the asset pack
            let pack_result = self.load_pack(Some(&pack_dir)).await;
            if let Err(e) = pack_result {
                match e.downcast::<IncompatibleGameVersionError>() {
                    // Check for a compatibility error
                    Ok(e) => {
                        tracing::warn!(
                            "Not loading pack `{}` because it requires game version \
                            `{}` and this is version `{}`",
                            e.pack_meta.name,
                            e.pack_meta.game_version,
                            e.game_version,
                        );
                        // Add it to the list of incompatible packs.
                        self.store.incompabile_packs.insert(e.pack_dir, e.pack_meta);
                    }
                    // If this is another kind of error, return the error
                    Err(e) => {
                        return Err(e).context(format!("Error loading asset pack: {pack_dir}"))
                    }
                }
            }
        }

        Ok(())
    }

    /// Load the asset pack with the given folder name, or else the default pack if [`None`].
    pub async fn load_pack(&self, pack: Option<&str>) -> anyhow::Result<AssetPackSpec> {
        // Load the core pack differently
        if pack.is_none() {
            return self
                .load_core_pack()
                .await
                .context("Error loading core asset pack");
        }

        // Load the asset packfile
        let packfile_contents = self
            .io
            .load_file((Path::new("pack.yaml"), pack).into())
            .await?;
        let meta: PackfileMeta = serde_yaml::from_slice(&packfile_contents)?;
        tracing::debug!(?pack, ?meta, "Loaded asset pack meta.");

        // If the game version doesn't match, then don't continue loading this pack.
        if !meta.game_version.matches(&self.game_version()) {
            return Err(IncompatibleGameVersionError {
                game_version: self.game_version(),
                pack_dir: pack.unwrap().to_owned(),
                pack_meta: meta,
            }
            .into());
        }

        // Store the asset pack spec associated to the pack dir name.
        if let Some(pack_dir) = pack {
            self.store.pack_dirs.insert(
                pack_dir.into(),
                AssetPackSpec {
                    id: meta.id,
                    version: meta.version.clone(),
                },
            );
        }

        // Load the schemas
        let schemas = self.load_pack_schemas(pack, &meta.schemas).await?;

        // Load the asset and produce a handle
        let root_loc = AssetLocRef {
            path: &meta.root,
            pack,
        };

        // Return the loaded asset pack.
        let spec = AssetPackSpec {
            id: meta.id,
            version: meta.version.clone(),
        };
        self.store.packs.insert(
            spec.clone(),
            AssetPack {
                name: meta.name,
                id: meta.id,
                version: meta.version,
                game_version: meta.game_version,
                schemas,
                root: default(),
            },
        );
        let root_handle = self.load_asset(root_loc);
        self.store.packs.get_mut(&spec).unwrap().root = root_handle;

        Ok(spec)
    }

    /// Load the core asset pack.
    pub async fn load_core_pack(&self) -> anyhow::Result<AssetPackSpec> {
        // Load the core asset packfile
        let packfile_contents = self
            .io
            .load_file(AssetLocRef {
                path: Path::new("pack.yaml"),
                pack: None,
            })
            .await
            .context("Could not load pack file")?;
        let meta: CorePackfileMeta = serde_yaml::from_slice(&packfile_contents)?;
        tracing::debug!(?meta, "Loaded core pack meta.");

        // Load the asset and produce a handle
        let root_loc = AssetLocRef {
            path: &meta.root,
            pack: None,
        };
        let handle = self.load_asset(root_loc);

        // Load the schemas
        let schemas = self.load_pack_schemas(None, &meta.schemas).await?;

        // Return the loaded asset pack.
        let game_version = self.game_version();

        let id = *CORE_PACK_ID;
        *self.store.core_pack.lock() = Some(AssetPack {
            name: "Core".into(),
            id,
            version: game_version.clone(),
            game_version: VersionReq {
                comparators: [semver::Comparator {
                    op: semver::Op::Exact,
                    major: game_version.major,
                    minor: Some(game_version.minor),
                    patch: Some(game_version.patch),
                    pre: game_version.pre.clone(),
                }]
                .to_vec(),
            },
            schemas,
            root: handle,
        });

        Ok(AssetPackSpec {
            id,
            version: game_version.clone(),
        })
    }

    /// Load an asset.
    pub fn load_asset(&self, loc: AssetLocRef<'_>) -> UntypedHandle {
        self.impl_load_asset(loc, false)
    }

    /// Like [`load_asset()`][Self::load_asset] but forces the asset to reload, even it if has
    /// already been loaded.
    pub fn load_asset_forced(&self, loc: AssetLocRef<'_>) -> UntypedHandle {
        self.impl_load_asset(loc, true)
    }

    fn impl_load_asset(&self, loc: AssetLocRef<'_>, force: bool) -> UntypedHandle {
        // Get the asset pool
        let pool = IoTaskPool::get();

        // Absolutize the asset location
        let loc = AssetLoc {
            path: loc.path.absolutize_from("/").unwrap().into_owned(),
            pack: loc.pack.map(|x| x.to_owned()),
        };

        // If this isn't a force reload
        if !force {
            // And we already have an asset handle created for this path
            if let Some(handle) = self.store.path_handles.get(&loc) {
                // Return the existing handle and stop processing
                return *handle;
            }
        }

        // Determine the handle and insert it into the path_handles map
        let mut should_load = true;
        let handle = *self
            .store
            .path_handles
            .entry(loc.clone())
            // If we've already loaded this asset before
            .and_modify(|handle| {
                // If we aren't forcing a reload and we already have asset data, we don't need to
                // trigger another load.
                if self.store.asset_ids.get(handle).is_some() && !force {
                    should_load = false;
                }
            })
            // If this is an unloaded location, create a new handle for it
            .or_insert(UntypedHandle {
                rid: Ulid::create(),
            });

        if should_load {
            // Add one more asset that needs loading.
            self.load_progress.inc_to_load();

            // Spawn a task to load the asset
            let server = self.clone();
            let loc_ = loc.clone();
            pool.spawn(async move {
                tracing::debug!(?loc, ?force, "Loading asset");
                let loc = loc_;
                let result = async {
                    let cid = server.load_asset_bytes(loc.clone(), force).await?;
                    server.load_progress.inc_downloaded();
                    let data = server
                        .store
                        .asset_data
                        .get(&cid)
                        .expect("asset not loaded")
                        .clone();

                    // Try to load a metadata asset if it has a YAML/JSON extension, if that doesn't work, and
                    // it has a schema not found error, try to load a data asset for the same path, if that
                    // doesn't work and it is an extension not found error, return the metadata error message.
                    let partial = if path_is_metadata(&loc.path) {
                        match server.load_metadata_asset(loc.as_ref(), &data).await {
                            Err(meta_err) => {
                                if meta_err.downcast_ref::<LoaderNotFound>().is_some() {
                                    match server.load_data_asset(loc.as_ref(), &data).await {
                                        Err(data_err) => {
                                            if data_err.downcast_ref::<LoaderNotFound>().is_some() {
                                                Err(meta_err)
                                            } else {
                                                Err(data_err)
                                            }
                                        }
                                        ok => ok,
                                    }
                                } else {
                                    Err(meta_err)
                                }
                            }
                            ok => ok,
                        }
                    } else {
                        server.load_data_asset(loc.as_ref(), &data).await
                    }?;

                    let loaded_asset = LoadedAsset {
                        cid: partial.cid,
                        pack_spec: loc.pack.as_ref().map(|x| {
                            server
                                .store
                                .pack_dirs
                                .get(x.as_str())
                                .expect("Pack dir not loaded properly")
                                .clone()
                        }),
                        loc: loc.to_owned(),
                        dependencies: partial.dependencies,
                        data: partial.data,
                    };

                    // If there is already loaded asset data for this path
                    if let Some((_, cid)) = server.store.asset_ids.remove(&handle) {
                        // Remove the old asset data
                        let (_, previous_asset) = server.store.assets.remove(&cid).unwrap();

                        // Remove the previous asset's reverse dependencies.
                        //
                        // aka. now that we are removing the old asset, none of the assets that the
                        // old asset dependended on should have a reverse dependency record saying that
                        // this asset depends on it.
                        //
                        // In other words, this asset is removed and doesn't depend on anything else
                        // anymore.
                        for dep in previous_asset.dependencies.iter() {
                            server
                                .store
                                .reverse_dependencies
                                .get_mut(dep)
                                .unwrap()
                                .remove(&handle);
                        }

                        // If there are any assets that depended on this asset, they now need to be re-loaded.
                        if let Some(rev_deps) = server.store.reverse_dependencies.get(&handle) {
                            for dep in rev_deps.iter() {
                                server
                                    .asset_change_send
                                    .try_send(ChangedAsset::Handle(*dep))
                                    .unwrap();
                            }
                        }
                    }

                    // Update reverse dependencies
                    for dep in loaded_asset.dependencies.iter() {
                        server
                            .store
                            .reverse_dependencies
                            .entry(*dep)
                            .or_default()
                            .insert(handle);
                    }

                    server.store.asset_ids.insert(handle, partial.cid);
                    server.store.assets.insert(partial.cid, loaded_asset);
                    server.load_progress.inc_loaded();

                    Ok::<_, anyhow::Error>(())
                }
                .await;

                if let Err(e) = result {
                    server.load_progress.inc_errored();
                    tracing::error!("Error loading asset: {e}");
                }
            })
            .detach();
        }

        handle
    }

    async fn load_metadata_asset<'a>(
        &'a self,
        loc: AssetLocRef<'a>,
        contents: &[u8],
    ) -> anyhow::Result<PartialAsset> {
        // Get the schema for the asset
        let filename = loc
            .path
            .file_name()
            .ok_or_else(|| anyhow::format_err!("Invalid asset filename"))?
            .to_str()
            .ok_or_else(|| anyhow::format_err!("Invalid unicode in filename"))?;
        let before_extension = filename.rsplit_once('.').unwrap().0;
        let schema_name = before_extension
            .rsplit_once('.')
            .map(|x| x.1)
            .unwrap_or(before_extension);
        let schema = SCHEMA_REGISTRY
            .schemas
            .iter()
            .find(|schema| {
                schema
                    .type_data
                    .get::<AssetKind>()
                    .and_then(|asset_kind| match asset_kind {
                        AssetKind::Metadata { extension } => Some(extension == schema_name),
                        _ => None,
                    })
                    .unwrap_or(false)
            })
            .ok_or_else(|| LoaderNotFound {
                name: schema_name.into(),
            })?;
        let mut dependencies = Vec::new();

        let mut cid = Cid::default();
        // Use the schema name and the file contents to create a unique, content-addressed ID for
        // the asset.
        cid.update(schema.full_name.as_bytes());
        cid.update(contents);

        let loader = MetaAssetLoadCtx {
            server: self,
            loc,
            schema,
            dependencies: &mut dependencies,
        };
        let data = if loc.path.extension().unwrap().to_str().unwrap() == "json" {
            let mut deserializer = serde_json::Deserializer::from_slice(contents);
            loader.deserialize(&mut deserializer)?
        } else {
            let deserializer = serde_yaml::Deserializer::from_slice(contents);
            loader.deserialize(deserializer)?
        };

        // Update Cid with the Cids of it's dependencies
        dependencies.sort();
        for dep in &dependencies {
            let dep_cid = loop {
                let listener = self.load_progress.listen();
                let Some(cid) = self.store.asset_ids.get(dep) else {
                    listener.await;
                    continue;
                };
                break *cid;
            };
            cid.update(dep_cid.0.as_slice());
        }

        Ok(PartialAsset {
            cid,
            dependencies,
            data,
        })
    }

    async fn load_data_asset<'a>(
        &self,
        loc: AssetLocRef<'a>,
        contents: &'a [u8],
    ) -> anyhow::Result<PartialAsset> {
        // Get the schema for the asset
        let filename = loc
            .path
            .file_name()
            .ok_or_else(|| anyhow::format_err!("Invalid asset filename"))?
            .to_str()
            .ok_or_else(|| anyhow::format_err!("Invalid unicode in filename"))?;
        let (_name, extension) = filename.split_once('.').unwrap();
        let (loader, schema) = SCHEMA_REGISTRY
            .schemas
            .iter()
            .find_map(|schema| {
                if let Some(asset_kind) = schema.type_data.get::<AssetKind>() {
                    match asset_kind {
                        AssetKind::Custom { extensions, loader } => {
                            if extensions
                                .iter()
                                .any(|ext| ext == extension || ext == filename)
                            {
                                Some((loader, schema))
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .ok_or_else(|| LoaderNotFound {
                name: extension.into(),
            })?;

        let dependencies = Arc::new(AppendOnlyVec::new());

        let mut cid = Cid::default();
        // Use the schema name and the file contents to create a unique, content-addressed ID for
        // the asset.
        cid.update(schema.full_name.as_bytes());
        cid.update(contents);

        let ctx = AssetLoadCtx {
            asset_server: self.clone(),
            loc: loc.to_owned(),
            dependencies: dependencies.clone(),
        };
        let sbox = loader.load(ctx, contents).await?;

        // Update Cid with the Cids of it's dependencies
        let mut dependencies = dependencies.iter().cloned().collect::<Vec<_>>();
        dependencies.sort();
        for dep in &dependencies {
            let dep_cid = loop {
                let listener = self.load_progress.listen();
                let Some(cid) = self.store.asset_ids.get(dep) else {
                    listener.await;
                    continue;
                };
                break *cid;
            };
            cid.update(dep_cid.0.as_slice());
        }

        Ok(PartialAsset {
            cid,
            data: sbox,
            dependencies,
        })
    }

    /// Borrow a [`LoadedAsset`] associated to the given handle.
    pub fn get_asset_untyped(&self, handle: UntypedHandle) -> Option<MapRef<Cid, LoadedAsset>> {
        let cid = self.store.asset_ids.get(&handle)?;
        self.store.assets.get(&cid)
    }

    /// Borrow a [`LoadedAsset`] associated to the given handle.
    pub fn get_asset_untyped_mut(
        &self,
        handle: UntypedHandle,
    ) -> Option<MapRefMut<Cid, LoadedAsset>> {
        let cid = self.store.asset_ids.get(&handle)?;
        self.store.assets.get_mut(&cid)
    }

    /// Read the core asset pack.
    ///
    /// # Panics
    ///
    /// Panics if the assets have not be loaded yet with [`AssetServer::load_assets`].
    #[track_caller]
    pub fn core(&self) -> MappedMutexGuard<AssetPack> {
        MutexGuard::map(self.store.core_pack.lock(), |x| x.as_mut().unwrap())
    }

    /// Get the core asset pack's root asset.
    pub fn root<T: HasSchema>(&self) -> MappedMapRef<Cid, LoadedAsset, T> {
        self.get(self.core().root.typed())
    }

    /// Get the core asset pack's root asset as a type-erased [`SchemaBox`].
    pub fn untyped_root(&self) -> MappedMapRef<Cid, LoadedAsset, SchemaBox> {
        self.get_untyped(self.core().root)
    }

    /// Read the loaded asset packs.
    pub fn packs(&self) -> &DashMap<AssetPackSpec, AssetPack> {
        &self.store.packs
    }

    /// Borrow a loaded asset.
    ///
    /// # Panics
    ///
    /// Panics if the asset is not loaded or if the asset asset with the given handle doesn't have a
    /// schema matching `T`.
    #[track_caller]
    pub fn get<T: HasSchema>(&self, handle: Handle<T>) -> MappedMapRef<Cid, LoadedAsset, T> {
        self.try_get(handle).unwrap()
    }

    /// Borrow a loaded asset.
    ///
    /// # Panics
    ///
    /// Panics if the asset is not loaded.
    #[track_caller]
    pub fn get_untyped(&self, handle: UntypedHandle) -> MappedMapRef<Cid, LoadedAsset, SchemaBox> {
        self.try_get_untyped(handle).unwrap()
    }

    /// Borrow a loaded asset.
    ///
    /// # Panics
    ///
    /// Panics if the asset is not loaded.
    #[track_caller]
    pub fn get_untyped_mut(
        &self,
        handle: UntypedHandle,
    ) -> MappedMapRefMut<Cid, LoadedAsset, SchemaBox> {
        self.try_get_untyped_mut(handle).unwrap()
    }

    /// Borrow a loaded asset.
    pub fn try_get<T: HasSchema>(
        &self,
        handle: Handle<T>,
    ) -> Option<MappedMapRef<Cid, LoadedAsset, T>> {
        let cid = self.store.asset_ids.get(&handle.untyped())?;
        Some(MapRef::map(self.store.assets.get(&cid).unwrap(), |x| {
            let asset = &x.data;

            // If this is a handle to a schema box, then return the schema box directly without casting
            if T::schema() == <SchemaBox as HasSchema>::schema() {
                // SOUND: the above comparison verifies that T is concretely a SchemaBox so &Schemabox
                // is the same as &T.
                unsafe { std::mem::transmute(asset) }
            } else {
                asset.cast_ref()
            }
        }))
    }

    /// Borrow a loaded asset.
    pub fn try_get_untyped(
        &self,
        handle: UntypedHandle,
    ) -> Option<MappedMapRef<Cid, LoadedAsset, SchemaBox>> {
        let cid = self.store.asset_ids.get(&handle)?;
        Some(MapRef::map(self.store.assets.get(&cid).unwrap(), |x| {
            &x.data
        }))
    }

    /// Borrow a loaded asset.
    pub fn try_get_untyped_mut(
        &self,
        handle: UntypedHandle,
    ) -> Option<MappedMapRefMut<Cid, LoadedAsset, SchemaBox>> {
        let cid = self.store.asset_ids.get_mut(&handle)?;
        Some(MapRefMut::map(
            self.store.assets.get_mut(&cid).unwrap(),
            |x| &mut x.data,
        ))
    }

    /// Mutably borrow a loaded asset.
    ///
    /// # Panics
    ///
    /// Panics if the asset is not loaded or if the asset asset with the given handle doesn't have a
    /// schema matching `T`.
    #[track_caller]
    pub fn get_mut<T: HasSchema>(
        &mut self,
        handle: &Handle<T>,
    ) -> MappedMapRefMut<Cid, LoadedAsset, T> {
        let cid = self
            .store
            .asset_ids
            .get(&handle.untyped())
            .expect(NO_ASSET_MSG);
        MapRefMut::map(self.store.assets.get_mut(&cid).unwrap(), |x| {
            let asset = &mut x.data;

            // If this is a handle to a schema box, then return the schema box directly without casting
            if T::schema() == <SchemaBox as HasSchema>::schema() {
                // SOUND: the above comparison verifies that T is concretely a SchemaBox so &mut Schemabox
                // is the same as &mut T.
                unsafe { std::mem::transmute(asset) }
            } else {
                asset.cast_mut()
            }
        })
    }

    /// Load the schemas for an asset pack.
    async fn load_pack_schemas(
        &self,
        pack: Option<&str>,
        schema_paths: &[PathBuf],
    ) -> anyhow::Result<Vec<&'static Schema>> {
        let mut schemas = Vec::with_capacity(schema_paths.len());
        for schema_path in schema_paths {
            let contents = self
                .io
                .load_file(AssetLocRef {
                    path: schema_path,
                    pack,
                })
                .await?;

            let pack_schema: schema_loader::PackSchema = serde_yaml::from_slice(&contents)?;
            let schema = pack_schema.0;
            tracing::debug!(?pack, ?schema.name, "Loaded schema from pack.");
            schemas.push(schema);
        }
        Ok(schemas)
    }

    /// Get the game version config, used when making sure asset packs are compatible with this
    /// game version.
    pub fn game_version(&self) -> Version {
        self.game_version.lock().unwrap().clone()
    }

    /// Set the game version config, used when making sure asset packs are compatible with this
    /// game version.
    pub fn set_game_version(&self, version: Version) {
        *self.game_version.lock().unwrap() = version;
    }
}

/// Partial of a [`LoadedAsset`] used internally while loading is in progress.
struct PartialAsset {
    pub cid: Cid,
    pub data: SchemaBox,
    pub dependencies: Vec<UntypedHandle>,
}

const NO_ASSET_MSG: &str = "Asset not loaded";
fn path_is_metadata(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    ext == "yaml" || ext == "yml" || ext == "json"
}

pub use metadata::*;
mod metadata {
    use serde::de::{DeserializeSeed, Error, Unexpected, VariantAccess, Visitor};

    use super::*;

    /// Context provided while loading a metadata asset.
    pub struct MetaAssetLoadCtx<'srv> {
        /// The asset server.
        pub server: &'srv AssetServer,
        /// The dependency list of this asset. This should be updated by asset loaders as
        /// dependencies are added.
        pub dependencies: &'srv mut Vec<UntypedHandle>,
        /// The location that the asset is being loaded from.
        pub loc: AssetLocRef<'srv>,
        /// The schema of the asset being loaded.
        pub schema: &'static Schema,
    }

    impl<'asset, 'de> DeserializeSeed<'de> for MetaAssetLoadCtx<'asset> {
        type Value = SchemaBox;

        fn deserialize<D>(mut self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            // Allocate the object.
            let mut ptr = SchemaBox::default(self.schema);

            SchemaPtrLoadCtx {
                ctx: &mut self,
                ptr: ptr.as_mut(),
            }
            .deserialize(deserializer)?;

            Ok(ptr)
        }
    }

    /// The load context for a [`SchemaRefMut`].
    pub struct SchemaPtrLoadCtx<'a, 'srv, 'ptr> {
        /// The metadata asset load context.
        pub ctx: &'a mut MetaAssetLoadCtx<'srv>,
        /// The pointer to load.
        pub ptr: SchemaRefMut<'ptr>,
    }

    impl<'a, 'srv, 'ptr, 'de> DeserializeSeed<'de> for SchemaPtrLoadCtx<'a, 'srv, 'ptr> {
        type Value = ();

        fn deserialize<D>(mut self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            // Load asset handles.
            if self
                .ptr
                .schema()
                .type_data
                .get::<SchemaAssetHandle>()
                .is_some()
            {
                let path_string = String::deserialize(deserializer)?;
                let mut pack = self.ctx.loc.pack.map(|x| x.to_owned());
                let relative_path;
                if let Some((pack_prefix, path)) = path_string.split_once(':') {
                    let pack_id = LabeledId::new(pack_prefix).map_err(|e| D::Error::custom(format!("Error parsing pack prefix while parsing asset path `{path_string}`: {e}")))?;
                    if pack_prefix == "core" {
                        pack = None;
                    } else {
                        pack = Some(self.ctx
                            .server
                            .store
                            .pack_dirs
                            .iter()
                            .find(|x| x.value().id == pack_id)
                            .map(|x| x.key().to_owned())
                            .ok_or_else(|| {
                                D::Error::custom(format!("Dependent pack {pack_id} not loaded when trying to load asset with path: {path_string}."))
                            })?);
                    }
                    relative_path = path;
                } else {
                    relative_path = &path_string;
                };
                let relative_path = PathBuf::from(relative_path);
                let path = relative_path
                    .absolutize_from(self.ctx.loc.path.parent().unwrap())
                    .unwrap();
                let handle = self.ctx.server.load_asset((&*path, pack.as_deref()).into());
                self.ctx.dependencies.push(handle);
                *self
                    .ptr
                    .try_cast_mut()
                    .map_err(|e| D::Error::custom(e.to_string()))? = handle;
                return Ok(());
            }

            // Use custom asset load or deserialize implementation if present.
            if let Some(custom_loader) = self.ptr.schema().type_data.get::<SchemaMetaAssetLoader>()
            {
                return custom_loader
                    .load(self.ctx, self.ptr, deserializer)
                    .map_err(|e| D::Error::custom(e.to_string()));
            } else if let Some(schema_deserialize) =
                self.ptr.schema().type_data.get::<SchemaDeserialize>()
            {
                return schema_deserialize.deserialize(self.ptr, deserializer);
            }

            match &self.ptr.schema().kind {
                SchemaKind::Struct(s) => {
                    // If this is a newtype struct
                    if s.fields.len() == 1 && s.fields[0].name.is_none() {
                        // Deserialize it as the inner type
                        // SOUND: it is safe to cast a struct with one field to it's field type
                        let ptr = unsafe {
                            SchemaRefMut::from_ptr_schema(self.ptr.as_ptr(), s.fields[0].schema)
                        };
                        SchemaPtrLoadCtx { ptr, ctx: self.ctx }.deserialize(deserializer)?
                    } else {
                        deserializer.deserialize_any(StructVisitor {
                            ptr: self.ptr,
                            ctx: self.ctx,
                        })?
                    }
                }
                SchemaKind::Vec(_) => deserializer.deserialize_seq(VecVisitor {
                    ptr: self.ptr,
                    ctx: self.ctx,
                })?,
                SchemaKind::Map { .. } => deserializer.deserialize_map(MapVisitor {
                    ptr: self.ptr,
                    ctx: self.ctx,
                })?,
                SchemaKind::Enum(_) => deserializer.deserialize_any(EnumVisitor {
                    ptr: self.ptr,
                    ctx: self.ctx,
                })?,
                SchemaKind::Box(_) => SchemaPtrLoadCtx {
                    ctx: self.ctx,
                    ptr: self.ptr.into_box().unwrap(),
                }
                .deserialize(deserializer)?,
                SchemaKind::Primitive(p) => {
                    match p {
                        Primitive::Bool => *self.ptr.cast_mut() = bool::deserialize(deserializer)?,
                        Primitive::U8 => *self.ptr.cast_mut() = u8::deserialize(deserializer)?,
                        Primitive::U16 => *self.ptr.cast_mut() = u16::deserialize(deserializer)?,
                        Primitive::U32 => *self.ptr.cast_mut() = u32::deserialize(deserializer)?,
                        Primitive::U64 => *self.ptr.cast_mut() = u64::deserialize(deserializer)?,
                        Primitive::U128 => *self.ptr.cast_mut() = u128::deserialize(deserializer)?,
                        Primitive::I8 => *self.ptr.cast_mut() = i8::deserialize(deserializer)?,
                        Primitive::I16 => *self.ptr.cast_mut() = i16::deserialize(deserializer)?,
                        Primitive::I32 => *self.ptr.cast_mut() = i32::deserialize(deserializer)?,
                        Primitive::I64 => *self.ptr.cast_mut() = i64::deserialize(deserializer)?,
                        Primitive::I128 => *self.ptr.cast_mut() = i128::deserialize(deserializer)?,
                        Primitive::F32 => *self.ptr.cast_mut() = f32::deserialize(deserializer)?,
                        Primitive::F64 => *self.ptr.cast_mut() = f64::deserialize(deserializer)?,
                        Primitive::String => {
                            *self.ptr.cast_mut() = String::deserialize(deserializer)?
                        }
                        Primitive::Opaque { .. } => {
                            return Err(D::Error::custom(
                                "Opaque types must be #[repr(C)] or have `SchemaDeserialize` type data in order \
                                to be loaded in a metadata asset.",
                            ));
                        }
                    };
                }
            };

            Ok(())
        }
    }

    struct StructVisitor<'a, 'srv, 'ptr> {
        ctx: &'a mut MetaAssetLoadCtx<'srv>,
        ptr: SchemaRefMut<'ptr>,
    }

    impl<'a, 'srv, 'ptr, 'de> Visitor<'de> for StructVisitor<'a, 'srv, 'ptr> {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            // TODO: Write very verbose error messages for metadata asset deserializers.
            // The schema describes the type that we are trying to deserialize, so we should
            // use that information to format a nice-to-read error message that documents
            // what data it's expecting.
            write!(
                formatter,
                "asset metadata matching the schema: {}",
                self.ptr.schema().full_name
            )
        }

        fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let field_count = self.ptr.schema().kind.as_struct().unwrap().fields.len();

            for i in 0..field_count {
                let field = self.ptr.access_mut().field(i).unwrap();
                if seq
                    .next_element_seed(SchemaPtrLoadCtx {
                        ctx: self.ctx,
                        ptr: field.into_schema_ref_mut(),
                    })?
                    .is_none()
                {
                    break;
                }
            }

            Ok(())
        }

        fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            while let Some(key) = map.next_key::<String>()? {
                match self.ptr.access_mut().field(&key) {
                    Ok(field) => {
                        map.next_value_seed(SchemaPtrLoadCtx {
                            ctx: self.ctx,
                            ptr: field.into_schema_ref_mut(),
                        })?;
                    }
                    Err(_) => {
                        let fields = &self.ptr.schema().kind.as_struct().unwrap().fields;
                        let mut msg = format!("unknown field `{key}`, ");
                        if !fields.is_empty() {
                            msg += "expected one of ";
                            for (i, field) in fields.iter().enumerate() {
                                msg += &field
                                    .name
                                    .as_ref()
                                    .map(|x| format!("`{x}`"))
                                    .unwrap_or_else(|| format!("`{i}`"));
                                if i < fields.len() - 1 {
                                    msg += ", "
                                }
                            }
                        } else {
                            msg += "there are no fields"
                        }
                        return Err(A::Error::custom(msg));
                    }
                }
            }

            Ok(())
        }
    }

    struct VecVisitor<'a, 'srv, 'ptr> {
        ctx: &'a mut MetaAssetLoadCtx<'srv>,
        ptr: SchemaRefMut<'ptr>,
    }

    impl<'a, 'srv, 'ptr, 'de> Visitor<'de> for VecVisitor<'a, 'srv, 'ptr> {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            // TODO: Write very verbose error messages for metadata asset deserializers.
            write!(
                formatter,
                "asset metadata matching the schema: {}",
                self.ptr.schema().full_name
            )
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            // SOUND: schema asserts this is a SchemaVec.
            let v = unsafe { &mut *(self.ptr.as_ptr() as *mut SchemaVec) };
            loop {
                let item_schema = v.schema();
                let mut item = SchemaBox::default(item_schema);
                let item_ref = item.as_mut();
                if seq
                    .next_element_seed(SchemaPtrLoadCtx {
                        ctx: self.ctx,
                        ptr: item_ref,
                    })?
                    .is_none()
                {
                    break;
                }
                v.push_box(item);
            }

            Ok(())
        }
    }

    struct MapVisitor<'a, 'srv, 'ptr> {
        ctx: &'a mut MetaAssetLoadCtx<'srv>,
        ptr: SchemaRefMut<'ptr>,
    }

    impl<'a, 'srv, 'ptr, 'de> Visitor<'de> for MapVisitor<'a, 'srv, 'ptr> {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            // TODO: Write very verbose error messages for metadata asset deserializers.
            write!(
                formatter,
                "asset metadata matching the schema: {}",
                self.ptr.schema().full_name
            )
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            // SOUND: schema asserts this is a SchemaMap.
            let v = unsafe { &mut *(self.ptr.as_ptr() as *mut SchemaMap) };
            let is_ustr = v.key_schema() == Ustr::schema();
            if v.key_schema() != String::schema() && !is_ustr {
                return Err(A::Error::custom(
                    "Can only deserialize maps with `String` or `Ustr` keys.",
                ));
            }
            while let Some(key) = map.next_key::<String>()? {
                let key = if is_ustr {
                    SchemaBox::new(ustr(&key))
                } else {
                    SchemaBox::new(key)
                };
                let mut value = SchemaBox::default(v.value_schema());
                map.next_value_seed(SchemaPtrLoadCtx {
                    ctx: self.ctx,
                    ptr: value.as_mut(),
                })?;

                v.insert_box(key, value);
            }
            Ok(())
        }
    }

    struct EnumVisitor<'a, 'srv, 'ptr> {
        ctx: &'a mut MetaAssetLoadCtx<'srv>,
        ptr: SchemaRefMut<'ptr>,
    }

    impl<'a, 'srv, 'ptr, 'de> Visitor<'de> for EnumVisitor<'a, 'srv, 'ptr> {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            // TODO: Write very verbose error messages for metadata asset deserializers.
            write!(
                formatter,
                "asset metadata matching the schema: {}",
                self.ptr.schema().full_name
            )
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            let enum_info = self.ptr.schema().kind.as_enum().unwrap();
            let var_idx = enum_info
                .variants
                .iter()
                .position(|x| x.name == v)
                .ok_or_else(|| E::invalid_value(Unexpected::Str(v), &self))?;

            if !enum_info.variants[var_idx]
                .schema
                .kind
                .as_struct()
                .unwrap()
                .fields
                .is_empty()
            {
                return Err(E::custom(format!(
                    "Cannot deserialize enum variant with fields from string: {v}"
                )));
            }

            // SOUND: we match the cast with the enum tag type.
            unsafe {
                match enum_info.tag_type {
                    EnumTagType::U8 => self.ptr.as_ptr().cast::<u8>().write(var_idx as u8),
                    EnumTagType::U16 => self.ptr.as_ptr().cast::<u16>().write(var_idx as u16),
                    EnumTagType::U32 => self.ptr.as_ptr().cast::<u32>().write(var_idx as u32),
                }
            }

            Ok(())
        }

        fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::EnumAccess<'de>,
        {
            let (value_ptr, var_access) = data.variant_seed(EnumPtrLoadCtx { ptr: self.ptr })?;

            var_access.newtype_variant_seed(SchemaPtrLoadCtx {
                ctx: self.ctx,
                ptr: value_ptr,
            })?;

            Ok(())
        }
    }

    struct EnumPtrLoadCtx<'ptr> {
        ptr: SchemaRefMut<'ptr>,
    }

    impl<'ptr, 'de> DeserializeSeed<'de> for EnumPtrLoadCtx<'ptr> {
        type Value = SchemaRefMut<'ptr>;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let var_name = String::deserialize(deserializer)?;
            let enum_info = self.ptr.schema().kind.as_enum().unwrap();
            let value_offset = self.ptr.schema().field_offsets()[0].1;
            let (var_idx, var_schema) = enum_info
                .variants
                .iter()
                .enumerate()
                .find_map(|(idx, info)| (info.name == var_name).then_some((idx, info.schema)))
                .ok_or_else(|| {
                    D::Error::custom(format!(
                        "Unknown enum variant `{var_name}`, expected one of: {}",
                        enum_info
                            .variants
                            .iter()
                            .map(|x| format!("`{}`", x.name))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                })?;

            // Write the enum variant
            // SOUND: the schema asserts that the write to the enum discriminant is valid
            match enum_info.tag_type {
                EnumTagType::U8 => unsafe { self.ptr.as_ptr().cast::<u8>().write(var_idx as u8) },
                EnumTagType::U16 => unsafe {
                    self.ptr.as_ptr().cast::<u16>().write(var_idx as u16)
                },
                EnumTagType::U32 => unsafe {
                    self.ptr.as_ptr().cast::<u32>().write(var_idx as u32)
                },
            }

            if var_schema.kind.as_struct().is_none() {
                return Err(D::Error::custom(
                    "All enum variant types must have a struct Schema",
                ));
            }

            unsafe {
                Ok(SchemaRefMut::from_ptr_schema(
                    self.ptr.as_ptr().add(value_offset),
                    var_schema,
                ))
            }
        }
    }
}
