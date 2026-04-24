//! Sorted entity index total-order tests (S5.7 — issue #112).

use beast_ecs::{Builder, EcsWorld, MarkerKind, SortedEntityIndex};
use specs::Entity;

#[test]
fn iter_all_gives_marker_then_entity_total_order() {
    // 50 creatures + 50 pathogens, inserted in interleaved order.
    let mut world = EcsWorld::new();
    let entities: Vec<Entity> = (0..100).map(|_| world.create_entity().build()).collect();

    let mut index = SortedEntityIndex::new();
    // Interleave Creature/Pathogen insertions so sort-ness is
    // unmistakably doing work.
    for (i, entity) in entities.iter().enumerate() {
        let marker = if i % 2 == 0 {
            MarkerKind::Creature
        } else {
            MarkerKind::Pathogen
        };
        index.insert(*entity, marker);
    }

    let all: Vec<(MarkerKind, Entity)> = index.iter_all().collect();

    // 1) Length: every entity appears exactly once.
    assert_eq!(all.len(), entities.len());

    // 2) Ordering: MarkerKind ascending (Creature < Pathogen), then
    //    Entity ascending within each group.
    for pair in all.windows(2) {
        let (ka, ea) = pair[0];
        let (kb, eb) = pair[1];
        assert!(
            ka < kb || (ka == kb && ea < eb),
            "iter_all total order broken: {:?} then {:?}",
            pair[0],
            pair[1]
        );
    }

    // 3) Creature group comes first, exactly 50 entries.
    let creature_count = all
        .iter()
        .take_while(|(k, _)| *k == MarkerKind::Creature)
        .count();
    assert_eq!(creature_count, 50);
    let pathogen_count = all.len() - creature_count;
    assert_eq!(pathogen_count, 50);
}

#[test]
fn iteration_order_is_stable_across_repeated_calls() {
    let mut world = EcsWorld::new();
    let entities: Vec<Entity> = (0..20).map(|_| world.create_entity().build()).collect();

    let mut index = SortedEntityIndex::new();
    // Insert 10 creatures + 10 agents out of order.
    for i in [17, 3, 11, 5, 1, 9, 15, 7, 13, 19] {
        index.insert(entities[i], MarkerKind::Creature);
    }
    for i in [8, 0, 4, 12, 2, 6, 16, 10, 14, 18] {
        index.insert(entities[i], MarkerKind::Agent);
    }

    let a: Vec<(MarkerKind, Entity)> = index.iter_all().collect();
    let b: Vec<(MarkerKind, Entity)> = index.iter_all().collect();
    let c: Vec<(MarkerKind, Entity)> = index.iter_all().collect();
    assert_eq!(a, b);
    assert_eq!(b, c);
}
