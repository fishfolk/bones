use std::path::{Path, PathBuf};

use anyhow::Context;
use once_cell::sync::Lazy;
use semver::VersionReq;
use serde::{de::DeserializeSeed, Deserialize};
use ulid::Ulid;

use crate::prelude::*;
use bones_utils::*;

/// YAML format for the core asset pack's `pack.yaml` file.
#[derive(Debug, Clone, Deserialize)]
pub struct CorePackfileMeta {
    /// The path to the root asset for the pack.
    pub root: PathBuf,
}

/// YAML format for asset packs' `pack.yaml` file.
#[derive(Debug, Clone, Deserialize)]
pub struct PackfileMeta {
    /// The path to the root asset for the pack.
    pub root: PathBuf,
    /// The unique ID of the asset pack.
    pub id: AssetPackId,
    /// The version of the asset pack.
    pub version: Version,
    /// The required game version to be compatible with this asset pack.
    pub game_version: VersionReq,
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
            You may need to register the asset with asset_server.register_asset::<AssetType>()",
            self.name
        )
    }
}
impl std::error::Error for LoaderNotFound {}

impl AssetServer {
    /// Initialize a new [`AssetServer`].
    pub fn new<Io: AssetIo + 'static>(io: Io, version: Version) -> Self {
        Self {
            io: Box::new(io),
            game_version: version,
            store: default(),
            asset_types: default(),
            incompabile_packs: default(),
            asset_changes: default(),
        }
    }

    /// Register an asset type.
    pub fn register_asset<T: HasSchema>(&mut self) -> &mut Self {
        if T::schema().type_data.get::<AssetKind>().is_none() {
            panic!(
                "Type `{}` must have AssetType type data",
                std::any::type_name::<T>()
            );
        }
        self.asset_types.push(T::schema());

        self
    }

    /// Set the [`AssetIo`] implementation.
    ///
    /// This should almost always be called before calling [`load_assets()`][Self::load_assets].
    pub fn set_io<Io: AssetIo + 'static>(&mut self, io: Io) {
        self.io = Box::new(io);
    }

    /// Load the assets.
    ///
    /// All of the assets are immediately loaded synchronously, blocking until load is complete.
    pub fn load_assets(&mut self) -> anyhow::Result<()> {
        let core_pack = self.load_pack(None)?;
        let mut packs = HashMap::default();

        // For every asset pack
        for pack_dir in self.io.enumerate_packs()? {
            // Load the asset pack
            let pack_result = self.load_pack(Some(&pack_dir));
            match pack_result {
                // If the load was successful
                Ok(pack) => {
                    // Add it to our pack list.
                    let spec = AssetPackSpec {
                        id: pack.id,
                        version: pack.version.clone(),
                    };
                    packs.insert(spec, pack);
                }
                // If there was an error.
                Err(e) => match e.downcast::<IncompatibleGameVersionError>() {
                    // Check for a compatibility error
                    Ok(e) => {
                        // Add it to the list of incompatible packs.
                        self.incompabile_packs.insert(e.pack_dir, e.pack_meta);
                    }
                    // If this is another kind of error, return the error
                    Err(e) => {
                        return Err(e).context(format!("Error loading asset pack: {pack_dir}"))
                    }
                },
            }
        }

        self.store.packs = packs;
        self.store.core_pack = Some(core_pack);

        Ok(())
    }

    /// Load the asset pack with the given folder name, or else the default pack if [`None`].
    pub fn load_pack(&mut self, pack: Option<&str>) -> anyhow::Result<AssetPack> {
        // Load the core pack differently
        if pack.is_none() {
            return self
                .load_core_pack()
                .context("Error loading core asset pack");
        }

        // Load the asset packfile
        let packfile_contents = self.io.load_file((Path::new("pack.yaml"), pack).into())?;
        let meta: PackfileMeta = serde_yaml::from_slice(&packfile_contents)?;

        // If the game version doesn't match, then don't continue loading this pack.
        if !meta.game_version.matches(&self.game_version) {
            return Err(IncompatibleGameVersionError {
                game_version: self.game_version.clone(),
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

        if !path_is_metadata(&meta.root) {
            anyhow::bail!(
                "Root asset must be a JSON or YAML file with a name in the form: \
                [filename].[asset_kind].[yaml|json]"
            );
        }

        // Load the asset and produce a handle
        let root_loc = AssetLocRef {
            path: &meta.root,
            pack,
        };
        let root_handle = self.load_asset(root_loc).map_err(|e| {
            e.context(format!(
                "Error loading asset from pack `{}`: {:?}",
                pack.unwrap(),
                meta.root
            ))
        })?;

        // Return the loaded asset pack.
        Ok(AssetPack {
            name: "Core".into(),
            id: meta.id,
            version: meta.version,
            game_version: meta.game_version,
            // TODO: load & import schemas that are defined in asset packs.
            schemas: default(),
            import_schemas: default(),
            root: root_handle,
        })
    }

    /// Responds to any asset changes reported by the [`AssetIo`] implementation.
    ///
    /// This must be called or asset changes will be ignored. Additionally, the [`AssetIo`]
    /// implementation must be able to detect asset changes or this will do nothing.
    pub fn handle_asset_changes<F: FnMut(&mut AssetServer, UntypedHandle)>(
        &mut self,
        mut handle_change: F,
    ) {
        if let Some(receiver) = self.asset_changes.get_or_init(|| self.io.watch()).clone() {
            while let Ok(loc) = receiver.try_recv() {
                match self.load_asset_forced(loc.as_ref()) {
                    Ok(handle) => handle_change(self, handle),
                    Err(_) => {
                        // TODO: Handle/log asset error.
                        continue;
                    }
                };
            }
        }
    }

    /// Load the core asset pack.
    pub fn load_core_pack(&mut self) -> anyhow::Result<AssetPack> {
        // Load the core asset packfile
        let packfile_contents = self.io.load_file(AssetLocRef {
            path: Path::new("pack.yaml"),
            pack: None,
        })?;
        let meta: CorePackfileMeta = serde_yaml::from_slice(&packfile_contents)?;

        if !path_is_metadata(&meta.root) {
            anyhow::bail!(
                "Root asset must be a JSON or YAML file with a name in the form: \
                [filename].[asset_kind].[yaml|json]"
            );
        }

        // Load the asset and produce a handle
        let root_loc = AssetLocRef {
            path: &meta.root,
            pack: None,
        };
        let handle = self
            .load_asset(root_loc)
            .map_err(|e| e.context(format!("Error loading core asset: {:?}", meta.root)))?;

        // Return the loaded asset pack.
        Ok(AssetPack {
            name: "Core".into(),
            id: *CORE_PACK_ID,
            version: self.game_version.clone(),
            game_version: VersionReq {
                comparators: [semver::Comparator {
                    op: semver::Op::Exact,
                    major: self.game_version.major,
                    minor: Some(self.game_version.minor),
                    patch: Some(self.game_version.patch),
                    pre: self.game_version.pre.clone(),
                }]
                .to_vec(),
            },
            schemas: default(),
            import_schemas: default(),
            root: handle,
        })
    }

    /// Load an asset.
    pub fn load_asset(&mut self, loc: AssetLocRef) -> anyhow::Result<UntypedHandle> {
        self.impl_load_asset(loc, false)
    }

    /// Like [`load_asset()`][Self::load_asset] but forces the asset to reload, even it if has
    /// already been loaded.
    pub fn load_asset_forced(&mut self, loc: AssetLocRef) -> anyhow::Result<UntypedHandle> {
        self.impl_load_asset(loc, true)
    }

    fn impl_load_asset(&mut self, loc: AssetLocRef, force: bool) -> anyhow::Result<UntypedHandle> {
        let contents = self.io.load_file(loc).context(format!(
            "Could not load asset file: {:?} from path {:?}",
            loc.path,
            loc.pack.unwrap_or("[core]")
        ))?;

        let loc = AssetLoc {
            path: normalize_path(loc.path),
            pack: loc.pack.map(|x| x.to_owned()),
        };

        if !force {
            if let Some(handle) = self.store.path_handles.get(&loc) {
                return Ok(*handle);
            }
        }

        let loc = loc.as_ref();

        // Try to load a metadata asset if it has a YAML/JSON extension, if that doesn't work, and
        // it has a schema not found error, try to load a data asset for the same path, if that
        // doesn't work and it is an extension not found error, return the metadata error message.
        let partial = if path_is_metadata(loc.path) {
            match self.load_metadata_asset(loc, &contents) {
                Err(meta_err) => {
                    if meta_err.downcast_ref::<LoaderNotFound>().is_some() {
                        match self.load_data_asset(loc, &contents) {
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
            self.load_data_asset(loc, &contents)
        }?;

        let loaded_asset = LoadedAsset {
            cid: partial.cid,
            pack_spec: loc.pack.map(|x| {
                self.store
                    .pack_dirs
                    .get(x)
                    .expect("Pack dir not loaded properly")
                    .clone()
            }),
            loc: loc.to_owned(),
            dependencies: partial.dependencies,
            data: partial.data,
        };

        let mut reverse_deps = SmallVec::<[_; 16]>::new();
        let handle = *self
            .store
            .path_handles
            .entry(loc.to_owned())
            // If we've already loaded this asset before
            .and_modify(|handle| {
                // Remove the old asset data
                let cid = self.store.asset_ids.remove(handle).unwrap();
                let previous_asset = self.store.assets.remove(&cid).unwrap();

                // Remove the previous asset's reverse dependencies
                for dep in previous_asset.dependencies {
                    self.store
                        .reverse_dependencies
                        .get_mut(&dep)
                        .unwrap()
                        .remove(&cid);
                }

                // Reload the assets that depended on the previous asset.
                if let Some(rev_deps) = self.store.reverse_dependencies.get(&cid) {
                    for dep in rev_deps {
                        reverse_deps.push(self.store.assets.get(dep).unwrap().loc.clone());
                    }
                }
            })
            // Otherwise, create a new handle
            .or_insert(UntypedHandle { rid: Ulid::new() });

        // Update reverse dependencies
        for dep in &loaded_asset.dependencies {
            self.store
                .reverse_dependencies
                .entry(*dep)
                .or_default()
                .insert(partial.cid);
        }

        self.store.asset_ids.insert(handle, partial.cid);
        self.store.assets.insert(partial.cid, loaded_asset);

        // Reload any assets that depended on this asset before it was reloaded
        for loc in reverse_deps {
            self.load_asset_forced(loc.as_ref())?;
        }

        Ok(handle)
    }

    fn load_metadata_asset(
        &mut self,
        loc: AssetLocRef,
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
        let schema = *self
            .asset_types
            .iter()
            .find(|schema| {
                let asset_kind = schema.type_data.get::<AssetKind>().unwrap();
                match asset_kind {
                    AssetKind::Metadata { extension } => extension == schema_name,
                    _ => false,
                }
            })
            .ok_or_else(|| LoaderNotFound {
                name: schema_name.into(),
            })?;
        let mut dependencies = Vec::new();

        let mut cid = Cid::default();
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

        // Update the Cid
        dependencies.sort();
        for dep in &dependencies {
            cid.update(&dep.0);
        }

        Ok(PartialAsset {
            cid,
            dependencies,
            data,
        })
    }

    fn load_data_asset(
        &mut self,
        loc: AssetLocRef,
        contents: &[u8],
    ) -> anyhow::Result<PartialAsset> {
        // Get the schema for the asset
        let filename = loc
            .path
            .file_name()
            .ok_or_else(|| anyhow::format_err!("Invalid asset filename"))?
            .to_str()
            .ok_or_else(|| anyhow::format_err!("Invalid unicode in filename"))?;
        let (_name, extension) = filename.split_once('.').unwrap();
        let loader = self
            .asset_types
            .iter()
            .find_map(|schema| {
                let asset_kind = schema.type_data.get::<AssetKind>().unwrap();
                match &asset_kind {
                    AssetKind::Custom { extensions, loader } => {
                        if extensions
                            .iter()
                            .any(|ext| ext == extension || ext == filename)
                        {
                            Some(loader)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .ok_or_else(|| LoaderNotFound {
                name: extension.into(),
            })?;

        let mut dependencies = Vec::new();
        let mut cid = Cid::default();
        cid.update(contents);

        let ctx = AssetLoadCtx {
            asset_server: self,
            loc,
            dependencies: &mut dependencies,
        };
        let sbox = loader.load(ctx, contents)?;

        Ok(PartialAsset {
            cid,
            data: sbox,
            dependencies,
        })
    }

    /// Borrow a [`LoadedAsset`] associated to the given handle.
    pub fn get_untyped(&self, handle: UntypedHandle) -> Option<&LoadedAsset> {
        let cid = self.store.asset_ids.get(&handle)?;
        self.store.assets.get(cid)
    }

    /// Borrow a [`LoadedAsset`] associated to the given handle.
    pub fn get_untyped_mut(&mut self, handle: UntypedHandle) -> Option<&mut LoadedAsset> {
        let cid = self.store.asset_ids.get(&handle)?;
        self.store.assets.get_mut(cid)
    }

    /// Read the core asset pack.
    ///
    /// # Panics
    ///
    /// Panics if the assets have not be loaded yet with [`AssetServer::load_assets`].
    #[track_caller]
    pub fn core(&self) -> &AssetPack {
        self.store.core_pack.as_ref().unwrap()
    }

    /// Get the core asset pack's root asset.
    pub fn root<T: HasSchema>(&self) -> &T {
        self.get(self.core().root.typed())
    }

    /// Read the loaded asset packs.
    pub fn packs(&self) -> &HashMap<AssetPackSpec, AssetPack> {
        &self.store.packs
    }

    /// Borrow a loaded asset.
    ///
    /// # Panics
    ///
    /// Panics if the asset is not loaded or if the asset asset with the given handle doesn't have a
    /// schema matching `T`.
    #[track_caller]
    pub fn get<T: HasSchema>(&self, handle: Handle<T>) -> &T {
        let cid = self
            .store
            .asset_ids
            .get(&handle.untyped())
            .expect(NO_ASSET_MSG);
        self.store.assets.get(cid).expect(NO_ASSET_MSG).cast_ref()
    }

    /// Mutably borrow a loaded asset.
    ///
    /// # Panics
    ///
    /// Panics if the asset is not loaded or if the asset asset with the given handle doesn't have a
    /// schema matching `T`.
    #[track_caller]
    pub fn get_mut<T: HasSchema>(&mut self, handle: &Handle<T>) -> &mut T {
        let cid = self
            .store
            .asset_ids
            .get(&handle.untyped())
            .expect(NO_ASSET_MSG);
        self.store
            .assets
            .get_mut(cid)
            .expect(NO_ASSET_MSG)
            .cast_mut()
    }
}

/// Partial of a [`LoadedAsset`] used internally while loading is in progress.
struct PartialAsset {
    pub cid: Cid,
    pub data: SchemaBox,
    pub dependencies: Vec<Cid>,
}

const NO_ASSET_MSG: &str = "Asset not loaded";
fn path_is_metadata(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    ext == "yaml" || ext == "yml" || ext == "json"
}

use metadata::*;
mod metadata {
    use serde::de::{DeserializeSeed, Error, Visitor};

    use super::*;

    pub struct MetaAssetLoadCtx<'srv> {
        pub server: &'srv mut AssetServer,
        pub dependencies: &'srv mut Vec<Cid>,
        pub loc: AssetLocRef<'srv>,
        pub schema: &'static Schema,
    }

    impl<'asset, 'de> DeserializeSeed<'de> for MetaAssetLoadCtx<'asset> {
        type Value = SchemaBox;

        fn deserialize<D>(mut self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            if self.schema.has_opaque() {
                return Err(D::Error::custom(
                    "Cannot deserialize schemas containing opaque types.",
                ));
            }

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

    struct SchemaPtrLoadCtx<'a, 'srv, 'ptr, 'prnt> {
        ctx: &'a mut MetaAssetLoadCtx<'srv>,
        ptr: SchemaRefMut<'ptr, 'prnt>,
    }

    impl<'a, 'srv, 'ptr, 'prnt, 'de> DeserializeSeed<'de> for SchemaPtrLoadCtx<'a, 'srv, 'ptr, 'prnt> {
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
                let relative_path = PathBuf::from(String::deserialize(deserializer)?);
                let path = normalize_path_relative_to(&relative_path, self.ctx.loc.path);
                let handle = self
                    .ctx
                    .server
                    .load_asset((path.as_path(), self.ctx.loc.pack).into())
                    .map_err(|e| D::Error::custom(e.to_string()))?;
                self.ctx
                    .dependencies
                    .push(*self.ctx.server.store.asset_ids.get(&handle).unwrap());
                *self
                    .ptr
                    .try_cast_mut()
                    .map_err(|e| D::Error::custom(e.to_string()))? = handle;
                return Ok(());
            }

            match &self.ptr.schema().kind {
                SchemaKind::Struct(_) => deserializer.deserialize_any(StructVisitor {
                    ptr: self.ptr,
                    ctx: self.ctx,
                })?,
                SchemaKind::Vec(_) => deserializer.deserialize_seq(VecVisitor {
                    ptr: self.ptr,
                    ctx: self.ctx,
                })?,
                SchemaKind::Map { .. } => deserializer.deserialize_map(MapVisitor {
                    ptr: self.ptr,
                    ctx: self.ctx,
                })?,
                SchemaKind::Box(_) => {
                    // SOUND: schema asserts pointer is a SchemaBox.
                    let b = unsafe { self.ptr.deref_mut::<SchemaBox>() };
                    SchemaPtrLoadCtx {
                        ctx: self.ctx,
                        ptr: b.as_mut(),
                    }
                    .deserialize(deserializer)?
                }
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
                        Primitive::Opaque { .. } => panic!(
                            "Cannot deserialize opaque types from metadata files.\
                            This error should have been handled above"
                        ),
                    };
                }
            };

            Ok(())
        }
    }

    struct StructVisitor<'a, 'srv, 'ptr, 'prnt> {
        ctx: &'a mut MetaAssetLoadCtx<'srv>,
        ptr: SchemaRefMut<'ptr, 'prnt>,
    }

    impl<'a, 'srv, 'ptr, 'prnt, 'de> Visitor<'de> for StructVisitor<'a, 'srv, 'ptr, 'prnt> {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            // TODO: write a really nice error message for this.
            write!(
                formatter,
                "asset metadata matching the schema: {:#?}",
                self.ptr.schema()
            )
        }

        fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let field_count = self.ptr.schema().kind.as_struct().unwrap().fields.len();

            for i in 0..field_count {
                let field = self.ptr.get_field(i).unwrap();
                if seq
                    .next_element_seed(SchemaPtrLoadCtx {
                        ctx: self.ctx,
                        ptr: field,
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
                match self.ptr.get_field(&key) {
                    Ok(field) => {
                        map.next_value_seed(SchemaPtrLoadCtx {
                            ctx: self.ctx,
                            ptr: field,
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

    struct VecVisitor<'a, 'srv, 'ptr, 'prnt> {
        ctx: &'a mut MetaAssetLoadCtx<'srv>,
        ptr: SchemaRefMut<'ptr, 'prnt>,
    }

    impl<'a, 'srv, 'ptr, 'prnt, 'de> Visitor<'de> for VecVisitor<'a, 'srv, 'ptr, 'prnt> {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            // TODO: write a really nice error message for this.
            write!(
                formatter,
                "asset metadata matching the schema: {:#?}",
                self.ptr.schema()
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

    struct MapVisitor<'a, 'srv, 'ptr, 'prnt> {
        ctx: &'a mut MetaAssetLoadCtx<'srv>,
        ptr: SchemaRefMut<'ptr, 'prnt>,
    }

    impl<'a, 'srv, 'ptr, 'prnt, 'de> Visitor<'de> for MapVisitor<'a, 'srv, 'ptr, 'prnt> {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            // TODO: write a really nice error message for this.
            write!(
                formatter,
                "asset metadata matching the schema: {:#?}",
                self.ptr.schema()
            )
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            // SOUND: schema asserts this is a SchemaMap.
            let v = unsafe { &mut *(self.ptr.as_ptr() as *mut SchemaMap) };
            if v.key_schema() != String::schema() {
                return Err(A::Error::custom(
                    "Can only deserialize maps with string keys.",
                ));
            }
            while let Some(key) = map.next_key::<String>()? {
                let key = SchemaBox::new(key);
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
}
