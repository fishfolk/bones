use std::borrow::Cow;

use bones_reflect::prelude::*;
use glam::{Vec2, Vec3};

#[derive(HasSchema, Debug, Clone, Default)]
#[repr(C)]
struct DataA {
    x: f32,
    y: f32,
}

#[derive(HasSchema, Debug, Clone, Default)]
#[repr(C)]
struct DataB(f32, f32);

#[derive(HasSchema, Debug, Clone, Default)]
#[repr(C)]
struct DataC {
    a: DataA,
    b: DataB,
}

#[test]
fn cast_glam() {
    // Create a glam vector
    let mut a = Vec2::new(1.2, 3.4);
    // Cast it to custom type with the same layout
    let b: &DataA = a.cast();

    // Make sure the values match after casting
    assert_eq!(a.x, b.x);
    assert_eq!(a.y, b.y);

    // Try it with tuple struct
    let c: &DataB = a.cast();
    assert_eq!(a.x, c.0);
    assert_eq!(a.y, c.1);

    // Do a mutable cast
    let d: &mut DataA = a.cast_mut();
    // Modify the data through casted ref
    d.y = 5.0;

    // Make sure the original vec was modified
    assert_eq!(a.y, 5.0);
}

#[test]
fn ptr_cast() {
    // Let's say we have a store that we need to store asset data in.
    let mut store = Vec::new();

    // First we'll create a glam vec
    let a = Vec2::new(1.2, 3.4);

    // Create a type-erased pointer to the vec
    let ptr = SchemaBox::new(a);

    // And add it to the store
    store.push(ptr);

    // Now we can also add a different type to the store
    let b = Vec3::new(1.2, 3.4, 5.6);
    store.push(SchemaBox::new(b));

    // When we want to get the data back out
    for ptr in &store {
        // We can try to cast the data back to any  type with the same schema.
        if let Ok(data) = ptr.try_cast::<DataA>() {
            assert_eq!(data.x, a.x);
            assert_eq!(data.y, a.y);
        } else if let Ok(data) = ptr.try_cast::<Vec3>() {
            assert_eq!(data.x, b.x);
            assert_eq!(data.y, b.y);
            assert_eq!(data.z, b.z);
        }
    }

    // And we can modify the data, too.
    // Here we use the panicking version of the cast function
    store[1].cast_mut::<Vec3>().x = 7.0;
    assert_eq!(store[1].cast::<Vec3>().x, 7.0);

    // And we can even clone it ( you can only create `SchemaBox` for types that can be cloned ).
    let ptr = store[1].clone();

    assert_eq!(ptr.cast::<Vec3>(), store[1].cast::<Vec3>());
}

#[test]
#[should_panic = "Invalid cast: the schemas of the casted types are not compatible."]
fn cast_not_matching_fails_ref() {
    let a = Vec3::ONE;
    a.cast::<DataA>();
}

#[test]
#[should_panic = "Invalid cast: the schemas of the casted types are not compatible."]
fn cast_not_matching_fails_mut() {
    let mut a = Vec3::ONE;
    a.cast_mut::<DataA>();
}

#[test]
fn ptr_fields() {
    let mut data = DataC {
        a: DataA { x: 1.0, y: 2.0 },
        b: DataB(3.0, 4.0),
    };

    {
        let ptr = SchemaPtr::new(&data);

        let a = ptr.field("a");
        let x = a.field("x").cast::<f32>();
        let y = a.field("y").cast::<f32>();
        let b = ptr.field("b");
        let b0 = b.field(0).cast::<f32>();
        let b1 = b.field(1).cast::<f32>();

        assert_eq!(*x, 1.0);
        assert_eq!(*y, 2.0);
        assert_eq!(*b0, 3.0);
        assert_eq!(*b1, 4.0);
    }

    {
        let mut ptr = SchemaPtrMut::new(&mut data);

        let mut a = ptr.field("a");
        let mut x = a.field("x");
        let x = x.cast_mut::<f32>();
        assert_eq!(*x, 1.0);
        *x *= 3.0;
        let mut y = a.field("y");
        let y = y.cast_mut::<f32>();
        assert_eq!(*y, 2.0);
        *y *= 3.0;

        let mut b = ptr.field("b");
        let mut b0 = b.field(0);
        let b0 = b0.cast_mut::<f32>();
        assert_eq!(*b0, 3.0);
        *b0 *= 3.0;
        let mut b1 = b.field(1);
        let b1 = b1.cast_mut::<f32>();
        assert_eq!(*b1, 4.0);
        *b1 *= 3.0;
    }

    assert_eq!(data.a.x, 3.0);
    assert_eq!(data.a.y, 6.0);
    assert_eq!(data.b.0, 9.0);
    assert_eq!(data.b.1, 12.0);
}

#[test]
fn schema_vec() {
    let mut v = SchemaVec::new(Cow::Borrowed(DataA::schema()));
    assert_eq!(v.len(), 0);
    assert_eq!(v.capacity(), 0);
    v.push(SchemaBox::new(DataA { x: 1.0, y: 2.0 }));
    v.push(SchemaBox::new(DataA { x: 3.0, y: 4.0 }));
    assert_eq!(v.len(), 2);

    let d0 = v.get(0).unwrap().cast::<DataA>();
    assert_eq!(d0.x, 1.0);
    assert_eq!(d0.y, 2.0);
    let d1 = v.get(1).unwrap().cast::<DataA>();
    assert_eq!(d1.x, 3.0);
    assert_eq!(d1.y, 4.0);

    assert_eq!(v.len(), 2);

    let d1 = v.pop().unwrap();
    assert_eq!(d1.cast::<DataA>().x, 3.0);
    let d0 = v.pop().unwrap();
    assert_eq!(d0.cast::<DataA>().x, 1.0);
    assert!(v.pop().is_none());
}
