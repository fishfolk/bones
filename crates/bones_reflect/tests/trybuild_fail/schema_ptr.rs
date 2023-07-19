use bones_reflect::prelude::*;

#[derive(HasSchema, Clone, Default)]
#[repr(C)]
struct DataA {
    x: f32,
    y: f32,
}

#[derive(HasSchema, Clone, Default)]
#[repr(C)]
struct DataB(u32, u32);

#[derive(HasSchema, Clone, Default)]
#[repr(C)]
struct DataC {
    a: DataA,
    b: DataB,
}

fn main() {
    let mut data = DataC {
        a: DataA { x: 1.0, y: 2.0 },
        b: DataB(3, 4),
    };
    let mut ptr = SchemaPtrMut::new(&mut data);

    let mut a = ptr.field("a");
    let mut x = a.field("x");
    let x = x.cast_mut::<f32>();
    *x = 1.5;
    let mut y = a.field("y");
    let y = y.cast_mut::<f32>();
    *y = 2.5;

    // Important no-no, can't borrow x while y is borrowed.
    dbg!(x);
}
