use serde::{Deserialize, Serialize};

#[cfg(feature = "cid_debug_trace")]
pub(crate) use cid_debug_trace::*;

/// A unique content ID.
///
/// Represents the Sha-256 hash of the contents of a [`LoadedAsset`][crate::LoadedAsset].
#[derive(Clone, Copy, Eq, PartialEq, Hash, Default, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Cid(pub [u8; 32]);

impl std::fmt::Display for Cid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

impl std::fmt::Debug for Cid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cid({})", self)
    }
}

impl Cid {
    /// Update the CID by combining it's current data with the hash of the provided bytes.
    pub fn update(&mut self, bytes: &[u8]) {
        use sha2::Digest;
        let pre_hash = self.0;
        let mut hasher = sha2::Sha256::new();
        hasher.update(pre_hash);
        hasher.update(bytes);
        let result = hasher.finalize();
        self.0.copy_from_slice(&result);
    }
}

#[cfg(feature = "cid_debug_trace")]
mod cid_debug_trace {

    use crate::{AssetLoc, Cid};
    use std::path::Path;

    use bones_utils::{default, Ustr};

    pub(crate) struct CidDebugTrace<'a> {
        pub schema_full_name: Ustr,
        pub file_path: &'a Path,

        pub cid_after_schema_fullname: Cid,
        pub cid_after_contents: Cid,

        /// Tuple of dep_cid, updated cid, and dep asset loc
        pub cid_after_deps: Vec<(Cid, Cid, Option<AssetLoc>)>,

        pub final_cid: Cid,
    }

    impl<'a> CidDebugTrace<'a> {
        pub(crate) fn new(schema_full_name: Ustr, file_path: &'a Path) -> Self {
            Self {
                schema_full_name,
                file_path,
                cid_after_schema_fullname: default(),
                cid_after_contents: default(),
                cid_after_deps: default(),
                final_cid: default(),
            }
        }
    }

    impl<'a> std::fmt::Display for CidDebugTrace<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // dump asset meta
            writeln!(
                f,
                "Cid trace schema: {:?} file path: {:?}",
                self.schema_full_name, self.file_path
            )?;
            writeln!(f, "Trace is in order of updates, which impacts result")?;

            // cid schema fullname update
            writeln!(
                f,
                "[Intermediate] Cid from schema fullname: {:?} cid: {}",
                self.schema_full_name, self.cid_after_schema_fullname
            )?;

            // cid content update
            writeln!(
                f,
                "[Intermediate] Cid from contents: cid: {}",
                self.cid_after_contents
            )?;

            // cid dependency update
            writeln!(f, "Dumping updates from sorted dependency cids:")?;
            for (dep_cid, updated_cid, dep_asset_loc) in self.cid_after_deps.iter() {
                writeln!(
                    f,
                    "    dep_cid: {}, cid: {}, dep_asset_loc: {:?}",
                    dep_cid, updated_cid, dep_asset_loc
                )?;
            }

            // final cid
            writeln!(f, "Final cid: {}", self.final_cid)?;

            Ok(())
        }
    }
}
