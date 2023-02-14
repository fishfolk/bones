use std::marker::PhantomData;

mod stopwatch;
mod time;
mod timer;

pub mod prelude {
    //! The Time Prelude.
    #[doc(hidden)]
    pub use crate::{stopwatch::*, time::*, timer::*};
}
use bevy::prelude::{App, Plugin, Res, ResMut};
use bones_bevy_renderer::{BonesStage, HasBonesWorld};

/// Adds time functionality to Apps.
#[derive(Default)]
pub struct TimePlugin<W: HasBonesWorld> {
    _phantom: PhantomData<W>,
}

impl<W: HasBonesWorld> Plugin for TimePlugin<W> {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(BonesStage::Sync, sync_time::<W>);
    }
}

/// The system that renders the bones world.
fn sync_time<W: HasBonesWorld>(
    world_resource: Option<ResMut<W>>,
    bevy_time: Res<bevy::prelude::Time>,
) {
    let Some(mut world_resource) = world_resource else {
        return;
    };

    let world = world_resource.world();

    // Initialize the time resource if it doesn't exist.
    if world.get_resource::<time::Time>().is_none() {
        world.init_resource::<time::Time>();
    }

    // Use the Bevy time if it's available, otherwise use the default time.
    let time = world.resource::<time::Time>();
    let mut time = time.borrow_mut();

    if let Some(instant) = bevy_time.last_update() {
        time.update_with_instant(instant);
    } else {
        time.update();
    }
}
