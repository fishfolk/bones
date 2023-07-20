use std::path::PathBuf;

use bones_utils::HashMap;

pub struct DummyIo {
    core: HashMap<PathBuf, Vec<u8>>,
    packs: HashMap<String, HashMap<PathBuf, Vec<u8>>>,
}

impl DummyIo {
    pub fn new<'a, I: IntoIterator<Item = (&'a str, Vec<u8>)>>(core: I) -> Self {
        Self {
            core: core
                .into_iter()
                .map(|(p, d)| (PathBuf::from(p), d))
                .collect(),
            packs: Default::default(),
        }
    }
}

impl bones_asset::AssetIo for DummyIo {
    fn enumerate_packs(&self) -> anyhow::Result<Vec<String>> {
        Ok(self.packs.keys().cloned().collect())
    }

    fn load_file(
        &self,
        pack_folder: Option<&str>,
        path: &std::path::Path,
    ) -> anyhow::Result<Vec<u8>> {
        let err = || {
            anyhow::format_err!(
                "File not found: `{:?}` in pack `{:?}`",
                path,
                pack_folder.unwrap_or("[core]")
            )
        };
        if let Some(pack_folder) = pack_folder {
            self.packs
                .get(pack_folder)
                .ok_or_else(err)?
                .get(path)
                .cloned()
                .ok_or_else(err)
        } else {
            self.core.get(path).cloned().ok_or_else(err)
        }
    }

    fn watch(&self) -> Option<async_channel::Receiver<bones_asset::AssetChange>> {
        None
    }
}
