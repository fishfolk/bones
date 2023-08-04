//! An asset integration between Bevy and bones.
//!
//! Provides an easy way to load metadata for bones games using Bevy assets.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use std::marker::PhantomData;
use std::time::Duration;

use bevy_app::App;
use bevy_asset::{prelude::*, Asset};

/// The prelude.
pub mod prelude {
    pub use crate::*;
    pub use bones_lib::prelude as bones;
    pub use type_ulid::TypeUlid;
}

use bones_bevy_utils::BevyWorld;
use bones_lib::prelude::Color;
use prelude::*;

pub use bones_bevy_asset_macros::{BonesBevyAsset, BonesBevyAssetLoad};

#[doc(hidden)]
pub mod _private {
    pub use serde_json;
    pub use serde_yaml;
}

/// Trait that may be derived to implement a Bevy asset type.
// TODO: Integrate or move `HasLoadProgress` to `BonesBevyAsset`.
pub trait BonesBevyAsset: TypeUlid + Asset {
    /// Install the asset loader for this type.
    fn install_asset(app: &mut App);
}

/// Extension trait for [`App`] that makes it easy to register bones assets.
pub trait BonesBevyAssetAppExt {
    /// Adds a [`BonesBevyAsset`] to the app, including it's asset loader.
    fn add_bones_asset<T: BonesBevyAsset>(&mut self) -> &mut Self;
}

impl BonesBevyAssetAppExt for App {
    fn add_bones_asset<T: BonesBevyAsset>(&mut self) -> &mut Self {
        T::install_asset(self);

        self
    }
}

/// Trait implemented for types that may appear in the fields of a [`BonesBevyAsset`] and may need
/// to perform aditional loading with the bevy load context.
pub trait BonesBevyAssetLoad {
    /// Allows the field to do any extra loading that it might need to do from the Bevy load context
    /// when the asset is loaded.
    fn load(
        &mut self,
        load_context: &mut bevy_asset::LoadContext,
        dependencies: &mut Vec<bevy_asset::AssetPath<'static>>,
    ) {
        let _ = (load_context, dependencies);
    }
}

impl BonesBevyAssetLoad for Duration {}

impl BonesBevyAssetLoad for Color {}

impl<T: TypeUlid> BonesBevyAssetLoad for bones::Handle<T> {
    fn load(
        &mut self,
        load_context: &mut bevy_asset::LoadContext,
        dependencies: &mut Vec<bevy_asset::AssetPath<'static>>,
    ) {
        // Convert this path to a path relative to the parent asset
        self.path.normalize_relative_to(load_context.path());

        // Create a bevy asset path from this bones handle
        let asset_path = self.path.get_bevy_asset_path();
        let path_id = asset_path.get_id();
        dependencies.push(asset_path);

        // Load the asset
        let handle = load_context.get_handle::<_, DummyAsset>(path_id);

        // Leak the strong handle so that the asset doesn't get unloaded
        std::mem::forget(handle);
    }
}

impl<T: BonesBevyAssetLoad> BonesBevyAssetLoad for Vec<T> {
    fn load(
        &mut self,
        load_context: &mut bevy_asset::LoadContext,
        dependencies: &mut Vec<bevy_asset::AssetPath<'static>>,
    ) {
        self.iter_mut()
            .for_each(|x| x.load(load_context, dependencies))
    }
}

impl<T: BonesBevyAssetLoad> BonesBevyAssetLoad for Option<T> {
    fn load(
        &mut self,
        load_context: &mut bevy_asset::LoadContext,
        dependencies: &mut Vec<bevy_asset::AssetPath<'static>>,
    ) {
        if let Some(x) = self.as_mut() {
            x.load(load_context, dependencies)
        }
    }
}

impl<K, H, T: BonesBevyAssetLoad> BonesBevyAssetLoad for std::collections::HashMap<K, T, H> {
    fn load(
        &mut self,
        load_context: &mut bevy_asset::LoadContext,
        dependencies: &mut Vec<bevy_asset::AssetPath<'static>>,
    ) {
        self.iter_mut()
            .for_each(|(_k, v)| v.load(load_context, dependencies))
    }
}

impl<K, V: BonesBevyAssetLoad> BonesBevyAssetLoad for bevy_utils::HashMap<K, V> {
    fn load(
        &mut self,
        load_context: &mut bevy_asset::LoadContext,
        dependencies: &mut Vec<bevy_asset::AssetPath<'static>>,
    ) {
        self.iter_mut()
            .for_each(|(_k, v)| v.load(load_context, dependencies))
    }
}

/// Helper make empty load implementations for a list of types.
macro_rules! impl_default_traits {
    ( $($type:ty),* $(,)? ) => {
        $(
            impl BonesBevyAssetLoad for $type {}
        )*
    };
}

// Implement for types that don't need special loading behavior.
impl_default_traits!(
    String,
    f32,
    f64,
    usize,
    u8,
    u16,
    u32,
    u64,
    u128,
    i8,
    i16,
    i32,
    i64,
    i128,
    glam::Vec2,
    glam::Vec3,
    glam::UVec2,
    bool,
    bones_lib::prelude::Key
);

/// Bones [`SystemParam`][bones_lib::ecs::system::SystemParam] for borrowing bevy
/// [`Assets`][bevy_asset::Assets] from the [`BevyWorld`] resource.
pub struct BevyAssets<'a, T: bevy_asset::Asset> {
    cell: bones::AtomicRef<'a, BevyWorld>,
    _phantom: PhantomData<T>,
}
impl<'a, T: bevy_asset::Asset> std::ops::Deref for BevyAssets<'a, T> {
    type Target = bevy_asset::Assets<T>;
    fn deref(&self) -> &Self::Target {
        self.cell
            .as_ref()
            .expect("Bevy world not present in `BevyWorld` resource.")
            .resource::<Assets<T>>()
    }
}

impl<'a, T: bevy_asset::Asset> bones_lib::ecs::system::SystemParam for BevyAssets<'a, T> {
    type State = bones::AtomicResource<BevyWorld>;
    type Param<'s> = BevyAssets<'s, T>;

    fn initialize(_world: &mut bones::World) {}

    fn get_state(world: &bones::World) -> Self::State {
        world.resource::<BevyWorld>()
    }

    fn borrow(state: &mut Self::State) -> Self::Param<'_> {
        BevyAssets {
            cell: state.borrow(),
            _phantom: PhantomData,
        }
    }
}

/// Dummy asset needed as a type parameter for the `load_context.get_handle` method that doesn't
/// have an untyped equivalent.
#[derive(bevy_reflect::TypeUuid)]
#[uuid = "ece514f7-4ffe-4251-9c25-d568acd696eb"]
struct DummyAsset;
