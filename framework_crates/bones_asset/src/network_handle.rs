use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Asset handle that may be replicated over network and converted back into [`Handle`] or [`UntypedHandle`].
#[derive(Serialize, Deserialize)]
pub struct NetworkHandle<T> {
    /// Content id of the asset, used to lookup asset from [`AssetServer`].
    pub cid: Cid,
    phantom: PhantomData<T>,
}

impl<T> NetworkHandle<T> {
    /// Create [`NetworkHandle`] from content id ([`Cid`]).
    pub fn from_cid(cid: Cid) -> Self {
        Self {
            cid,
            phantom: PhantomData,
        }
    }

    /// Create asset [`Handle`] by looking up [`NetworkHandle`]'s [`Cid`] in [`AssetServer`].
    /// Panics if Cid not found in asset server.
    pub fn into_handle(&self, asset_server: &AssetServer) -> Handle<T> {
        asset_server
            .try_get_handle_from_cid(&self.cid)
            .expect("Failed to lookup NetworkHandle content id in AssetServer. Is asset loaded? Invalid Cid?")
    }

    /// Convert into [`UntypedHandle`].
    /// Panics if [`AssetServer`] fails to find handle of asset loaded with [`Cid`].
    pub fn into_untyped_handle(&self, asset_server: &AssetServer) -> UntypedHandle {
        asset_server
            .try_get_untyped_handle_from_cid(&self.cid)
            .expect("Failed to lookup NetworkHandle content id in AssetServer. Is asset loaded? Invalid Cid?")
    }
}
