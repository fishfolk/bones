//! Tests for [`DesyncTree`] build that rely on types in [`bones_ecs`].

use bones_ecs::prelude::*;

#[derive(Clone, HasSchema, Debug, Eq, PartialEq, Default, DesyncHash)]
#[net]
#[repr(C)]
struct Pos(i32, i32);

#[test]
fn desync_tree_entity_names() {
    let world = World::default();

    // Scope this so mut borrows are finished by time tree is being created.
    let (ent1, ent2) = {
        let mut entities = world.resource_mut::<Entities>();
        let mut positions = world.component_mut::<Pos>();
        let mut names = world.component_mut::<Name>();

        let ent1 = entities.create();
        positions.insert(ent1, Pos(0, 0));

        let ent2 = entities.create();
        positions.insert(ent2, Pos(1, 1));
        names.insert(ent2, "entity2".into());
        (ent1, ent2)
    };

    let mut found_ent1_metadata = false;
    let mut found_ent2_metadata = false;

    let desync_tree = world.desync_hash_tree::<fxhash::FxHasher>(false);
    for node in desync_tree.root().dfs_preorder_iter() {
        if let DesyncNodeMetadata::Component { entity_idx } = node.metadata() {
            if *entity_idx == ent1.index() {
                found_ent1_metadata = true;
            } else if *entity_idx == ent2.index() {
                found_ent2_metadata = true;
                assert_eq!(*node.name(), Some("entity2".to_string()));
            }
        }
    }

    assert!(found_ent1_metadata);
    assert!(found_ent2_metadata);
}
