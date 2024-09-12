use std::hash::Hasher;

use fxhash::FxHasher;
use glam::Vec3;

use bones_schema::prelude::*;
use bones_utils::{net, DesyncHash};

#[derive(HasSchema, DesyncHash, Debug, Clone, Default)]
#[desync_hash_module(crate)]
#[net]
struct StructA {
    a: f32,
    b: String,
}

#[derive(HasSchema, DesyncHash, Debug, Clone, Default)]
#[desync_hash_module(crate)]
struct StructB {
    a: f32,
    b: String,
}

/// Test DesyncHash proc macro on Enum variants
#[derive(HasSchema, DesyncHash, Debug, Clone, Default)]
#[repr(C, u8)]
#[desync_hash_module(crate)]
#[allow(dead_code)]
enum EnumA {
    #[default]
    A,
    B,
    C(),
    D(f32, u8),
    E {
        a: f64,
        b: u16,
    },
    F = 52,
}

fn hash_value<T: DesyncHash>(value: &T) -> u64 {
    let mut hasher = FxHasher::default();
    DesyncHash::hash(value, &mut hasher);
    hasher.finish()
}

#[test]
fn desync_hash_enum() {
    let a = EnumA::A;
    let b = EnumA::B;

    // ensure enum variants do not hash to same value
    assert_ne!(hash_value(&a), hash_value(&b));

    // verify mutating field of tuple variant gives different hash
    let d1 = EnumA::D(16.0, 3);
    let d2 = EnumA::D(16.0, 2);
    assert_ne!(hash_value(&d1), hash_value(&d2));

    // verify mutating field of named struct variant gives different hash
    let e1 = EnumA::E { a: 1.0, b: 2 };
    let e2 = EnumA::E { a: 1.0, b: 1 };
    assert_ne!(hash_value(&e1), hash_value(&e2));
}

#[test]
fn desync_hash_struct() {
    let a = StructA {
        a: 1.0,
        b: "foo".to_string(),
    };
    let b = StructA {
        a: 1.0,
        b: "bar".to_string(),
    };

    assert_ne!(hash_value(&a), hash_value(&b));
}

#[test]
fn desync_hash_glam() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(1.0, 1.0, 1.0);

    assert_ne!(hash_value(&a), hash_value(&b));
}

#[test]
fn desync_hash_schemaref() {
    // Test that these hash to different values, StructA
    // has SchemaDesyncHash typedata.
    let a = StructA {
        a: 1.0,
        b: "foo".to_string(),
    };
    let b = StructA {
        a: 1.0,
        b: "bar".to_string(),
    };
    let a_hash = hash_value(&a.as_schema_ref());
    let b_hash = hash_value(&b.as_schema_ref());
    assert_ne!(a_hash, b_hash);

    // StructB does not have SchemaDesyncHash typedata,
    // its SchemaRef does not have impl for DesyncHash,
    // even if data is different, should just get 0.
    let a = StructB {
        a: 1.0,
        b: "foo".to_string(),
    };
    let b = StructB {
        a: 1.0,
        b: "bar".to_string(),
    };
    let a_hash = hash_value(&a.as_schema_ref());
    let b_hash = hash_value(&b.as_schema_ref());
    // Test that these hash to differnet values, StructA
    // has SchemaDesyncHash typedata.
    assert_eq!(a_hash, b_hash);
    assert_eq!(a_hash, 0);
}
