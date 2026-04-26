//! Entity identifiers used throughout the ECS.
//!
//! `EntityId` is a transparent `u32` newtype. 32 bits gives us ~4.3 billion
//! possible IDs, far more than any plausible beast-evolution world will need,
//! while keeping component-storage footprint small and hash lookups cheap.
//!
//! Allocation strategy is owned by the ECS layer (L3); this crate only
//! provides the type and a deterministic generator helper suitable for tests
//! and bootstrap code.

use core::fmt;

use serde::{Deserialize, Serialize};

/// Opaque identifier for an entity (creature, pathogen, agent, settlement,
/// biome cell, …). Ordering is by numeric value and is stable across runs,
/// which makes `EntityId` safe to use as a key in a `BTreeMap` for
/// deterministic iteration.
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct EntityId(u32);

impl EntityId {
    /// The reserved "no entity" sentinel. Used by components that optionally
    /// reference another entity (e.g., parent, target).
    pub const NONE: Self = Self(u32::MAX);

    /// Construct from a raw `u32`. Accepting `u32::MAX` is permitted — callers
    /// that care about the sentinel value should check explicitly.
    #[inline]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// The underlying `u32`.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// `true` if this id is [`EntityId::NONE`].
    #[inline]
    pub const fn is_none(self) -> bool {
        self.0 == u32::MAX
    }
}

impl fmt::Debug for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_none() {
            write!(f, "EntityId(NONE)")
        } else {
            write!(f, "EntityId({})", self.0)
        }
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Monotonic, deterministic `EntityId` allocator.
///
/// The ECS layer will build a richer system (freelist, generational indices,
/// archetype-specific pools) on top of this. For now, this allocator gives
/// tests and bootstrap code a reproducible way to mint IDs.
///
/// [`EntityIdAllocator::alloc`] returns `Option<EntityId>` so that exhaustion
/// cannot be silently ignored: once all `u32::MAX - 1` non-sentinel IDs have
/// been handed out, further calls return `None`. The ECS layer will almost
/// certainly replace this with a generational-index allocator before
/// exhaustion is a real concern, but the uniqueness invariant must be
/// respected even in bootstrap code.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityIdAllocator {
    /// `pub(crate)` so external callers can't bypass `alloc()` /
    /// `is_exhausted()` by struct-literal construction. The internal
    /// tests fast-forward by setting this directly.
    pub(crate) next: u32,
}

impl EntityIdAllocator {
    /// Fresh allocator starting at id 0.
    #[inline]
    pub const fn new() -> Self {
        Self { next: 0 }
    }

    /// Allocate the next id.
    ///
    /// Returns `Some(EntityId)` for each of the first `u32::MAX - 1` calls
    /// (ids `0..=u32::MAX - 2`), and `None` thereafter. The id `u32::MAX` is
    /// reserved for [`EntityId::NONE`] and is never returned.
    #[inline]
    #[must_use]
    pub fn alloc(&mut self) -> Option<EntityId> {
        // The last valid id is u32::MAX - 2 so that the NONE sentinel
        // (u32::MAX) is never allocated; after handing out MAX - 2, `next`
        // becomes MAX - 1 and the allocator is exhausted.
        if self.next >= u32::MAX - 1 {
            return None;
        }
        let id = EntityId(self.next);
        self.next += 1;
        Some(id)
    }

    /// `true` once [`EntityIdAllocator::alloc`] will return `None`.
    #[inline]
    pub const fn is_exhausted(&self) -> bool {
        self.next >= u32::MAX - 1
    }

    /// Number of ids allocated so far.
    #[inline]
    pub const fn count(&self) -> u32 {
        self.next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_raw_roundtrip() {
        let id = EntityId::new(42);
        assert_eq!(id.raw(), 42);
    }

    #[test]
    fn none_sentinel() {
        assert!(EntityId::NONE.is_none());
        assert!(!EntityId::new(0).is_none());
    }

    #[test]
    fn ordering_is_numeric() {
        assert!(EntityId::new(1) < EntityId::new(2));
        assert!(EntityId::new(u32::MAX - 1) < EntityId::NONE);
    }

    #[test]
    fn allocator_is_monotonic_and_deterministic() {
        let mut alloc = EntityIdAllocator::new();
        let a = alloc.alloc().unwrap();
        let b = alloc.alloc().unwrap();
        let c = alloc.alloc().unwrap();
        assert_eq!(a, EntityId::new(0));
        assert_eq!(b, EntityId::new(1));
        assert_eq!(c, EntityId::new(2));
        assert_eq!(alloc.count(), 3);
        assert!(!alloc.is_exhausted());
    }

    #[test]
    fn allocator_reports_exhaustion_via_none() {
        // Fast-forward to the last valid id (u32::MAX - 2).
        let mut alloc = EntityIdAllocator { next: u32::MAX - 2 };
        let last = alloc.alloc().expect("one id remaining");
        assert_eq!(last.raw(), u32::MAX - 2);
        assert!(alloc.is_exhausted());
        // All subsequent calls are None — never a duplicate, never the NONE
        // sentinel leaking out.
        assert_eq!(alloc.alloc(), None);
        assert_eq!(alloc.alloc(), None);
    }

    #[test]
    fn allocator_never_returns_none_sentinel() {
        let mut alloc = EntityIdAllocator { next: u32::MAX - 2 };
        let id = alloc.alloc().unwrap();
        assert!(!id.is_none());
    }

    #[test]
    fn debug_formats_none() {
        assert_eq!(format!("{:?}", EntityId::NONE), "EntityId(NONE)");
        assert_eq!(format!("{:?}", EntityId::new(7)), "EntityId(7)");
    }

    #[test]
    fn serde_transparent() {
        let id = EntityId::new(12345);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "12345");
        let back: EntityId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
