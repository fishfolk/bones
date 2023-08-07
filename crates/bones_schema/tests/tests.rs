use std::alloc::Layout;

use bones_schema::prelude::*;
use glam::{Vec2, Vec3};

#[derive(HasSchema, Debug, Clone, Default)]
#[repr(C)]
struct DataA {
    x: f32,
    y: f32,
}

#[derive(HasSchema, Debug, Clone, Default, PartialEq)]
#[repr(C)]
struct DataB(f32, f32);

#[derive(HasSchema, Debug, Clone, Default)]
#[repr(C)]
struct DataC {
    a: DataA,
    b: DataB,
}

#[derive(HasSchema, Debug, Clone, Default)]
#[repr(C)]
struct Zst;

#[derive(HasSchema, Debug, Clone, Default)]
#[schema(opaque)]
struct OpaqueZst;

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
        if let Ok(data) = ptr.try_cast_ref::<DataA>() {
            assert_eq!(data.x, a.x);
            assert_eq!(data.y, a.y);
        } else if let Ok(data) = ptr.try_cast_ref::<Vec3>() {
            assert_eq!(data.x, b.x);
            assert_eq!(data.y, b.y);
            assert_eq!(data.z, b.z);
        }
    }

    // And we can modify the data, too.
    // Here we use the panicking version of the cast function
    store[1].cast_mut::<Vec3>().x = 7.0;
    assert_eq!(store[1].cast_ref::<Vec3>().x, 7.0);

    // And we can even clone it ( cloning will panic if the schema doesn't implement cloning ).
    let ptr = store[1].clone();

    assert_eq!(ptr.cast_ref::<Vec3>(), store[1].cast_ref::<Vec3>());

    // Finally, we can conver the box to the inner type, if we know what it is. This will panic if
    // the schema doesn't match.
    let ptr = SchemaBox::new(String::from("hello"));
    let inner = ptr.into_inner::<String>();
    assert_eq!(inner, "hello");
}

#[test]
fn sbox() {
    let mut b = SBox::new(String::from("hello"));
    assert_eq!(*b, "hello");
    b.push('!');
    assert_eq!(*b, "hello!");
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
        let ptr = SchemaRef::new(&data);

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
        let mut ptr = SchemaRefMut::new(&mut data);

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
    let mut v = SchemaVec::new(DataA::schema());
    assert_eq!(v.len(), 0);
    assert_eq!(v.capacity(), 0);
    v.push(DataA { x: 1.0, y: 2.0 });
    v.push_box(SchemaBox::new(DataA { x: 3.0, y: 4.0 }));
    assert_eq!(v.len(), 2);

    let d0 = v.get::<DataA>(0).unwrap();
    assert_eq!(d0.x, 1.0);
    assert_eq!(d0.y, 2.0);
    let d1 = v.get_ref(1).unwrap().cast::<DataA>();
    assert_eq!(d1.x, 3.0);
    assert_eq!(d1.y, 4.0);

    assert_eq!(v.len(), 2);

    let d1 = v.pop_box().unwrap();
    assert_eq!(d1.cast_ref::<DataA>().x, 3.0);
    let d0 = v.pop::<DataA>().unwrap();
    assert_eq!(d0.x, 1.0);
    assert!(v.pop_box().is_none());
}

#[test]
fn svec() {
    let mut v = SVec::new();
    for i in 0..10 {
        v.push(i);
    }

    let mut i = 0;
    #[allow(clippy::explicit_counter_loop)]
    for n in &v {
        assert_eq!(i, *n);
        i += 1;
    }

    for n in &mut v {
        *n *= 2;
    }

    let mut i = 0;
    #[allow(clippy::explicit_counter_loop)]
    for n in &v {
        assert_eq!(i * 2, *n);
        i += 1;
    }
}

#[test]
fn schema_map() {
    let k1 = String::from("hello");
    let k2 = String::from("goodbye");
    let mut m = SchemaMap::new(String::schema(), DataB::schema());
    let previous = m.insert(k1.clone(), DataB(1.0, 2.0));
    assert!(previous.is_none());
    m.insert(k2.clone(), DataB(3.0, 4.0));

    {
        let v1: &mut DataB = m.get_mut(&k1).unwrap();
        assert_eq!(v1.1, 2.0);
        v1.0 = 7.0;
    }
    {
        let v2: &mut DataB = m.get_mut(&k2).unwrap();
        assert_eq!(v2.0, 3.0);
        assert_eq!(v2.1, 4.0);
    }

    let v1: &mut DataB = m.get_mut(&k1).unwrap();
    assert_eq!(v1.0, 7.0);

    let previous = m.insert(k1.clone(), DataB(10., 11.));
    assert_eq!(previous, Some(DataB(7.0, 2.0)))
}

#[test]
fn eq_hash() {
    let b1 = SchemaBox::new(String::from("hello"));
    let b2 = SchemaBox::new(String::from("hello"));
    let b3 = SchemaBox::new(String::from("goodbye"));
    assert_eq!(b1, b2);
    assert_ne!(b3, b2);
    assert_ne!(b3, b1);
    assert_eq!(dbg!(b1.hash()), b2.hash());
    assert_ne!(dbg!(b3.hash()), b1.hash());

    let s_hash_fn = b1.schema().hash_fn.unwrap();
    assert_eq!(unsafe { (s_hash_fn)(b1.as_ref().as_ptr()) }, b1.hash());
}

#[test]
fn zst() {
    let b = SchemaBox::new(Zst);
    assert!(matches!(b.cast_ref(), Zst));
    let b = SchemaBox::new(OpaqueZst);
    assert!(matches!(b.cast_ref(), OpaqueZst));
}

#[test]
fn schema_layout_matches_rust_layout() {
    #[derive(HasSchema, Default, Clone)]
    #[repr(C)]
    struct A;

    #[derive(HasSchema, Default, Clone)]
    #[repr(C)]
    struct B {
        a: f32,
        b: u8,
        c: String,
        d: SVec<Vec2>,
    }

    macro_rules! layout_eq {
        ( $( $t:ident ),* ) => {
            $(
                assert_eq!($t::schema().layout(), Layout::new::<$t>());
            )*
        };
    }
    layout_eq!(A, B);
}
