//! Bones ECS system parameters.

use std::{cell::RefCell, pin::Pin};

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
    pub fn iter(&self) -> PacksIter<'_, T> {
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

/// A helper system param that allows for iteration over data contained in the
/// core game asset pack (the one in the `assets/` directory) and the supplementary asset
/// packs (the sub-directories of the `packs/` directory).
///
/// This is intended for use with lists contained in the asset packs. For example, the core and
/// supplementary asset packs may contain lists of characters that users can choose to play as.
/// This system param may be used to iterate all of the characters available in all of the asset
/// packs.
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
/// struct GameMeta {
///     maps: SVec<Handle<MapMeta>>,
/// }
///
/// #[derive(Clone, Default, HasSchema)]
/// #[type_data(metadata_asset("pack"))]
/// #[repr(C)]
/// struct PackMeta {
///     maps: SVec<Handle<MapMeta>>,
/// }
///
/// #[derive(Clone, Default, HasSchema)]
/// #[type_data(metadata_asset("root"))]
/// struct MapMeta {
///     name: String,
/// }
///
/// // Log the names of all maps in the core and other asset packs.
/// fn test(asset_server: Res<AssetServer>, packs: AllPacksData<GameMeta, PackMeta>) {
///     for handle in packs.iter_with(
///         |game: &GameMeta| game.maps.iter().copied(),
///         |pack: &PackMeta| pack.maps.iter().copied()
///     ) {
///         let map_meta = asset_server.get(handle);
///         info!(name = map_meta.name, "map");
///     }
/// }
///
/// // Make sure that `AllPacksData` is a valid system param.
/// IntoSystem::system(test);
/// ```
///
pub struct AllPacksData<'a, Core, Pack>
where
    Core: HasSchema,
    Pack: HasSchema,
{
    core_root: Root<'a, Core>,
    pack_roots: Packs<'a, Pack>,
}

impl<'a, Core, Pack> SystemParam for AllPacksData<'a, Core, Pack>
where
    Core: HasSchema,
    Pack: HasSchema,
{
    type State = (
        <Root<'a, Core> as SystemParam>::State,
        <Packs<'a, Pack> as SystemParam>::State,
    );
    type Param<'s> = AllPacksData<'s, Core, Pack>;

    fn get_state(world: &World) -> Self::State {
        (
            Root::<'a, Core>::get_state(world),
            Packs::<'a, Pack>::get_state(world),
        )
    }

    fn borrow<'s>(world: &'s World, state: &'s mut Self::State) -> Self::Param<'s> {
        AllPacksData {
            core_root: Root::<'s, Core>::borrow(world, &mut state.0),
            pack_roots: Packs::<'s, Pack>::borrow(world, &mut state.1),
        }
    }
}

impl<'a, Core, Pack> AllPacksData<'a, Core, Pack>
where
    Core: HasSchema,
    Pack: HasSchema,
{
    /// Get the iterator over the core and supplementary asset packs.
    ///
    /// The first argument `core_accessor` is a function that must produce an iterator of `T` from
    /// the core asset metadata. This is only called once, prior to iteration.
    ///
    /// Similarly, the second argument `pack_accessor` is a function that must produce an iterator
    /// of `T` from a pack asset metadata. This is called once per pack, during iteration.
    pub fn iter_with<T, CoreItemIt: Iterator<Item = T>, PackItemIt: Iterator<Item = T>>(
        &'a self,
        mut core_accessor: impl FnMut(&'a Core) -> CoreItemIt,
        pack_accessor: impl 'static + FnMut(&'a Pack) -> PackItemIt,
    ) -> AllPacksDataIter<'a, T, CoreItemIt, Pack, PacksIter<'a, Pack>, PackItemIt> {
        AllPacksDataIter {
            core_item_iter: core_accessor(&*self.core_root),
            pack_iter: self.pack_roots.iter(),
            pack_accessor: Box::new(pack_accessor),
            current_pack: AllPacksDataCurrentPack::new(),
        }
    }
}

/// A flattened iterator of items of type `T` from data within the core and supplementary asset
/// packs. Items are first yielded from the core asset pack until exhausted, then items are yielded
/// from the supplementary asset packs, one at a time, and in no particular order.
///
/// Can be acquired from [`AllPacksData::iter_with`] which takes two functions that produce the
/// inner iterator of items from the game meta and the inner iterators of items from the asset
/// packs, respectively.
///
/// See [`AllPacksData`] for more info.
pub struct AllPacksDataIter<'a, T, CoreItemIt, Pack, PackIt, PackItemIt> {
    core_item_iter: CoreItemIt,
    pack_iter: PackIt,
    pack_accessor: Box<dyn FnMut(&'a Pack) -> PackItemIt>,
    current_pack: Pin<Box<RefCell<AllPacksDataCurrentPack<'a, Pack, T, PackItemIt>>>>,
}

struct AllPacksDataCurrentPack<'a, Pack, T, PackItemIt> {
    pack: Option<DashmapRef<'a, Pack>>,
    item_iter: Option<PackItemIt>,
    _marker: std::marker::PhantomData<fn() -> T>,
}

impl<'a, Pack, T, PackItemIt> AllPacksDataCurrentPack<'a, Pack, T, PackItemIt> {
    fn new() -> Pin<Box<RefCell<Self>>> {
        Box::pin(RefCell::new(AllPacksDataCurrentPack {
            pack: None,
            item_iter: None,
            _marker: std::marker::PhantomData,
        }))
    }

    fn set_pack(
        &mut self,
        next_pack: DashmapRef<'a, Pack>,
        pack_accessor: &mut dyn FnMut(&'a Pack) -> PackItemIt,
    ) -> &mut PackItemIt {
        // Drop the item iterator
        _ = self.item_iter.take();

        // Set the pack
        let pack = self.pack.insert(next_pack);

        // Setup the item iterator
        let pack = unsafe { std::mem::transmute::<&Pack, &'a Pack>(pack) };
        let pack_item_iter = (pack_accessor)(pack);
        self.item_iter.insert(pack_item_iter)
    }
}

impl<'a, Pack, T, CoreItemIt, PackIt, PackItemIt> Iterator
    for AllPacksDataIter<'a, T, CoreItemIt, Pack, PackIt, PackItemIt>
where
    CoreItemIt: Iterator<Item = T>,
    PackIt: Iterator<Item = DashmapRef<'a, Pack>>,
    PackItemIt: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let next_core_item = self.core_item_iter.next();
        if next_core_item.is_some() {
            return next_core_item;
        }

        if let Some(iter) = self.current_pack.borrow_mut().item_iter.as_mut() {
            let next_pack_item = iter.next();
            if next_pack_item.is_some() {
                return next_pack_item;
            }
        }

        let next_pack = self.pack_iter.next()?;

        let mut current_pack = self.current_pack.borrow_mut();
        let current_pack_item_iter = current_pack.set_pack(next_pack, &mut self.pack_accessor);

        current_pack_item_iter.next()
    }
}
