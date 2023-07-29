#![allow(warnings)]

use std::sync::{Arc, Mutex};

use bones_schema::prelude::*;
use glam::UVec2;

// The `Clone` and `Default` implementations are required by default.
#[derive(HasSchema, Clone, Default)]
#[repr(C)]
struct TupleExample(f32, f32, f32);

// But you can still opt out of having a clone or default implementation.
//
// Clone and Default are needed for things like assets and components in bones,
// so not everything will work without them.
#[derive(HasSchema)]
#[schema(no_clone, no_default)]
#[repr(C)]
struct Player {
    name: String,
    age: u32,
    height: u32,
    tile_size: UVec2,
    /// Opaque means scripts won't know how to interact with it, because it can't be described by a
    /// [`Schema`].
    #[schema(opaque)]
    arc_mutex: Arc<Mutex<()>>,
    favorite_things: Vec<String>,
    fancier: Vec<Vec<TupleExample>>,
}

/// You can also make the entire type opaque, so that it contains no real description of it's type
/// other than the size and alignment.
#[derive(HasSchema)]
#[schema(opaque, no_clone, no_default)]
struct OpaqueType {
    data: u32,
}

fn main() {
    dbg!(TupleExample::schema());
    dbg!(Player::schema());
    dbg!(Player::schema().layout());
    dbg!(OpaqueType::schema());
}
