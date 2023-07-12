use crate::prelude::*;

/// Context needed to load an asset pack. Can be used to deserialize assets.
pub struct AssetPackLoadCtx<'a> {
    /// Reference to the asset server.
    pub server: &'a AssetServer,
}

/// YAML format for the core asset pack's `pack.yaml` file.
#[derive(Debug, Clone, Deserialize)]
pub struct CorePackileMeta {
    /// The path to the root asset for the pack.
    pub root: PathBuf,
}

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
        // Load the core asset packfile
        let core_packfile_contents = self.io.load_file(None, Path::new("pack.yaml"))?;
        let core_packfile_meta: CorePackileMeta = serde_yaml::from_slice(&core_packfile_contents)?;

        Ok(())
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
