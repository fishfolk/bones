use std::{
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        atomic::{AtomicU32, Ordering::SeqCst},
        Arc,
    },
};

use append_only_vec::AppendOnlyVec;
use bones_utils::{parking_lot::Mutex, prelude::*};
use dashmap::DashMap;
use event_listener::{Event, EventListener};
use semver::VersionReq;

use crate::prelude::*;

/// The unique ID for an asset pack.
///
/// Asset pack IDs are made up of a human-readable label, and a unique identifier. For example:
///
/// > awesome-pack_01h502c309fddv1vq1gwa918e8
///
/// These IDs can be generated with the [TypeID gen][gen] utility.
///
/// [gen]: https://zicklag.github.io/type-id-gen/
pub type AssetPackId = LabeledId;

/// An asset pack contains assets that are loaded by the game.
///
/// The game's built-in assets are contained the the core asset pack, and mods or other assets may
/// also be loaded.
#[derive(Clone, Debug)]
pub struct AssetPack {
    /// The display name of the asset pack.
    pub name: String,
    /// The unique ID of the asset pack.
    pub id: AssetPackId,
    /// The version number of the asset pack.
    pub version: Version,

    /// The game [`VersionReq`] this asset pack is compatible with.
    pub game_version: VersionReq,

    /// Schemas provided in the asset pack.
    pub schemas: Vec<&'static Schema>,
    /// The root asset for the asset pack.
    pub root: UntypedHandle,
}

/// Specifies an asset pack, and it's exact version.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetPackSpec {
    /// The ID of the asset pack.
    pub id: AssetPackId,
    /// The version of the asset pack.
    pub version: Version,
}

impl std::fmt::Display for AssetPackSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { id, version } = self;
        write!(f, "{id}@{version}")
    }
}

/// A requirement specifier for an asset pack, made up of the asset pack's [`LabeledId`] and it's
/// [`VersionReq`].
#[derive(Debug, Clone)]
pub struct AssetPackReq {
    /// The asset pack ID.
    pub id: LabeledId,
    /// The version of the asset pack.
    pub version: VersionReq,
}

/// A schema reference, containing the ID of the pack that defined the schema, and the name of the
/// schema in the pack.
#[derive(Clone, Debug)]
pub struct SchemaPath {
    /// The ID of the pack, or [`None`] if it refers to the core pack.
    pub pack: Option<AssetPackReq>,
    /// The name of the schema.
    pub name: String,
}

/// Struct containing all the game's loaded assets, including the default assets and
/// asset-packs/mods.
pub struct LoadedAssets {
    /// The game's default asset pack.
    pub default: UntypedHandle,
    /// Extra asset packs. The key is the the name of the asset pack.
    pub packs: HashMap<String, UntypedHandle>,
}

/// The progress that has been made loading the game assets.
#[derive(Debug, Clone, Default)]
pub struct AssetLoadProgress {
    assets_to_load: Arc<AtomicU32>,
    assets_downloaded: Arc<AtomicU32>,
    assets_loaded: Arc<AtomicU32>,
    assets_errored: Arc<AtomicU32>,
    /// The event notifier that is used to wake interested tasks that are waiting for asset load
    /// to progress.
    event: Arc<Event>,
}

impl AssetLoadProgress {
    /// Increment the number of assets that need to be loaded by one.
    pub fn inc_to_load(&self) {
        self.assets_to_load.fetch_add(1, SeqCst);
    }

    /// Increment the number of assets that have errored during loading.
    pub fn inc_errored(&self) {
        self.assets_errored.fetch_add(1, SeqCst);
    }

    /// Increment the number of assets that have been downloaded by one.
    pub fn inc_downloaded(&self) {
        self.assets_downloaded.fetch_add(1, SeqCst);
    }

    /// Increment the number of assets that have been loaded by one.
    pub fn inc_loaded(&self) {
        self.assets_loaded.fetch_add(1, SeqCst);
        self.event.notify(usize::MAX);
    }

    /// Get whether or not all the assets are done loading.
    ///
    /// > **Note:** Assets that have errored while loading are still counted as "done loading".
    pub fn is_finished(&self) -> bool {
        let loaded = self.assets_loaded.load(SeqCst);
        let pending = self.assets_to_load.load(SeqCst);
        let errored = self.assets_errored.load(SeqCst);
        loaded != 0 && (loaded + errored) == pending
    }

    /// Get the number of assets that have been downloaded and loaded by their asset loaders.
    pub fn loaded(&self) -> u32 {
        self.assets_loaded.load(SeqCst)
    }

    /// Get the number of assets that have errored while loading.
    pub fn errored(&self) -> u32 {
        self.assets_errored.load(SeqCst)
    }

    /// Get the number of assets that must be loaded.
    ///
    /// Since assets are discovered as they are loaded this number may not be the final
    /// asset count and may increase as more assets are discovered.
    pub fn to_load(&self) -> u32 {
        self.assets_to_load.load(SeqCst)
    }

    /// Get the number of assets that have had their data downloaded. Once an asset is downloaded
    /// we have the raw bytes, but it may not have been processed by it's asset loader.
    pub fn downloaded(&self) -> u32 {
        self.assets_downloaded.load(SeqCst)
    }

    /// Get an event listener that will be notified each time asset load progress
    /// has been updated.
    pub fn listen(&self) -> Pin<Box<EventListener>> {
        self.event.listen()
    }
}

// TODO: Think of alternative to dashmap.
// Dashmap is annoying to use because it wraps all returned assets from our API in dashmap
// its reference container type to manage the locking. We should try to come up with a
// way to manage the concurrent asset loading without requring the locks if possible.

/// Stores assets for later retrieval.
#[derive(Default, Clone, Debug)]
pub struct AssetStore {
    /// Maps the handle of the asset to it's content ID.
    pub asset_ids: DashMap<UntypedHandle, Cid>,
    /// Content addressed cache of raw bytes for asset data.
    ///
    /// Storing asset data in this ways allows you to easily replicate assets to other players over
    /// the network by comparing available [`Cid`]s.
    pub asset_data: DashMap<Cid, Vec<u8>>,
    /// Maps asset content IDs, to assets that have been loaded by an asset loader from the raw
    /// bytes.
    pub assets: DashMap<Cid, LoadedAsset>,
    /// Maps the asset [`AssetLoc`] to it's handle.
    pub path_handles: DashMap<AssetLoc, UntypedHandle>,

    /// List of assets that depend on the given assets.
    pub reverse_dependencies: DashMap<UntypedHandle, HashSet<UntypedHandle>>,
    /// Lists the packs that have not been loaded due to an incompatible game version.
    pub incompabile_packs: DashMap<String, PackfileMeta>,

    /// The core asset pack, if it's been loaded.
    pub core_pack: Arc<Mutex<Option<AssetPack>>>,
    /// The asset packs that have been loaded.
    pub packs: DashMap<AssetPackSpec, AssetPack>,
    /// Maps the directory names of asset packs to their [`AssetPackSpec`].
    pub pack_dirs: DashMap<String, AssetPackSpec>,
}

/// Contains that path to an asset, and the pack_dir that it was loaded from.
///
/// A pack of [`None`] means that it was loaded from the core pack.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AssetLoc {
    /// The path to the asset in it's pack.
    pub path: PathBuf,
    /// The pack_dir of the pack that the asset is in.
    pub pack: Option<String>,
}

impl AssetLoc {
    /// Borrow as an [`AssetLocRef`].
    pub fn as_ref(&self) -> AssetLocRef {
        AssetLocRef {
            pack: self.pack.as_deref(),
            path: &self.path,
        }
    }
}

impl From<&AssetLocRef<'_>> for AssetLoc {
    fn from(value: &AssetLocRef<'_>) -> Self {
        AssetLoc {
            path: value.path.to_owned(),
            pack: value.pack.map(|x| x.to_owned()),
        }
    }
}

/// A borrowed version of [`AssetLoc`].
#[derive(Clone, PartialEq, Eq, Hash, Debug, Copy)]
pub struct AssetLocRef<'a> {
    /// The path to the asset in it's pack.
    pub path: &'a Path,
    /// The pack_dir of the pack that the asset is in.
    pub pack: Option<&'a str>,
}

impl<'a> From<(&'a Path, Option<&'a str>)> for AssetLocRef<'a> {
    fn from((path, pack): (&'a Path, Option<&'a str>)) -> Self {
        Self { pack, path }
    }
}

impl AssetLocRef<'_> {
    /// Clone data to an owned [`AssetLoc`].
    pub fn to_owned(&self) -> AssetLoc {
        self.into()
    }
}

/// An asset that has been loaded.
#[derive(Clone, Debug, Deref, DerefMut)]
pub struct LoadedAsset {
    /// The content ID of the loaded asset.
    ///
    /// This is a hash of the contents of the asset's binary data and all of the cids of it's
    /// dependencies.
    pub cid: Cid,
    /// The asset pack this was loaded from, or [`None`] if it is from the default pack.
    pub pack_spec: Option<AssetPackSpec>,
    /// The pack and path the asset was loaded from.
    pub loc: AssetLoc,
    /// The content IDs of any assets needed by this asset as a dependency.
    pub dependencies: Vec<UntypedHandle>,
    /// The loaded data of the asset.
    #[deref]
    pub data: SchemaBox,
}

/// An identifier for an asset.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Default)]
pub struct AssetInfo {
    /// The unique ID of the asset pack this asset is located in.
    pub pack: Cid,
    /// The path to the asset, relative to the root of the asset pack.
    pub path: PathBuf,
}

/// Context provided to custom asset loaders in the [`AssetLoader::load`] method.
pub struct AssetLoadCtx {
    /// The asset server.
    pub asset_server: AssetServer,
    /// The location of the asset.
    pub loc: AssetLoc,
    /// The [`Cid`]s of the assets this asset depends on.
    ///
    /// This is automatically updated when calling [`AssetLoadCtx::load_asset`].
    pub dependencies: Arc<AppendOnlyVec<UntypedHandle>>,
}

impl AssetLoadCtx {
    /// Load another asset as a child of this asset.
    pub fn load_asset(&mut self, path: &Path) -> anyhow::Result<UntypedHandle> {
        let handle = self.asset_server.load_asset(AssetLocRef {
            path,
            pack: self.loc.as_ref().pack,
        });
        self.dependencies.push(handle);
        Ok(handle)
    }
}

/// A custom assset loader.
pub trait AssetLoader: Sync + Send + 'static {
    /// Load the asset from raw bytes.
    fn load(
        &self,
        ctx: AssetLoadCtx,
        bytes: &[u8],
    ) -> futures::future::Boxed<anyhow::Result<SchemaBox>>;
}

/// A custom asset loader implementation for a metadata asset.
///
/// This is similar in purpose to implementing [`AssetLoader`], but instead of loading from bytes,
/// it loads from the deserialized [`SchemaRefMut`] of a metadata asset and must be added as a
/// schema type data.
#[derive(HasSchema)]
#[schema(no_clone, no_default)]
pub struct SchemaMetaAssetLoader(
    pub  fn(
        ctx: &mut MetaAssetLoadCtx,
        ptr: SchemaRefMut<'_>,
        deserialzer: &mut dyn erased_serde::Deserializer,
    ) -> anyhow::Result<()>,
);

impl SchemaMetaAssetLoader {
    /// Load the asset
    pub fn load<'a, 'de, D>(
        &self,
        ctx: &mut MetaAssetLoadCtx,
        ptr: SchemaRefMut<'a>,
        deserializer: D,
    ) -> Result<(), erased_serde::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let mut de = <dyn erased_serde::Deserializer>::erase(deserializer);
        (self.0)(ctx, ptr, &mut de).map_err(|e| erased_serde::Error::custom(e.to_string()))
    }
}

/// The kind of asset a type represents.
#[derive(HasSchema)]
#[schema(opaque, no_default, no_clone)]
pub enum AssetKind {
    /// This is a metadata asset that can be loaded from JSON or YAML files.
    Metadata {
        /// The `extension` is the portion of the extension that comes before the `.json`, `.yml`,
        /// or `.yaml` extension. For example, if the `extension` was set to `weapon`, then the asset
        /// could be loaded from `.weapon.json`, `.weapon.yml`, or `.weapon.yaml` files.
        extension: String,
    },
    /// An asset with a custom asset loader.
    Custom {
        /// The loader implementation for the asset.
        loader: Box<dyn AssetLoader>,
        /// The list of file extensions to load this asset from.
        extensions: Vec<String>,
    },
}

/// Helper function to return type data for a metadata asset.
///
/// # Example
///
/// This is meant to be used in a `type_data` attribute when deriving [`HasSchema`].
///
/// ```
/// # use bones_asset::prelude::*;
/// # use glam::*;
/// #[derive(HasSchema, Default, Clone)]
/// #[type_data(metadata_asset("atlas"))]
/// #[repr(C)]
/// struct AtlasMeta {
///     pub tile_size: Vec2,
///     pub grid_size: UVec2,
/// }
/// ```
pub fn metadata_asset(extension: &str) -> AssetKind {
    AssetKind::Metadata {
        extension: extension.into(),
    }
}

/// Helper function to return type data for a custom asset loader.
///
/// # Example
///
/// This is meant to be used in a `type_data` attribute when deriving [`HasSchema`].
///
/// ```
/// # use bones_asset::prelude::*;
/// # use bones_utils::prelude::*;
/// #[derive(HasSchema, Default, Clone)]
/// #[type_data(asset_loader("png", PngLoader))]
/// #[repr(C)]
/// struct Image {
///     data: SVec<u8>,
///     width: u32,
///     height: u32,
/// }
///
/// struct PngLoader;
/// impl AssetLoader for PngLoader {
///     fn load(&self, ctx: AssetLoadCtx, data: &[u8]) -> futures::future::Boxed<anyhow::Result<SchemaBox>> {
///         Box::pin(async move {
///             todo!("Load PNG from data");
///         })
///     }
/// }
/// ```
pub fn asset_loader<L: AssetLoader, E: Into<AssetExtensions>>(
    extensions: E,
    loader: L,
) -> AssetKind {
    AssetKind::Custom {
        loader: Box::new(loader),
        extensions: extensions.into().0,
    }
}

/// Helper type for storing asset extensions.
pub struct AssetExtensions(Vec<String>);
impl<'a, const N: usize> From<[&'a str; N]> for AssetExtensions {
    fn from(value: [&'a str; N]) -> Self {
        Self(value.iter().map(|x| x.to_string()).collect())
    }
}
impl<'a> From<&'a str> for AssetExtensions {
    fn from(value: &'a str) -> Self {
        Self(vec![value.to_string()])
    }
}
