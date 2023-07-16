use once_cell::sync::Lazy;
use serde::de::DeserializeSeed;
use ulid::Ulid;

use crate::prelude::*;

/// Context needed to load an asset pack. Can be used to deserialize assets.
pub struct AssetPackLoadCtx<'a> {
    /// Reference to the asset server.
    pub server: &'a AssetServer,
}

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

    /// Load the asset pack with the given name, or else the default pack if [`None`].
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
        let root_asset = self.load_asset(&meta.root, pack)?;
        let root_handle = UntypedHandle { rid: Ulid::new() };
        self.store.asset_ids.insert(root_handle, root_asset.cid);
        self.store.assets.insert(root_asset.cid, root_asset);

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
        let root_asset = self.load_asset(&meta.root, None)?;
        let root_handle = UntypedHandle { rid: Ulid::new() };
        self.store.asset_ids.insert(root_handle, root_asset.cid);
        self.store.assets.insert(root_asset.cid, root_asset);
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
    pub fn load_asset(&mut self, path: &Path, pack: Option<&str>) -> anyhow::Result<LoadedAsset> {
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
    ) -> anyhow::Result<LoadedAsset> {
        // Get the schema for the asset
        let filename = path
            .file_name()
            .ok_or_else(|| anyhow::format_err!("Invalid asset filename"))?
            .to_str()
            .ok_or_else(|| anyhow::format_err!("Invalid unicode in filename"))?;
        let (_name, schema_name) = filename
            .rsplit_once('.')
            .ok_or_else(|| anyhow::format_err!("Missing schema name in asset filename"))?;
        let schema = self
            .core_schemas
            .get(schema_name)
            .ok_or_else(|| anyhow::format_err!("Schema not found: {schema_name}"))?
            .clone();

        let loader = MetadataLoadContext {
            server: self,
            path,
            pack,
            schema,
        };
        if path.extension().unwrap().to_str().unwrap() == "json" {
            let mut deserializer = serde_json::Deserializer::from_slice(&contents);

            Ok(loader.deserialize(&mut deserializer)?)
        } else {
            let deserializer = serde_yaml::Deserializer::from_slice(&contents);
            Ok(loader.deserialize(deserializer)?)
        }
    }

    fn load_data_asset(
        &mut self,
        _path: &Path,
        _pack: Option<&str>,
        _contents: Vec<u8>,
    ) -> anyhow::Result<LoadedAsset> {
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

const NO_ASSET_MSG: &str = "Asset not loaded";
fn path_is_metadata(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false
    };

    ext == "yaml" || ext == "yml" || ext == "json"
}

use metadata::*;
mod metadata {
    use serde::de::DeserializeSeed;

    use super::*;

    pub struct MetadataLoadContext<'a> {
        pub server: &'a mut AssetServer,
        pub path: &'a Path,
        pub pack: Option<&'a str>,
        pub schema: Cow<'static, Schema>,
    }

    impl<'a, 'de> DeserializeSeed<'de> for MetadataLoadContext<'a> {
        type Value = LoadedAsset;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            match &self.schema.kind {
                SchemaKind::Struct(_) => todo!(),
                SchemaKind::Vec(_) => todo!(),
                SchemaKind::Primitive(_) => todo!(),
            }
        }
    }
}
