use crate::prelude::*;

/// Context needed to load an asset pack. Can be used to deserialize assets.
pub struct AssetPackLoadCtx<'a> {
    /// Reference to the asset server.
    pub server: &'a AssetServer,
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
        _path: &Path,
        _pack: Option<&str>,
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
        let cid = self.store.asset_ids.get(&handle.rid)?;
        self.store.assets.get(cid)
    }
}
