//! `EcsWorld` — facade over [`specs::World`] (S5.1 — issue #100).
//!
//! The wrapper keeps the `specs` dependency behind our crate wall so that
//! callers in `beast-sim` and above never import `specs` directly. See
//! `documentation/architecture/CRATE_LAYOUT.md` §Layer 3.
//!
//! Only the methods every S5 story needs are exposed up front; later
//! stories (5.3 systems, 5.4 resources, 5.5 sorted index) add methods on
//! top of this surface rather than replacing it.

use specs::{World, WorldExt};

/// The simulation world: entities, components, and (eventually) the
/// [`crate::resources::Resources`] struct attached via `specs`'s fetch
/// mechanism.
///
/// Construct with [`EcsWorld::new`]. The wrapper owns the inner
/// [`specs::World`]; downstream crates hold `&EcsWorld` or `&mut EcsWorld`
/// references instead of touching `specs` directly.
///
/// # Determinism
///
/// `EcsWorld` itself does not iterate. Systems that read or write
/// components must iterate through the sorted entity index introduced in
/// S5.5 to keep `INVARIANTS §1` (sorted iteration in hot loops) satisfied.
pub struct EcsWorld {
    inner: World,
}

impl Default for EcsWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl EcsWorld {
    /// Create an empty world with no registered components and no
    /// resources. Components are registered lazily via
    /// [`Self::register_component`] as systems are added.
    ///
    /// Internally delegates to [`specs::World::new`] (via
    /// [`specs::WorldExt`]), which seeds the ECS meta-tables `specs`
    /// relies on — plain `World::default()` from `shred` skips that setup
    /// and panics on first storage access.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: World::new(),
        }
    }

    /// Register a component type on the inner `specs::World`. Must be
    /// called once per component type before any entity is built with
    /// that component.
    ///
    /// This is a thin pass-through to [`specs::World::register`]; the only
    /// reason it exists on the wrapper is to keep `specs` imports out of
    /// caller code.
    pub fn register_component<C>(&mut self)
    where
        C: specs::Component,
        C::Storage: Default,
    {
        self.inner.register::<C>();
    }

    /// Start building a new entity. Chain `.with(...)` calls to attach
    /// components, then finish with `.build()` — see
    /// [`specs::EntityBuilder`] for the full builder API.
    pub fn create_entity(&mut self) -> specs::EntityBuilder<'_> {
        self.inner.create_entity()
    }

    /// Borrow the inner [`specs::World`] — **escape hatch only**.
    ///
    /// Exposes `specs::World` through this facade so callers can reach
    /// APIs we have not yet wrapped (e.g., direct `read_storage` /
    /// `write_storage` access). The return type names `specs::World`,
    /// which technically leaks the backend choice into the public API
    /// surface; in practice callers use type inference plus the
    /// [`specs`] re-exports from this crate (`Builder`, `WorldExt`,
    /// `ReadStorage`, etc.) so they do **not** need a direct `specs`
    /// dependency in their own `Cargo.toml`.
    ///
    /// Prefer dedicated methods on `EcsWorld` when a wrapper exists.
    /// Every use of this escape hatch is a candidate for future
    /// wrapping; audit with `rg 'world\(\)\.world\(\)'` before
    /// refactoring the backend.
    #[must_use]
    pub fn world(&self) -> &World {
        &self.inner
    }

    /// Mutable counterpart to [`Self::world`]. Same escape-hatch
    /// semantics — see the immutable version for guidance.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specs::{Builder, Component, DenseVecStorage, Join, WorldExt};

    /// A minimal component for smoke tests. Kept out of the public API —
    /// real components live in [`crate::components`] (S5.2).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestPosition {
        x: i32,
        y: i32,
    }

    impl Component for TestPosition {
        type Storage = DenseVecStorage<Self>;
    }

    #[test]
    fn new_world_has_no_entities() {
        let world = EcsWorld::new();
        let entities = world.world().entities();
        // `specs` gives us the entities resource but zero live ones.
        assert_eq!(entities.join().count(), 0);
    }

    #[test]
    fn register_and_create_entity_with_component() {
        let mut world = EcsWorld::new();
        world.register_component::<TestPosition>();

        let _entity = world
            .create_entity()
            .with(TestPosition { x: 3, y: -4 })
            .build();

        // Read back through the inner World's storage.
        let storage = world.world().read_storage::<TestPosition>();
        let values: Vec<TestPosition> = storage.join().copied().collect();
        assert_eq!(values, vec![TestPosition { x: 3, y: -4 }]);
    }

    #[test]
    fn default_and_new_are_equivalent() {
        // Default is derived; just ensure both paths compile and produce
        // a world we can actually put an entity into.
        let mut a = EcsWorld::new();
        let mut b = EcsWorld::default();
        a.register_component::<TestPosition>();
        b.register_component::<TestPosition>();
        let _ = a.create_entity().with(TestPosition { x: 0, y: 0 }).build();
        let _ = b.create_entity().with(TestPosition { x: 0, y: 0 }).build();
    }

    #[test]
    fn world_accessor_exposes_inner_for_storage_reads() {
        let mut world = EcsWorld::new();
        world.register_component::<TestPosition>();
        let _ = world
            .create_entity()
            .with(TestPosition { x: 1, y: 2 })
            .build();
        let _ = world
            .create_entity()
            .with(TestPosition { x: 5, y: 6 })
            .build();

        let count = world.world().read_storage::<TestPosition>().join().count();
        assert_eq!(count, 2);
    }
}
