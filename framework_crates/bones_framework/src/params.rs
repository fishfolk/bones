//! Bones ECS system parameters.

use crate::prelude::*;

/// Get the root asset of the core asset pack and cast it to type `T`.
pub struct Root<'a, T: HasSchema>(Ref<'a, T>);
impl<'a, T: HasSchema> std::ops::Deref for Root<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a, T: HasSchema> SystemParam for Root<'a, T> {
    type State = AtomicResource<AssetServer>;
    type Param<'s> = Root<'s, T>;

    fn initialize(_world: &mut World) {}
    fn get_state(world: &World) -> Self::State {
        world.resources.get_cell::<AssetServer>().unwrap()
    }
    fn borrow(state: &mut Self::State) -> Self::Param<'_> {
        Root(Ref::map(state.borrow(), |asset_server| asset_server.root()))
    }
}
