#![allow(warnings)]

use bones_schema::prelude::*;

#[derive(HasSchema, Clone, Default)]
#[repr(C)]
struct MyData {
    x: u32,
    y: u32,
    z: u32,
    w: u32,
}

#[derive(HasSchema)]
#[schema(opaque, no_clone, no_default)]
struct MyDataB(f32, f32, f32);

//This is a test.

// Advanced settings.


fn main() {
    let test = MyData::schema();

    dbg!(test.layout());
}
