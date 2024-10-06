//! Bones ECS system parameters.

use crate::prelude::*;

type DashmapRef<'a, T> = dashmap::mapref::one::MappedRef<'a, Cid, LoadedAsset, T>;

type DashmapIter<'a, K, V> = dashmap::iter::Iter<'a, K, V>;

/// Get the root asset of the core asset pack and cast it to type `T`.
pub struct Root<'a, T: HasSchema>(DashmapRef<'a, T>);

impl<'a, T: HasSchema> std::ops::Deref for Root<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T: HasSchema> SystemParam for Root<'a, T> {
    type State = AssetServer;
    type Param<'s> = Root<'s, T>;

    fn get_state(world: &World) -> Self::State {
        (*world.resources.get::<AssetServer>().unwrap()).clone()
    }
    fn borrow<'s>(_world: &'s World, asset_server: &'s mut Self::State) -> Self::Param<'s> {
        Root(asset_server.root())
    }
}

/// A helper system param for iterating over the root assets of the (non-core) asset packs, each
/// casted to type `T`.
///
/// Asset packs contain a root asset in the form of an untyped asset handle. Use the
/// [`iter`][Self::iter] method to get an iterator over all asset packs.
///
/// ## Example
///
/// ```rust
/// use bones_framework::prelude::*;
/// use tracing::info;
///
/// #[derive(Clone, Default, HasSchema)]
/// #[type_data(metadata_asset("root"))]
/// #[repr(C)]
/// struct PackMeta {
///     name: String,
/// }
///
/// // Log the names of all non-core asset packs.
/// fn test(packs: Packs<PackMeta>) -> Vec<String> {
///     let mut names = Vec::new();
///     for pack in packs.iter() {
///         names.push(pack.name.clone());
///     }
///     names
/// }
///
/// // Make sure that `Packs` is a valid system param.
/// IntoSystem::system(test);
/// ```
///
pub struct Packs<'a, T> {
    asset_server: AssetServer,
    _pack_t: std::marker::PhantomData<&'a T>,
}

impl<'a, T: HasSchema> SystemParam for Packs<'a, T> {
    type State = AssetServer;
    type Param<'s> = Packs<'s, T>;

    fn get_state(world: &World) -> Self::State {
        (*world.resource::<AssetServer>()).clone()
    }

    fn borrow<'s>(_world: &'s World, state: &'s mut Self::State) -> Self::Param<'s> {
        Packs {
            asset_server: state.clone(),
            _pack_t: std::marker::PhantomData,
        }
    }
}

impl<T> Packs<'_, T> {
    /// Get the typed asset pack roots iterator.
    pub fn iter(&self) -> PacksIter<T> {
        PacksIter {
            asset_server: &self.asset_server,
            asset_packs_iter: self.asset_server.packs().iter(),
            _pack_t: std::marker::PhantomData,
        }
    }
}

/// A typed iterator over asset pack roots.
pub struct PacksIter<'a, T> {
    asset_server: &'a AssetServer,
    asset_packs_iter: DashmapIter<'a, AssetPackSpec, AssetPack>,
    _pack_t: std::marker::PhantomData<&'a T>,
}

impl<'a, T: HasSchema> Iterator for PacksIter<'a, T> {
    type Item = DashmapRef<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let pack = self.asset_packs_iter.next()?;
        let pack_root = self.asset_server.get(pack.root.typed::<T>());
        Some(pack_root)
    }
}
