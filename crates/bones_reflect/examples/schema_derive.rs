use std::sync::{Arc, Mutex};

use bones_reflect::prelude::*;
use glam::UVec2;

#[derive(HasSchema)]
#[repr(C)]
struct TupleExample(f32, f32, f32);

#[derive(HasSchema)]
#[repr(C)]
struct Player {
    name: String,
    age: u32,
    height: u32,
    tile_size: UVec2,
    // Opaque means scripts won't know how to interact with it, because it can't be described by a
    // [`Schema`].
    #[schema(opaque)]
    arc_mutex: Arc<Mutex<()>>,
    favorite_things: Vec<String>,
    fancier: Vec<Vec<TupleExample>>,
}

fn main() {
    dbg!(TupleExample::schema());
    dbg!(Player::schema());
    dbg!(Player::schema().layout());
}
