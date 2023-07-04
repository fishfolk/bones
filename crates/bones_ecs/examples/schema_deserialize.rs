use std::path::Path;

use bones_ecs::prelude::*;

fn main() {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("schema")
        .join("player.schema.yaml");
    let schema_yaml = std::fs::read_to_string(schema_path).unwrap();
    let SchemaFile { schema } = serde_yaml::from_str(&schema_yaml).unwrap();

    dbg!(&schema);
    dbg!(schema.layout());
}
