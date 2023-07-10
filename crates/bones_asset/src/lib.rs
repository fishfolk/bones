//! An asset interface for Bones.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use bones_ecs::ulid::{TypeUlid, UlidMap};
use bones_reflect::schema::Schema;
use bones_utils::prelude::*;
use semver::{Version, VersionReq};
use serde::Deserialize;
use ulid::Ulid;

/// The prelude.
pub mod prelude {
    pub use crate::*;
}

mod cid;
pub use cid::*;
mod load;
pub use load::*;
mod io;
pub use io::*;

mod parse;

/// The unique ID for an asset pack.
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
#[derive(Clone, Debug)]
pub struct AssetPackSpecifier {
    /// The ID of the asset pack.
    pub id: AssetPackId,
    /// The version of the asset pack.
    pub version: Version,
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

/// Struct responsible for loading assets.
pub struct AssetServer {
    /// The [`AssetIo`] implementation used to load assets.
    pub io: Box<dyn AssetIo>,
    /// The asset store.
    pub store: AssetStore,
}

/// Struct containing all the game's loaded assets, including the default assets and
/// asset-packs/mods.
pub struct LoadedAssets {
    /// The game's default asset pack.
    pub default: UntypedHandle,
    /// Extra asset packs. The key is the the name of the asset pack.
    pub packs: HashMap<String, UntypedHandle>,
}

impl AssetServer {
    /// Initialize a new [`AssetServer`].
    pub fn new<Io: AssetIo + 'static>(io: Io) -> Self {
        Self {
            io: Box::new(io),
            store: default(),
        }
    }

    /// Load an asset
    pub fn load_asset<Io: AssetIo>(
        &mut self,
        path: &Path,
        pack: Option<&str>,
    ) -> anyhow::Result<UntypedHandle> {
        // use sha2::Digest;

        // let rid = Ulid::new();
        // let ctx = AssetLoadCtxCell::new(self, asset_io, pack, path);
        // let ends_with =
        //     |path: &Path, ext: &str| path.extension().unwrap().to_string_lossy().ends_with(ext);
        // let is_metadata =
        //     ends_with(path, "json") || ends_with(path, "yaml") || ends_with(path, "yml");

        // let meta = if is_metadata {
        //     if path.ends_with("json") {
        //         ctx.deserialize(&mut serde_json::Deserializer::from_slice(content))?
        //     } else {
        //         ctx.deserialize(serde_yaml::Deserializer::from_slice(content))?
        //     }
        // } else {
        //     todo!();
        // };
        // let mut ctx = ctx.into_inner();
        // let dependencies = std::mem::take(&mut ctx.dependencies);

        // // Calculate the content ID by hashing the contents of the file with the content IDs of the
        // // dependencies.
        // let mut sha = sha2::Sha256::new();
        // sha.update(content);
        // for dep in &dependencies {
        //     sha.update(dep.0);
        // }
        // let bytes = sha.finalize();
        // let mut cid = Cid::default();
        // cid.0.copy_from_slice(&bytes);

        // let loaded_asset = LoadedAsset {
        //     cid,
        //     dependencies,
        //     asset_kind: Metadata::ULID,
        //     data: AssetData::Metadata(meta),
        //     pack: pack.map(ToOwned::to_owned),
        //     path: path.to_owned(),
        // };

        // self.store.asset_ids.insert(rid, loaded_asset.cid);
        // self.store.assets.insert(loaded_asset.cid, loaded_asset);

        // Ok(UntypedHandle { rid })
        todo!();
    }

    /// Borrow a [`LoadedAsset`] associated to the given handle.
    pub fn get_untyped(&self, handle: &UntypedHandle) -> Option<&LoadedAsset> {
        // let cid = self.store.asset_ids.get(&handle.rid)?;
        // self.store.assets.get(cid)
        todo!();
    }
}

/// Stores assets for later retrieval.
#[derive(Default, Clone, Debug)]
pub struct AssetStore {
    /// Maps the runtime ID of the asset to it's content ID.
    pub asset_ids: UlidMap<Cid>,
    /// Maps asset content IDs, to loaded assets.
    pub assets: HashMap<Cid, LoadedAsset>,
}

/// An asset that has been loaded.
#[derive(Clone, Debug)]
pub struct LoadedAsset {
    /// The content ID of the loaded asset.
    ///
    /// This is a hash of the contents of the asset's binary data and all of the cids of it's
    /// dependencies.
    pub cid: Cid,
    /// The asset pack this was loaded from, or [`None`] if it is from the default pack.
    pub pack: Option<String>,
    /// The path in the asset pack that this asset is from.
    pub path: PathBuf,
    /// The content IDs of any assets needed by this asset as a dependency.
    pub dependencies: Vec<Cid>,
    /// Unique identifier for the asset kind. For Rust structs this will match the [`TypeUlid`].
    pub asset_kind: Ulid,
    /// The loaded data of the asset. This is in the format produced by the asset loader, not the
    /// binary of the asset source.
    pub data: AssetData,
}

/// The raw data stored for a loaded asset.
#[derive(Debug, Clone)]
pub enum AssetData {
    // /// Asset has been loaded and stored at the given pointer.
    // Raw(*mut u8),
    // /// The asset has been loaded from JSON/YAML data.
    // Metadata(Metadata),
}

/// An identifier for an asset.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Default)]
pub struct AssetInfo {
    /// The unique ID of the asset pack this asset is located in.
    pub pack: Cid,
    /// The path to the asset, relative to the root of the asset pack.
    pub path: AssetPath,
}

impl AssetInfo {
    /// Create a new asset ID.
    pub fn new<P: Into<PathBuf>>(pack: Cid, path: P, label: Option<String>) -> Self {
        Self {
            pack,
            path: AssetPath::new(path, label),
        }
    }
}

/// A path to an asset.
///
/// This is a virtual filesystem path, and may not actually refer to physical files.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPath {
    /// The virtual filesystem path
    pub path: Arc<Path>,
    /// The optional sub-asset label
    pub label: Option<Arc<str>>,
}

impl AssetPath {
    /// Create a new asset path.
    pub fn new<P: Into<PathBuf>>(path: P, label: Option<String>) -> Self {
        AssetPath {
            path: Arc::from(path.into()),
            label: label.map(Arc::from),
        }
    }

    /// Take this path, treat it as a path relative to `base_path`, normalize it, and update `self`
    /// with the result.
    pub fn normalize_relative_to(&mut self, base_path: &Path) {
        fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
            let mut components = path.components().peekable();
            let mut ret = if let Some(c @ std::path::Component::Prefix(..)) = components.peek() {
                let buf = std::path::PathBuf::from(c.as_os_str());
                components.next();
                buf
            } else {
                std::path::PathBuf::new()
            };

            for component in components {
                match component {
                    std::path::Component::Prefix(..) => unreachable!(),
                    std::path::Component::RootDir => {
                        ret.push(component.as_os_str());
                    }
                    std::path::Component::CurDir => {}
                    std::path::Component::ParentDir => {
                        ret.pop();
                    }
                    std::path::Component::Normal(c) => {
                        ret.push(c);
                    }
                }
            }

            ret
        }

        let is_relative = !self.path.starts_with(Path::new("/"));

        let path = if is_relative {
            let base = base_path.parent().unwrap_or_else(|| Path::new(""));
            base.join(&self.path)
        } else {
            self.path.to_path_buf()
        };

        self.path = Arc::from(normalize_path(&path));
    }
}

impl Default for AssetPath {
    fn default() -> Self {
        Self {
            path: Arc::from(PathBuf::default()),
            label: Default::default(),
        }
    }
}

/// A typed handle to an asset.
///
/// The type of the handle is used to help reduce runtime errors arising from mis-matching handle
/// types, but internally, the handle's only stored data is it's [`AssetPath`].
///
/// You can change the type of a handle by converting it to an untyped handle with
/// [`untyped()`][Self::untyped] and converting it back to a typed handle with
/// [`typed()`][UntypedHandle::typed].
#[derive(PartialEq, Eq, Hash, Default)]
pub struct Handle<T: TypeUlid> {
    /// The runtime ID of the asset.
    pub id: Ulid,
    phantom: PhantomData<T>,
}

impl<T: TypeUlid> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            phantom: self.phantom,
        }
    }
}

impl<T: TypeUlid> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle").field("id", &self.id).finish()
    }
}

impl<T: TypeUlid> Handle<T> {
    /// Convert the handle to an [`UntypedHandle`].
    pub fn untyped(self) -> UntypedHandle {
        UntypedHandle { rid: self.id }
    }
}

/// An untyped handle to an asset.
///
/// This simply contains the [`AssetPath`] of the asset.
///
/// Can be converted to a typed handle with the [`typed()`][Self::typed] method.
#[derive(Default, Clone, Debug, Hash, PartialEq, Eq, Copy)]
pub struct UntypedHandle {
    /// The runtime ID of the handle
    pub rid: Ulid,
}

impl UntypedHandle {
    /// Create a typed [`Handle<T>`] from this [`UntypedHandle`].
    pub fn typed<T: TypeUlid>(self) -> Handle<T> {
        Handle {
            id: self.rid,
            phantom: PhantomData,
        }
    }
}
