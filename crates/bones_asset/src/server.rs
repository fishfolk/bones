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

        if !path_is_metadata(&meta.root) {
            anyhow::bail!(
                "Root asset must be a JSON or YAML file with a name in the form: \
                [filename].[asset_kind].[yaml|json]"
            );
        }

        // Load the asset and produce a handle
        let partial = self.load_asset(&meta.root, None)?;
        let loaded_asset = LoadedAsset {
            cid: partial.cid,
            pack: None,
            pack_dir: None,
            path: meta.root.to_owned(),
            dependencies: partial.dependencies,
            data: partial.data,
        };
        let root_handle = UntypedHandle { rid: Ulid::new() };
        self.store.asset_ids.insert(root_handle, partial.cid);
        self.store.assets.insert(partial.cid, loaded_asset);

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
        let partial = self.load_asset(&meta.root, None)?;
        let loaded_asset = LoadedAsset {
            cid: partial.cid,
            pack: None,
            pack_dir: None,
            path: meta.root.to_owned(),
            dependencies: partial.dependencies,
            data: partial.data,
        };
        let root_handle = UntypedHandle { rid: Ulid::new() };
        self.store.asset_ids.insert(root_handle, partial.cid);
        self.store.assets.insert(partial.cid, loaded_asset);
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
            root: root_handle,
        })
    }

    /// Load the asset
    fn load_asset(&mut self, path: &Path, pack: Option<&str>) -> anyhow::Result<PartialAsset> {
        let contents = self.io.load_file(pack, path)?;
        if path_is_metadata(path) {
            self.load_metadata_asset(path, pack, contents)
        } else {
            self.load_data_asset(path, pack, contents)
        }
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
        let loader = MetadataLoadContext {
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

    pub struct MetadataLoadContext<'a> {
        pub server: &'a mut AssetServer,
        pub dependencies: &'a mut Vec<Cid>,
        pub path: &'a Path,
        pub pack: Option<&'a str>,
        pub schema: Cow<'static, Schema>,
    }

    impl<'a: 'de, 'de> DeserializeSeed<'de> for MetadataLoadContext<'a> {
        type Value = SchemaBox;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            if self.schema.has_opaque() {
                return Err(D::Error::custom(
                    "Cannot deserialize schemas containing opaque types.",
                ));
            }

            // Deserialize primitive types differently
            if let SchemaKind::Primitive(p) = &self.schema.kind {
                return PrimitiveLoader(p).deserialize(deserializer);
            }

            // Allocate the object.
            let layout_info = self.schema.layout_info();
            assert_ne!(layout_info.layout.size(), 0, "Layout size cannot be zero");
            // SAFE: checked layout size is not zero above
            let ptr = unsafe { std::alloc::alloc(layout_info.layout) };

            Ok(match &self.schema.kind {
                SchemaKind::Struct(s) => deserializer.deserialize_map(StructVisitor {
                    schema: &self.schema,
                    layout_info: &layout_info,
                    ctx: &self,
                    struct_schema: s,
                    ptr,
                })?,
                SchemaKind::Vec(v) => deserializer.deserialize_seq(SeqVisitor {
                    ctx: &self,
                    schema: v,
                })?,
                SchemaKind::Primitive(_) => unreachable!("Handled above"),
            })
        }
    }

    struct PrimitiveLoader<'a>(&'a Primitive);

    impl<'a, 'de> DeserializeSeed<'de> for PrimitiveLoader<'a> {
        type Value = SchemaBox;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(match self.0 {
                Primitive::Bool => SchemaBox::new(bool::deserialize(deserializer)?),
                Primitive::U8 => SchemaBox::new(u8::deserialize(deserializer)?),
                Primitive::U16 => SchemaBox::new(u16::deserialize(deserializer)?),
                Primitive::U32 => SchemaBox::new(u32::deserialize(deserializer)?),
                Primitive::U64 => SchemaBox::new(u64::deserialize(deserializer)?),
                Primitive::U128 => SchemaBox::new(u128::deserialize(deserializer)?),
                Primitive::I8 => SchemaBox::new(i8::deserialize(deserializer)?),
                Primitive::I16 => SchemaBox::new(i16::deserialize(deserializer)?),
                Primitive::I32 => SchemaBox::new(i32::deserialize(deserializer)?),
                Primitive::I64 => SchemaBox::new(i64::deserialize(deserializer)?),
                Primitive::I128 => SchemaBox::new(i128::deserialize(deserializer)?),
                Primitive::F32 => SchemaBox::new(f32::deserialize(deserializer)?),
                Primitive::F64 => SchemaBox::new(f64::deserialize(deserializer)?),
                Primitive::String => SchemaBox::new(String::deserialize(deserializer)?),
                Primitive::Opaque { .. } => panic!(
                    "Cannot deserialize opaque types from metadata files.\
                        This error should have been handled above"
                ),
            })
        }
    }

    struct StructVisitor<'a, 'b> {
        pub ctx: &'b MetadataLoadContext<'a>,
        pub schema: &'b Schema,
        pub struct_schema: &'b StructSchema,
        pub layout_info: &'b SchemaLayoutInfo,
        pub ptr: *mut u8,
    }

    impl<'a, 'b, 'de> Visitor<'de> for StructVisitor<'a, 'b> {
        type Value = SchemaBox;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "asset metadata matching the schema: {:#?}",
                self.struct_schema
            )
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            todo!();
        }
    }

    struct SeqVisitor<'a, 'b> {
        pub ctx: &'b MetadataLoadContext<'a>,
        pub schema: &'b Schema,
    }

    impl<'a, 'b, 'de> Visitor<'de> for SeqVisitor<'a, 'b> {
        type Value = SchemaBox;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                formatter,
                "asset metadata matching the schema: {:#?}",
                self.schema
            )
        }

        fn visit_seq<A>(self, _seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            todo!("Implement custom Vec type that can store dynamic layout data.");
        }
    }
}
