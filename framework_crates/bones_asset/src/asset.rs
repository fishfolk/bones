use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use bones_utils::prelude::*;
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
    pub schemas: HashMap<String, Schema>,
    /// Specify schemas to import from other asset packs.
    pub import_schemas: HashMap<String, SchemaPath>,
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

/// Struct responsible for loading assets into it's contained [`AssetStore`], using an [`AssetIo`]
/// implementation.
#[derive(HasSchema)]
#[schema(opaque, no_clone)]
pub struct AssetServer {
    /// The version of the game. This is used to evaluate whether asset packs are compatible with
    /// the game.
    pub game_version: Version,
    /// The [`AssetIo`] implementation used to load assets.
    pub io: Box<dyn AssetIo>,
    /// The asset store.
    pub store: AssetStore,
    /// List of registered asset types.
    pub asset_types: Vec<&'static Schema>,
    /// Lists the packs that have not been loaded due to an incompatible game version.
    pub incompabile_packs: HashMap<String, PackfileMeta>,
    /// Channel fro the [`AssetIo`] implementation that is used to detect asset changes.
    pub asset_changes: OnceLock<Option<async_channel::Receiver<AssetLoc>>>,
}

impl Default for AssetServer {
    fn default() -> Self {
        Self {
            game_version: Version::new(0, 0, 0),
            io: Box::new(DummyIo::new([])),
            store: default(),
            asset_types: default(),
            incompabile_packs: default(),
            asset_changes: default(),
        }
    }
}

/// Struct containing all the game's loaded assets, including the default assets and
/// asset-packs/mods.
pub struct LoadedAssets {
    /// The game's default asset pack.
    pub default: UntypedHandle,
    /// Extra asset packs. The key is the the name of the asset pack.
    pub packs: HashMap<String, UntypedHandle>,
}

/// Stores assets for later retrieval.
#[derive(Default, Clone, Debug)]
pub struct AssetStore {
    /// Maps the handle of the asset to it's content ID.
    pub asset_ids: HashMap<UntypedHandle, Cid>,
    /// Maps asset content IDs, to loaded assets.
    pub assets: HashMap<Cid, LoadedAsset>,
    /// Maps the asset [`AssetLoc`] to it's handle.
    pub path_handles: HashMap<AssetLoc, UntypedHandle>,
    /// List of assets that depend on the given assets.
    pub reverse_dependencies: HashMap<Cid, HashSet<Cid>>,

    /// The core asset pack, if it's been loaded.
    pub core_pack: Option<AssetPack>,
    /// The asset packs that have been loaded.
    pub packs: HashMap<AssetPackSpec, AssetPack>,
    /// Maps the directory names of asset packs to their [`AssetPackSpec`].
    pub pack_dirs: HashMap<String, AssetPackSpec>,
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
    pub dependencies: Vec<Cid>,
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
pub struct AssetLoadCtx<'a> {
    /// The asset server.
    pub asset_server: &'a mut AssetServer,
    /// The location of the asset.
    pub loc: AssetLocRef<'a>,
    /// The [`Cid`]s of the assets this asset depends on.
    ///
    /// This is automatically updated when calling [`AssetLoadCtx::load_asset`].
    pub dependencies: &'a mut Vec<Cid>,
}

impl AssetLoadCtx<'_> {
    /// Load another asset as a child of this asset.
    pub fn load_asset(&mut self, path: &Path) -> anyhow::Result<UntypedHandle> {
        let handle = self.asset_server.load_asset(AssetLocRef {
            path,
            pack: self.loc.pack,
        })?;
        let cid = self.asset_server.store.asset_ids.get(&handle).unwrap();
        self.dependencies.push(*cid);
        Ok(handle)
    }
}

/// A custom assset loader.
pub trait AssetLoader: Sync + Send + 'static {
    /// Load the asset from raw bytes.
    fn load(&self, ctx: AssetLoadCtx, bytes: &[u8]) -> anyhow::Result<SchemaBox>;
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
        ptr: SchemaRefMut<'_, '_>,
        deserialzer: &mut dyn erased_serde::Deserializer,
    ) -> anyhow::Result<()>,
);

impl SchemaMetaAssetLoader {
    /// Load the asset
    pub fn load<'a, 'de, D>(
        &self,
        ctx: &mut MetaAssetLoadCtx,
        ptr: SchemaRefMut<'a, 'a>,
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
///     fn load(&self, ctx: AssetLoadCtx, data: &[u8]) -> anyhow::Result<SchemaBox> {
///         todo!("Load PNG from data");
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
