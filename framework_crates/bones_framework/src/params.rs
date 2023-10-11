//! Bones ECS system parameters.

use crate::prelude::*;

use dashmap::mapref::one::MappedRef;
/// Get the root asset of the core asset pack and cast it to type `T`.
pub struct Root<'a, T: HasSchema>(MappedRef<'a, Cid, LoadedAsset, T>);
impl<'a, T: HasSchema> std::ops::Deref for Root<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a, T: HasSchema> SystemParam for Root<'a, T> {
    type State = AssetServer;
    type Param<'s> = Root<'s, T>;

    fn initialize(_world: &mut World) {}
    fn get_state(world: &World) -> Self::State {
        (*world.resources.get::<AssetServer>().unwrap()).clone()
    }
    fn borrow<'s>(_world: &'s World, asset_server: &'s mut Self::State) -> Self::Param<'s> {
        Root(asset_server.root())
    }
}
