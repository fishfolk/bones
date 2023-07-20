use once_cell::sync::Lazy;
use serde::de::DeserializeSeed;
use ulid::Ulid;

use crate::prelude::*;

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

impl AssetServer {
    /// Initialize a new [`AssetServer`].
    pub fn new<Io: AssetIo + 'static>(io: Io) -> Self {
        Self {
            io: Box::new(io),
            store: default(),
            core_schemas: default(),
        }
    }

    /// Register a type with the core schema, and the given name.
    ///
    /// TODO: better docs.
    pub fn register_core_schema<T: HasSchema>(&mut self, name: &str) {
        self.core_schemas
            .insert(name.into(), Cow::Borrowed(T::schema()));
    }

    /// Load the assets.
    ///
    /// All of the assets are immediately loaded synchronously, blocking until load is complete.
    pub fn load_assets(&mut self) -> anyhow::Result<()> {
        let core_pack = self.load_pack(None)?;
        let packs = self
            .io
            .enumerate_packs()?
            .into_iter()
            .map(|name| {
                self.load_pack(Some(&name)).map(|pack| {
                    (
                        AssetPackSpec {
                            id: pack.id,
                            version: pack.version.clone(),
                        },
                        pack,
                    )
                })
            })
            .collect::<Result<_, _>>()?;

        self.store.packs = packs;
        self.store.core_pack = Some(core_pack);

        Ok(())
    }

    /// Load the asset pack with the given folder name, or else the default pack if [`None`].
    pub fn load_pack(&mut self, pack: Option<&str>) -> anyhow::Result<AssetPack> {
        // Load the core pack differently
        if pack.is_none() {
            return self.load_core_pack();
        }

        // Load the asset packfile
        let packfile_contents = self.io.load_file(pack, Path::new("pack.yaml"))?;
        let meta: PackfileMeta = serde_yaml::from_slice(&packfile_contents)?;

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
        let root_handle = self.load_asset(&meta.root, None)?;

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
        let handle = self.load_asset(&meta.root, None)?;
        let game_version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();

        // Return the loaded asset pack.
        Ok(AssetPack {
            name: "Core".into(),
            id: *CORE_PACK_ID,
            version: game_version.clone(),
            game_version: VersionReq {
                comparators: [semver::Comparator {
                    op: semver::Op::Exact,
                    major: game_version.major,
                    minor: Some(game_version.minor),
                    patch: Some(game_version.patch),
                    pre: game_version.pre,
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
        let contents = self.io.load_file(pack, path)?;
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
        let schema = self
            .core_schemas
            .get(schema_name)
            .ok_or_else(|| anyhow::format_err!("Schema not found: {schema_name}"))?
            .clone();
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
        self.store.assets.get(cid).expect(NO_ASSET_MSG).cast()
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
        pub schema: Cow<'static, Schema>,
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
            let mut ptr = SchemaBox::default(self.schema.clone());

            SchemaPtrLoadCtx {
                ctx: &mut self,
                ptr: ptr.as_mut(),
            }
            .deserialize(deserializer)?;

            Ok(ptr)
        }
    }

    struct SchemaPtrLoadCtx<'a, 'srv, 'ptr, 'schm, 'prnt> {
        ctx: &'a mut MetaAssetLoadCtx<'srv>,
        ptr: SchemaPtrMut<'ptr, 'schm, 'prnt>,
    }

    impl<'a, 'srv, 'ptr, 'schm, 'prnt, 'de> DeserializeSeed<'de>
        for SchemaPtrLoadCtx<'a, 'srv, 'ptr, 'schm, 'prnt>
    {
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
                SchemaKind::Vec(_) => todo!("{:#?}", self.ptr.schema()),
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

    struct StructVisitor<'a, 'srv, 'ptr, 'schm, 'prnt> {
        ctx: &'a mut MetaAssetLoadCtx<'srv>,
        ptr: SchemaPtrMut<'ptr, 'schm, 'prnt>,
    }

    impl<'a, 'srv, 'ptr, 'schm, 'prnt, 'de> Visitor<'de>
        for StructVisitor<'a, 'srv, 'ptr, 'schm, 'prnt>
    {
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
}
