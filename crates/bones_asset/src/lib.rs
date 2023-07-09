//! An asset interface for Bones.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use std::{
    any::Any,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use bones_ecs::ulid::{TypeUlid, UlidMap};
use bones_reflect::schema::Schema;
use bones_utils::prelude::*;
use type_ulid::{TypeUlidDynamic, Ulid};

/// The prelude.
pub mod prelude {
    pub use crate::{cid::*, *};
}

mod cid;
pub use cid::*;

/// An asset pack contains assets that are loaded by the game.
///
/// The game's built-in assets are contained the the core asset pack, and mods or other assets may
/// also be loaded.
#[derive(Clone, Debug)]
pub struct AssetPack {
    /// The display name of the asset pack.
    pub name: String,
    /// The unique ID of the asset pack.
    pub id: LabeledId,
    /// Schemas provided in the asset pack.
    pub schemas: HashMap<String, Schema>,
    /// The root asset for the asset pack.
    pub root: UntypedHandle,
}

/// A schema identifier, containing the ID of the pack that defined the schema, and the name of the
/// schema in the pack.
#[derive(Clone, Debug)]
pub struct SchemaId {
    /// The ID of the pack, or [`None`] if it refers to the core pack.
    pub pack: Option<LabeledId>,
    /// The name of the schema.
    pub name: String,
}

/// Struct responsible for loading assets.
#[derive(Default)]
pub struct AssetServer {
    // /// The asset store.
    // pub store: AssetStore,
    // /// List of asset loaders.
    // pub loaders: Vec<Box<dyn AssetLoader>>,
}

/// [`AssetIo`] is a trait that is implemented for backends capable of loading all the games assets
/// and returning a [`LoadedAssets`].
pub trait AssetIo {
    /// Load the game assets.
    ///
    /// TODO: Add a load progress and/or make this function async.
    fn load_all(&self, server: &mut AssetServer) -> anyhow::Result<LoadedAssets>;

    /// Load an asset with the given path, from the given pack, or the default pack if not
    /// specified.
    fn load_asset(
        &self,
        server: &mut AssetServer,
        path: &Path,
        pack: Option<&str>,
    ) -> anyhow::Result<UntypedHandle>;

    /// Get the binary contents of the asset from the given path and pack.
    fn get_contents(&self, path: &Path, pack: Option<&str>) -> anyhow::Result<Vec<u8>>;
}

/// [`AssetIo`] implementation that loads from the filesystem.
#[cfg(not(target_arch = "wasm32"))]
pub struct FileAssetIo {
    /// The directory to load the default asset from.
    pub default_dir: PathBuf,
    /// The directory to load the asset packs from.
    pub packs_dir: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl AssetIo for FileAssetIo {
    fn load_all(&self, server: &mut AssetServer) -> anyhow::Result<LoadedAssets> {
        // Load the default asset pack
        let pack_file_name = 'file: {
            for filename in ["pack.yaml", "pack.yml", "pack.json"] {
                let path = self.default_dir.join(filename);
                if path.exists() {
                    break 'file PathBuf::from(filename);
                }
            }

            anyhow::bail!(
                "Pack file does not exist in default asset dir: {:?}",
                self.default_dir
            );
        };
        let default = self.load_asset(server, &pack_file_name, None)?;

        // Load the asset packs
        let packs = std::fs::read_dir(&self.packs_dir)?
            .map(|x| x.map(|dir| dir.file_name().to_str().unwrap().to_owned()))
            .map(|pack| {
                let pack = pack?;
                let pack_file_name = 'file: {
                    for filename in ["pack.yaml", "pack.yml", "pack.json"] {
                        let path = self.packs_dir.join(&pack).join(filename);
                        if path.exists() {
                            break 'file PathBuf::from(filename);
                        }
                    }

                    anyhow::bail!(
                        "Pack file does not exist in default asset dir: {:?}",
                        self.default_dir
                    );
                };

                let asset = self.load_asset(server, &pack_file_name, Some(&pack))?;
                Ok((pack, asset))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(LoadedAssets { default, packs })
    }

    fn load_asset(
        &self,
        server: &mut AssetServer,
        path: &Path,
        pack: Option<&str>,
    ) -> anyhow::Result<UntypedHandle> {
        let asset_file_contents = self.get_contents(path, pack)?;
        server.load_asset(self, &asset_file_contents, path, pack)
    }

    fn get_contents(&self, path: &Path, pack: Option<&str>) -> anyhow::Result<Vec<u8>> {
        let base_dir = pack
            .map(|x| self.packs_dir.join(x))
            .unwrap_or_else(|| self.default_dir.clone());
        let filepath = base_dir.join(path);
        std::fs::read(&filepath).context(format!(
            "Cannot read pack file for default asset: {filepath:?}"
        ))
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

/// Trait that must be implemented by asset types.
pub trait Asset: Any + TypeUlidDynamic {}
impl<T: Any + TypeUlidDynamic> Asset for T {}

/// Trait for asset loaders.
pub trait AssetLoader {
    /// Load an asset.
    fn load(&self, bytes: &[u8]) -> Box<dyn Asset>;

    /// Return the extensions to register the asset loader with.
    fn extensions(&self) -> &'static [&'static str];
}

impl AssetServer {
    /// Initialize a new [`AssetServer`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Load an asset
    pub fn load_asset<Io: AssetIo>(
        &mut self,
        asset_io: &Io,
        content: &[u8],
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
