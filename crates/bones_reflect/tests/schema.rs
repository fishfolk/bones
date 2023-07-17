use bones_reflect::schema::{HasSchema, SchemaBox, SchemaWalkerMut};
use bones_reflect_macros::HasSchema;
use bones_utils::PtrMut;
use glam::{Vec2, Vec3};

#[derive(HasSchema, Debug)]
#[repr(C)]
struct DataA {
    x: f32,
    y: f32,
}

#[derive(HasSchema, Debug)]
#[repr(C)]
struct DataB(f32, f32);

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
fn double_borrow() {
    unsafe {
        let schema = DataA::schema();
        let mut data = DataA { x: 3.0, y: 4.0 };
        let ptr = PtrMut::from(&mut data);
        let mut walker: SchemaWalkerMut<'_, '_, 'static> =
            SchemaWalkerMut::from_ptr_schema(ptr, schema);

        let mut field: SchemaWalkerMut<'_, '_, '_> = walker.get_field("x").unwrap();

        walker.get_field("y").unwrap();

        // let f2 = &mut field;

        // dbg!(f2);
        todo!("Implement proper test.");
    }
}
