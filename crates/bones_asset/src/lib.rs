//! An asset interface for Bones.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use std::{any::TypeId, collections::hash_map::Entry, marker::PhantomData};

use bones_ecs::ulid::{TypeUlid, Ulid, UlidMap};

/// The prelude.
pub mod prelude {
    pub use crate::*;
}

/// A resource that may be used to access [`AssetProvider`]s for all the different registered asset
/// types.
pub struct AssetProviders {
    providers: UlidMap<Box<dyn UntypedAssetProvider>>,
    type_ids: UlidMap<TypeId>,
}

impl AssetProviders {
    /// Add an asset provider for a specific asset type.
    pub fn add<T, A>(&mut self, provider: A)
    where
        T: TypeUlid + 'static,
        A: AssetProvider<T> + UntypedAssetProvider + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_ulid = T::ulid();

        match self.type_ids.entry(type_ulid) {
            Entry::Occupied(entry) => {
                if entry.get() != &type_id {
                    panic!("Multiple Rust types with the same Type ULID");
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(type_id);
            }
        }

        self.providers.insert(type_ulid, Box::new(provider));
    }

    /// Get the asset provider for the given type
    pub fn get<T: TypeUlid>(&self) -> AssetProviderRef<T> {
        self.try_get::<T>().unwrap()
    }

    /// Get the asset provider for the given asset type, if it exists.
    pub fn try_get<T: TypeUlid>(&self) -> Option<AssetProviderRef<T>> {
        self.providers.get(&T::ulid()).map(|x| {
            let untyped = x.as_ref();

            AssetProviderRef {
                untyped,
                _phantom: PhantomData,
            }
        })
    }

    /// Get the asset provider for the given type
    pub fn get_mut<T: TypeUlid>(&mut self) -> AssetProviderMut<T> {
        self.try_get_mut::<T>().unwrap()
    }

    /// Get the asset provider for the given asset type, if it exists.
    pub fn try_get_mut<T: TypeUlid>(&mut self) -> Option<AssetProviderMut<T>> {
        self.providers.get_mut(&T::ulid()).map(|x| {
            let untyped = x.as_mut();

            AssetProviderMut {
                untyped,
                _phantom: PhantomData,
            }
        })
    }
}

/// Trait implemented for asset providers that can return untyped pointers to their assets.
pub trait UntypedAssetProvider {
    /// Returns a read-only pointer to the asset for the given handle, or a null pointer if it
    /// doesn't exist.
    fn get(&self, handle: UntypedHandle) -> *const u8;
    /// Returns a mutable-only pointer to the asset for the given handle, or a null pointer if it
    /// doesn't exist.
    fn get_mut(&mut self, handle: UntypedHandle) -> *mut u8;
}

/// Trait for asset providers.
///
/// Asset providers are reponsible for returning references to assets out of their backing asset store, when giving handles to the asset to laod
pub trait AssetProvider<T: TypeUlid> {
    /// Get a reference to an asset, if it exists in the store.
    fn get(&self, handle: Handle<T>) -> Option<&T>;
    /// Get a mutable reference to an asset, if it exists in the store.
    fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T>;
}

impl<T: TypeUlid> UntypedAssetProvider for dyn AssetProvider<T> {
    fn get(&self, handle: UntypedHandle) -> *const u8 {
        let asset = <Self as AssetProvider<T>>::get(self, handle.typed());
        asset
            .map(|x| x as *const T as *const u8)
            .unwrap_or(std::ptr::null())
    }

    fn get_mut(&mut self, handle: UntypedHandle) -> *mut u8 {
        let asset = <Self as AssetProvider<T>>::get_mut(self, handle.typed());
        asset
            .map(|x| x as *mut T as *mut u8)
            .unwrap_or(std::ptr::null_mut())
    }
}

/// A borrow of an [`AssetProvider`].
pub struct AssetProviderRef<'a, T: TypeUlid> {
    untyped: &'a dyn UntypedAssetProvider,
    _phantom: PhantomData<T>,
}

impl<'a, T: TypeUlid> AssetProviderRef<'a, T> {
    /// Get an asset, given it's handle
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        let ptr = self.untyped.get(handle.untyped()) as *const T;

        if ptr.is_null() {
            None
        } else {
            // SAFE: AssetProviderRef may only be constructed by us, and we only construct it ( see
            // AssetProviders ) when we know the untyped provider matches T.
            unsafe { Some(&*ptr) }
        }
    }
}

/// A mutable borrow of an [`AssetProvider`].
pub struct AssetProviderMut<'a, T: TypeUlid> {
    untyped: &'a mut dyn UntypedAssetProvider,
    _phantom: PhantomData<T>,
}

impl<'a, T: TypeUlid> AssetProviderMut<'a, T> {
    /// Get an asset, given it's handle
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        let ptr = self.untyped.get(handle.untyped()) as *const T;

        if ptr.is_null() {
            None
        } else {
            // SAFE: AssetProviderRef may only be constructed by us, and we only construct it ( see
            // AssetProviders ) when we know the untyped provider matches T.
            unsafe { Some(&*ptr) }
        }
    }

    /// Get an asset, given it's handle
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        let ptr = self.untyped.get_mut(handle.untyped()) as *mut T;

        if ptr.is_null() {
            None
        } else {
            // SAFE: AssetProviderRef may only be constructed by us, and we only construct it ( see
            // AssetProviders ) when we know the untyped provider matches T.
            unsafe { Some(&mut *ptr) }
        }
    }
}

/// A typed handle to an asset.
///
/// The type of the handle is used to help reduce errros at runtime, but internally, the handle's
/// only data is it's [`Ulid`] `id`.
///
/// It can be converted to an untyped handle with the [`untyped()`][Self::untyped] method.
#[derive(Copy, Clone, Debug)]
pub struct Handle<T: TypeUlid> {
    /// The unique identifier of the asset this handle represents.
    pub id: Ulid,
    phantom: PhantomData<T>,
}

impl<T: TypeUlid> Default for Handle<T> {
    fn default() -> Self {
        Self {
            id: Default::default(),
            phantom: Default::default(),
        }
    }
}

impl<T: TypeUlid> Handle<T> {
    /// Convert the handle to an [`UntypedHandle`].
    pub fn untyped(self) -> UntypedHandle {
        UntypedHandle { id: self.id }
    }
}

/// An untyped handle to an asset.
///
/// This simply contains the asset's unique [`Ulid`] id.
///
/// Can be converted to a typed handle with the [`typed()`][Self::typed] method.
#[derive(Default, Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct UntypedHandle {
    /// The unique identifier of the asset this handle represents.
    pub id: Ulid,
}

impl UntypedHandle {
    /// Create a new, random [`UntypedHandle`].
    pub fn new() -> Self {
        Self { id: Ulid::new() }
    }

    /// Create a typed [`Handle<T>`] from this [`UntypedHandle`].
    pub fn typed<T: TypeUlid>(self) -> Handle<T> {
        Handle {
            id: self.id,
            phantom: PhantomData,
        }
    }
}
