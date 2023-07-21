//! An asset interface for Bones.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    str::FromStr,
};

use bones_ecs::prelude::{Deref, DerefMut};
use bones_reflect::prelude::*;
use bones_utils::prelude::*;
use semver::{Version, VersionReq};
use serde::Deserialize;

/// The prelude.
pub mod prelude {
    pub use crate::*;
    pub use bones_reflect::prelude::*;
    pub use semver::Version;
}

mod cid;
pub use cid::*;
mod server;
pub use server::*;
mod io;
pub use io::*;
mod path;
pub use path::*;
mod handle;
pub use handle::*;

mod parse;

/// The unique ID for an asset pack.
///
/// Asset pack IDs are made up of a human-readable label, and a unique identifier. For example:
///
///     awesome-pack_01h502c309fddv1vq1gwa918e8
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
    pub import_schemas: HashMap<String, SchemaId>,
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

/// A schema identifier, containing the ID of the pack that defined the schema, and the name of the
/// schema in the pack.
#[derive(Clone, Debug)]
pub struct SchemaId {
    /// The ID of the pack, or [`None`] if it refers to the core pack.
    pub pack: Option<AssetPackReq>,
    /// The name of the schema.
    pub name: String,
}

/// Struct responsible for loading assets into it's contained [`AssetStore`], using an [`AssetIo`]
/// implementation.
pub struct AssetServer {
    /// The version of the game. This is used to evaluate whether asset packs are compatible with
    /// the game.
    pub game_version: Version,
    /// The [`AssetIo`] implementation used to load assets.
    pub io: Box<dyn AssetIo>,
    /// The asset store.
    pub store: AssetStore,
    /// Mapping of core schemas.
    ///
    /// The string key may be used in asset file extensions like `some_name.key.yaml` or
    /// `some_name.key.json`.
    pub core_schemas: HashMap<String, Cow<'static, Schema>>,
    /// Lists the packs that have not been loaded due to an incompatible game version.
    pub incompabile_packs: HashMap<String, PackfileMeta>,
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

    /// The core asset pack, if it's been loaded.
    pub core_pack: Option<AssetPack>,
    /// The asset packs that have been loaded.
    pub packs: HashMap<AssetPackSpec, AssetPack>,
    /// Maps the directory names of asset packs to their [`AssetPackSpec`].
    pub pack_dirs: HashMap<String, AssetPackSpec>,
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
    pub pack: Option<AssetPackSpec>,
    /// The name of the directory this pack was loaded from, unless it is from the default pack.
    pub pack_dir: Option<String>,
    /// The path in the asset pack that this asset is from.
    pub path: PathBuf,
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
