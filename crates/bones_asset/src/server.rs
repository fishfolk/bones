use std::path::{Path, PathBuf};

use anyhow::Context;
use once_cell::sync::Lazy;
use semver::VersionReq;
use serde::{de::DeserializeSeed, Deserialize};
use ulid::Ulid;

use crate::prelude::*;
use bones_utils::prelude::*;

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

impl AssetServer {
    /// Initialize a new [`AssetServer`].
    pub fn new<Io: AssetIo + 'static>(io: Io, version: Version) -> Self {
        Self {
            io: Box::new(io),
            game_version: version,
            store: default(),
            core_schemas: default(),
            incompabile_packs: default(),
        }
    }

    /// Register a type with the core schema, and the given name.
    ///
    /// TODO: better docs.
    pub fn register_core_schema<T: HasSchema>(&mut self, name: &str) {
        self.core_schemas.insert(name.into(), T::schema());
    }

    /// Load the assets.
    ///
    /// All of the assets are immediately loaded synchronously, blocking until load is complete.
    pub fn load_assets(&mut self) -> anyhow::Result<()> {
        let core_pack = self.load_pack(None)?;
        let mut packs = HashMap::new();

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
        let packfile_contents = self.io.load_file(pack, Path::new("pack.yaml"))?;
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
        let root_handle = self.load_asset(&meta.root, pack).map_err(|e| {
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
            // TODO: load & import schemas.
            schemas: default(),
            import_schemas: default(),
            root: root_handle,
        })
    }

    /// Load the core asset pack.
    pub fn load_core_pack(&mut self) -> anyhow::Result<AssetPack> {
        // Load the core asset packfile
        let packfile_contents = self.io.load_file(None, Path::new("pack.yaml"))?;
        let meta: CorePackfileMeta = serde_yaml::from_slice(&packfile_contents)?;

        if !path_is_metadata(&meta.root) {
            anyhow::bail!(
                "Root asset must be a JSON or YAML file with a name in the form: \
                [filename].[asset_kind].[yaml|json]"
            );
        }

        // Load the asset and produce a handle
        let handle = self
            .load_asset(&meta.root, None)
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

    /// Load the asset
    fn load_asset(&mut self, path: &Path, pack: Option<&str>) -> anyhow::Result<UntypedHandle> {
        let contents = self
            .io
            .load_file(pack, path)
            .context(format!("Could not load asset file: {path:?}"))?;
        let partial = if path_is_metadata(path) {
            self.load_metadata_asset(path, pack, contents)
        } else {
            self.load_data_asset(path, pack, contents)
        }?;
        let loaded_asset = LoadedAsset {
            cid: partial.cid,
            pack: pack.map(|x| {
                self.store
                    .pack_dirs
                    .get(x)
                    .expect("Pack dir not loaded properly")
                    .clone()
            }),
            pack_dir: None,
            path: path.to_owned(),
            dependencies: partial.dependencies,
            data: partial.data,
        };
        let handle = UntypedHandle { rid: Ulid::new() };
        self.store.asset_ids.insert(handle, partial.cid);
        self.store.assets.insert(partial.cid, loaded_asset);

        Ok(handle)
    }

    fn load_metadata_asset(
        &mut self,
        path: &Path,
        pack: Option<&str>,
        contents: Vec<u8>,
    ) -> anyhow::Result<PartialAsset> {
        // Get the schema for the asset
        let filename = path
            .file_name()
            .ok_or_else(|| anyhow::format_err!("Invalid asset filename"))?
            .to_str()
            .ok_or_else(|| anyhow::format_err!("Invalid unicode in filename"))?;
        let (_name, schema_name) = filename
            .rsplit_once('.')
            .unwrap()
            .0
            .rsplit_once('.')
            .ok_or_else(|| anyhow::format_err!("Missing schema name in asset filename"))?;
        let schema = *self
            .core_schemas
            .get(schema_name)
            .ok_or_else(|| anyhow::format_err!("Schema not found: {schema_name}"))?;
        let mut dependencies = Vec::new();

        let mut cid = Cid::default();
        cid.update(&contents);
        let loader = MetaAssetLoadCtx {
            server: self,
            path,
            pack,
            schema,
            dependencies: &mut dependencies,
        };
        let data = if path.extension().unwrap().to_str().unwrap() == "json" {
            let mut deserializer = serde_json::Deserializer::from_slice(&contents);
            loader.deserialize(&mut deserializer)?
        } else {
            let deserializer = serde_yaml::Deserializer::from_slice(&contents);
            loader.deserialize(deserializer)?
        };

        // Update the CI
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
        _path: &Path,
        _pack: Option<&str>,
        _contents: Vec<u8>,
    ) -> anyhow::Result<PartialAsset> {
        todo!()
    }

    /// Borrow a [`LoadedAsset`] associated to the given handle.
    pub fn get_untyped(&self, handle: &UntypedHandle) -> Option<&LoadedAsset> {
        let cid = self.store.asset_ids.get(handle)?;
        self.store.assets.get(cid)
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
    pub fn get<T: HasSchema>(&self, handle: &Handle<T>) -> &T {
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
        return false
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
        pub path: &'srv Path,
        pub pack: Option<&'srv str>,
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
                let path = normalize_path_relative_to(&relative_path, self.ctx.path);
                let handle = self
                    .ctx
                    .server
                    .load_asset(&path, self.ctx.pack)
                    .map_err(|e| D::Error::custom(e.to_string()))?;
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
                SchemaKind::Map { .. } => todo!(),
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
            // FIXME: write a really nice error message for this.
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
            // FIXME: write a really nice error message for this.
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
            // TODO: Is there a safe way to do this?
            let v = unsafe { &mut *(self.ptr.ptr().as_ptr() as *mut SchemaVec) };
            loop {
                let item_schema = self.ptr.schema().kind.as_vec().unwrap();
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
}
