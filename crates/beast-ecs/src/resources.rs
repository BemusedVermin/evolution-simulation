//! `Resources` — registries, PRNG streams, tick counter (S5.4 — issue #104).
//!
//! Every system receives `&mut Resources` on each tick. This struct is
//! the single owner of the channel + primitive registries and the
//! per-subsystem PRNG streams derived from the master `world_seed`.
//!
//! # Determinism contract (INVARIANTS §1)
//!
//! * The master PRNG is created from `world_seed` at construction time;
//!   it is not stored — only its derived children are kept. Re-running
//!   `Resources::new(world_seed, ...)` with the same inputs produces a
//!   byte-identical `Resources` value.
//! * Each subsystem has **one** `Prng` stream, derived via
//!   [`beast_core::prng::Prng::split_stream`]. Systems never cross-borrow
//!   each other's streams; mutation belongs to the owning stage alone.
//! * `tick_counter` starts at [`TickCounter::ZERO`] and is advanced
//!   exactly once per tick by the scheduler (S6).

use beast_channels::ChannelRegistry;
use beast_core::prng::{Prng, Stream};
use beast_core::TickCounter;
use beast_primitives::PrimitiveRegistry;

use crate::entity_id::SortedEntityIndex;

/// Global per-tick state held outside the ECS world: registries, PRNG
/// streams, and the tick counter.
///
/// Cheap to move (registries are wrapped in BTreeMaps, PRNGs are 32-byte
/// state), but `Resources` does not implement `Clone` on purpose —
/// duplicating PRNG state is almost always a bug (both copies would draw
/// the same sequence, breaking the one-stream-per-subsystem rule).
#[derive(Debug)]
pub struct Resources {
    /// Current simulation tick. Advanced by the scheduler at the end of
    /// each tick. Starts at [`TickCounter::ZERO`].
    pub tick_counter: TickCounter,
    /// The seed used to derive the master PRNG. Stored so the save/load
    /// code (S7) can reconstruct an identical `Resources` from a save
    /// file plus the replay journal.
    pub world_seed: u64,
    /// Channel manifest registry (core + mods + genesis). Mutable so the
    /// `ChannelGenesisSystem` can install new genesis-born channels.
    pub channels: ChannelRegistry,
    /// Primitive manifest registry. Immutable after world init today —
    /// primitives come from shipped vocabulary; mod support lands later.
    pub primitives: PrimitiveRegistry,

    /// Genetic mutation operators — point mutation, duplication, etc.
    pub rng_genetics: Prng,
    /// Phenotype interpreter stochastic tie-breaks.
    pub rng_phenotype: Prng,
    /// Physics & movement jitter.
    pub rng_physics: Prng,
    /// Combat & interaction resolution.
    pub rng_combat: Prng,
    /// Metabolism, ageing, healing.
    pub rng_physiology: Prng,
    /// Ecology: random events (migrations, plagues), spawning, biome
    /// dynamics.
    pub rng_ecology: Prng,
    /// World generation — terrain, biome placement. One-shot at world
    /// init, but kept live so the `Resources` remains a single home for
    /// PRNG state.
    pub rng_worldgen: Prng,
    /// Chronicler pattern clustering and sampling.
    pub rng_chronicler: Prng,

    /// Deterministic per-marker entity index (S5.5). Systems iterate
    /// through this rather than `specs::Join` so entity order is a
    /// documented contract, not a `specs` implementation detail.
    pub entity_index: SortedEntityIndex,
}

impl Resources {
    /// Build a fresh `Resources` for a new world.
    ///
    /// Derives one PRNG stream per subsystem from `world_seed` using
    /// [`Prng::split_stream`]. Streams do not overlap for `2^192` draws
    /// each (see the `split_stream` docs), so cross-contamination between
    /// subsystems is effectively impossible within any realistic
    /// simulation lifetime.
    ///
    /// # Example
    ///
    /// ```
    /// use beast_channels::ChannelRegistry;
    /// use beast_primitives::PrimitiveRegistry;
    /// use beast_ecs::Resources;
    ///
    /// let resources = Resources::new(
    ///     0xDEAD_BEEF,
    ///     ChannelRegistry::new(),
    ///     PrimitiveRegistry::new(),
    /// );
    /// assert_eq!(resources.world_seed, 0xDEAD_BEEF);
    /// assert_eq!(resources.tick_counter.raw(), 0);
    /// ```
    #[must_use]
    pub fn new(world_seed: u64, channels: ChannelRegistry, primitives: PrimitiveRegistry) -> Self {
        let master = Prng::from_seed(world_seed);
        Self {
            tick_counter: TickCounter::ZERO,
            world_seed,
            channels,
            primitives,
            rng_genetics: master.split_stream(Stream::Genetics),
            rng_phenotype: master.split_stream(Stream::Phenotype),
            rng_physics: master.split_stream(Stream::Physics),
            rng_combat: master.split_stream(Stream::Combat),
            rng_physiology: master.split_stream(Stream::Physiology),
            rng_ecology: master.split_stream(Stream::Ecology),
            rng_worldgen: master.split_stream(Stream::Worldgen),
            rng_chronicler: master.split_stream(Stream::Chronicler),
            entity_index: SortedEntityIndex::new(),
        }
    }

    /// Advance the tick counter by one. Called by the S6 scheduler after
    /// every stage has run for the current tick.
    ///
    /// Saturates at [`TickCounter::MAX`] (`u64::MAX`) rather than
    /// wrapping — at 60 ticks/s the ceiling is ~9.7 billion years, so
    /// in practice saturation indicates a logic bug rather than a
    /// legitimate runtime event. See `beast_core::time::TickCounter`.
    pub fn advance_tick(&mut self) {
        self.tick_counter.advance();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Resources {
        Resources::new(42, ChannelRegistry::new(), PrimitiveRegistry::new())
    }

    #[test]
    fn new_starts_at_tick_zero() {
        let r = fresh();
        assert_eq!(r.tick_counter.raw(), 0);
        assert_eq!(r.world_seed, 42);
    }

    #[test]
    fn same_seed_produces_identical_prng_draws() {
        // Determinism contract: Resources::new is a pure function of its
        // arguments. Any two Resources built with the same seed must
        // produce the same first N draws on every stream (all 8).
        let mut a = fresh();
        let mut b = fresh();
        for _ in 0..64 {
            assert_eq!(a.rng_genetics.next_u64(), b.rng_genetics.next_u64());
            assert_eq!(a.rng_phenotype.next_u64(), b.rng_phenotype.next_u64());
            assert_eq!(a.rng_physics.next_u64(), b.rng_physics.next_u64());
            assert_eq!(a.rng_combat.next_u64(), b.rng_combat.next_u64());
            assert_eq!(a.rng_physiology.next_u64(), b.rng_physiology.next_u64());
            assert_eq!(a.rng_ecology.next_u64(), b.rng_ecology.next_u64());
            assert_eq!(a.rng_worldgen.next_u64(), b.rng_worldgen.next_u64());
            assert_eq!(a.rng_chronicler.next_u64(), b.rng_chronicler.next_u64());
        }
    }

    #[test]
    fn streams_are_independent_across_subsystems() {
        // Each subsystem should see a distinct first draw — otherwise the
        // split_stream invariant (no stream overlap) is broken.
        let mut r = fresh();
        let genetics = r.rng_genetics.next_u64();
        let phenotype = r.rng_phenotype.next_u64();
        let physics = r.rng_physics.next_u64();
        let combat = r.rng_combat.next_u64();
        let physiology = r.rng_physiology.next_u64();
        let ecology = r.rng_ecology.next_u64();
        let worldgen = r.rng_worldgen.next_u64();
        let chronicler = r.rng_chronicler.next_u64();

        let draws = [
            genetics, phenotype, physics, combat, physiology, ecology, worldgen, chronicler,
        ];
        let mut sorted: Vec<u64> = draws.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            draws.len(),
            "two subsystems drew the same first u64 — split_stream is broken"
        );
    }

    #[test]
    fn advance_tick_increments_counter() {
        let mut r = fresh();
        for expected in 1..=10 {
            r.advance_tick();
            assert_eq!(r.tick_counter.raw(), expected);
        }
    }

    #[test]
    fn different_seeds_diverge_per_stream() {
        let mut a = Resources::new(1, ChannelRegistry::new(), PrimitiveRegistry::new());
        let mut b = Resources::new(2, ChannelRegistry::new(), PrimitiveRegistry::new());
        // Overwhelmingly likely: find divergence in the first few draws.
        let mut differed = false;
        for _ in 0..16 {
            if a.rng_genetics.next_u64() != b.rng_genetics.next_u64() {
                differed = true;
                break;
            }
        }
        assert!(differed, "same stream under different seeds should diverge");
    }
}
