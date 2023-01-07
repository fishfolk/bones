//! An asset interface for Bones.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use std::{
    any::TypeId,
    collections::hash_map::Entry,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};

use bones_ecs::{
    prelude::{AtomicRefCell, Deref, DerefMut},
    ulid::{TypeUlid, UlidMap},
};

/// The prelude.
pub mod prelude {
    pub use crate::*;
}

/// A resource that may be used to access [`AssetProvider`]s for all the different registered asset
/// types.
///
/// > ⚠️ **Warning:** This API is work-in-progress and has not been used in an actual project yet.
/// > In the official Bevy integration we are currently "cheating" by just borrowing asset directly
/// > from the Bevy world.
/// >
/// > Cheating like this means that we can't acess assets through the C API that we want to provide
/// > later, and it isn't very ergonomic, so we will want to make something like this
/// > [`AssetProviders`] resource work properly later.
#[derive(Default)]
pub struct AssetProviders {
    providers: UlidMap<Box<dyn UntypedAssetProvider>>,
    type_ids: UlidMap<TypeId>,
}

/// Type alias for getting the [`AssetProviders`] resource.
pub type ResAssetProviders<'a> = bones_ecs::system::Res<'a, AssetProvidersResource>;

/// The type of the [`AssetProviders`] resource.
// TODO: Make a custom system parameter to prevent needing to manualy .borrow() this resource.
#[derive(Deref, DerefMut, Clone, TypeUlid)]
#[ulid = "01GNWY5HKV5JZQRKG20ANJXHCK"]

pub struct AssetProvidersResource(pub Arc<AtomicRefCell<AssetProviders>>);

impl Default for AssetProvidersResource {
    fn default() -> Self {
        Self(Arc::new(AtomicRefCell::new(AssetProviders::default())))
    }
}

impl AssetProviders {
    /// Add an asset provider for a specific asset type.
    pub fn add<T, A>(&mut self, provider: A)
    where
        T: TypeUlid + 'static,
        A: AssetProvider<T> + UntypedAssetProvider + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_ulid = T::ULID;

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
        self.providers.get(&T::ULID).map(|x| {
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
        self.providers.get_mut(&T::ULID).map(|x| {
            let untyped = x.as_mut();

            AssetProviderMut {
                untyped,
                _phantom: PhantomData,
            }
        })
    }

    /// Remove an asset provider.
    pub fn remove<T: TypeUlid>(&mut self) -> Box<dyn UntypedAssetProvider> {
        self.try_remove::<T>().unwrap()
    }

    /// Remove an asset provider.
    pub fn try_remove<T: TypeUlid>(&mut self) -> Option<Box<dyn UntypedAssetProvider>> {
        self.providers.remove(&T::ULID)
    }
}

/// Trait implemented for asset providers that can return untyped pointers to their assets.
pub trait UntypedAssetProvider: Sync + Send {
    /// Returns a read-only pointer to the asset for the given handle, or a null pointer if it
    /// doesn't exist.
    fn get(&self, handle: UntypedHandle) -> *const u8;
    /// Returns a mutable-only pointer to the asset for the given handle, or a null pointer if it
    /// doesn't exist.
    fn get_mut(&mut self, handle: UntypedHandle) -> *mut u8;
}

/// Trait for asset providers.
///
/// Asset providers are reponsible for returning references to assets out of their backing asset
/// store, when giving handles to the asset to laod
pub trait AssetProvider<T: TypeUlid>: Sync + Send {
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

/// A path to an asset.
///
/// This is a virtual filesystem path, and may not actually refer to physical files.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPath {
    /// The virtual filesystem path
    pub path: Arc<Path>,
    /// The optional sub-asset label
    pub label: Option<Arc<str>>,
}

impl AssetPath {
    /// Create a new asset path.
    pub fn new<P: Into<PathBuf>>(path: P, label: Option<String>) -> Self {
        AssetPath {
            path: Arc::from(path.into()),
            label: label.map(Arc::from),
        }
    }

    /// Take this path, treat it as a path relative to `base_path`, normalize it, and update `self`
    /// with the result.
    pub fn normalize_relative_to(&mut self, base_path: &Path) {
        fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
            let mut components = path.components().peekable();
            let mut ret = if let Some(c @ std::path::Component::Prefix(..)) = components.peek() {
                let buf = std::path::PathBuf::from(c.as_os_str());
                components.next();
                buf
            } else {
                std::path::PathBuf::new()
            };

            for component in components {
                match component {
                    std::path::Component::Prefix(..) => unreachable!(),
                    std::path::Component::RootDir => {
                        ret.push(component.as_os_str());
                    }
                    std::path::Component::CurDir => {}
                    std::path::Component::ParentDir => {
                        ret.pop();
                    }
                    std::path::Component::Normal(c) => {
                        ret.push(c);
                    }
                }
            }

            ret
        }

        let is_relative = !self.path.starts_with(Path::new("/"));

        let path = if is_relative {
            let base = base_path.parent().unwrap_or_else(|| Path::new(""));
            base.join(&self.path)
        } else {
            self.path.strip_prefix("/").unwrap().to_owned()
        };

        self.path = Arc::from(normalize_path(&path));
    }
}

impl Default for AssetPath {
    fn default() -> Self {
        Self {
            path: Arc::from(PathBuf::default()),
            label: Default::default(),
        }
    }
}

/// A typed handle to an asset.
///
/// The type of the handle is used to help reduce runtime errors arising from mis-matching handle
/// types, but internally, the handle's only stored data is it's [`AssetPath`].
///
/// You can change the type of a handle by converting it to an untyped handle with
/// [`untyped()`][Self::untyped] and converting it back to a typed handle with
/// [`typed()`][UntypedHandle::typed].
#[derive(PartialEq, Eq, Hash)]
pub struct Handle<T: TypeUlid> {
    /// The [`AssetPath`] for the asset.
    pub path: AssetPath,
    phantom: PhantomData<T>,
}

impl<T: TypeUlid> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            phantom: self.phantom,
        }
    }
}

impl<T: TypeUlid> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle").field("path", &self.path).finish()
    }
}

impl<T: TypeUlid> Handle<T> {
    /// Create a new asset handle, from it's path and label.
    pub fn new<P: Into<PathBuf>>(path: P, label: Option<String>) -> Self {
        Handle {
            path: AssetPath::new(path, label),
            phantom: PhantomData,
        }
    }
}

impl<T: TypeUlid> Default for Handle<T> {
    fn default() -> Self {
        Self {
            path: AssetPath::default(),
            phantom: Default::default(),
        }
    }
}

impl<T: TypeUlid> Handle<T> {
    /// Convert the handle to an [`UntypedHandle`].
    pub fn untyped(self) -> UntypedHandle {
        UntypedHandle { path: self.path }
    }
}

/// An untyped handle to an asset.
///
/// This simply contains the [`AssetPath`] of the asset.
///
/// Can be converted to a typed handle with the [`typed()`][Self::typed] method.
#[derive(Default, Clone, Debug, Hash, PartialEq, Eq)]
pub struct UntypedHandle {
    /// The unique identifier of the asset this handle represents.
    pub path: AssetPath,
}

impl UntypedHandle {
    /// Create a new handle from it's path and label.
    pub fn new<P: Into<PathBuf>>(path: P, label: Option<String>) -> Self {
        UntypedHandle {
            path: AssetPath::new(path, label),
        }
    }

    /// Create a typed [`Handle<T>`] from this [`UntypedHandle`].
    pub fn typed<T: TypeUlid>(self) -> Handle<T> {
        Handle {
            path: self.path,
            phantom: PhantomData,
        }
    }
}

impl<'de> serde::Deserialize<'de> for UntypedHandle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(UntypedHandleVisitor)
    }
}

impl<'de, T: TypeUlid> serde::Deserialize<'de> for Handle<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_str(UntypedHandleVisitor)
            .map(UntypedHandle::typed)
    }
}

struct UntypedHandleVisitor;
impl<'de> serde::de::Visitor<'de> for UntypedHandleVisitor {
    type Value = UntypedHandle;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "A string path to an asset with an optional label."
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let (path, label) = match v.rsplit_once('#') {
            Some((path, label)) => (path, Some(label)),
            None => (v, None),
        };

        Ok(UntypedHandle {
            path: AssetPath::new(path, label.map(String::from)),
        })
    }
}

#[cfg(feature = "has_load_progress")]
mod has_load_progress {
    use bevy_asset::LoadState;
    use bones_has_load_progress::{HasLoadProgress, LoadProgress};
    use type_ulid::TypeUlid;

    impl<T: TypeUlid> HasLoadProgress for super::Handle<T> {
        fn load_progress(
            &self,
            loading_resources: &bones_has_load_progress::LoadingResources,
        ) -> bones_has_load_progress::LoadProgress {
            let bevy_handle = self.get_bevy_handle_untyped();
            let state = loading_resources.asset_server.get_load_state(&bevy_handle);
            let loaded = state == LoadState::Loaded;

            LoadProgress {
                #[allow(clippy::bool_to_int_with_if)]
                loaded: if loaded { 1 } else { 0 },
                total: 1,
            }
        }
    }

    impl HasLoadProgress for super::UntypedHandle {
        fn load_progress(
            &self,
            loading_resources: &bones_has_load_progress::LoadingResources,
        ) -> LoadProgress {
            let bevy_handle = self.get_bevy_handle();
            let state = loading_resources.asset_server.get_load_state(&bevy_handle);
            let loaded = state == LoadState::Loaded;

            LoadProgress {
                #[allow(clippy::bool_to_int_with_if)]
                loaded: if loaded { 1 } else { 0 },
                total: 1,
            }
        }
    }
}

/// Implement bevy conversions when bevy feature is enabled
#[cfg(feature = "bevy")]
mod bevy {
    use bevy_asset::{prelude::*, Asset, AssetPath};
    use bones_bevy_utils::*;
    use bones_ecs::ulid::TypeUlid;

    impl IntoBevy<AssetPath<'static>> for super::AssetPath {
        fn into_bevy(self) -> AssetPath<'static> {
            AssetPath::new(self.path.to_path_buf(), self.label.map(|x| x.to_string()))
        }
    }
    impl<T: Asset + TypeUlid> super::Handle<T> {
        /// Get a Bevy weak [`Handle`] from from this bones asset handle.
        pub fn get_bevy_handle(&self) -> Handle<T> {
            let asset_path = AssetPath::new(
                self.path.path.to_path_buf(),
                self.path.label.as_ref().map(|x| x.to_string()),
            );
            Handle::weak(asset_path.into())
        }
    }

    impl<T: TypeUlid> super::Handle<T> {
        /// Get a Bevy weak [`HandleUntyped`] from this bones asset handle.
        pub fn get_bevy_handle_untyped(&self) -> HandleUntyped {
            let asset_path = AssetPath::new(
                self.path.path.to_path_buf(),
                self.path.label.as_ref().map(|x| x.to_string()),
            );
            HandleUntyped::weak(asset_path.into())
        }
    }
    impl super::UntypedHandle {
        /// Get a Bevy weak [`HandleUntyped`] from this bones asset handle.
        pub fn get_bevy_handle(&self) -> HandleUntyped {
            let asset_path = AssetPath::new(
                self.path.path.to_path_buf(),
                self.path.label.as_ref().map(|x| x.to_string()),
            );
            HandleUntyped::weak(asset_path.into())
        }
    }
}
