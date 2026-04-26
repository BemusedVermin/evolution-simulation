//! Integration tests for [`SpriteAtlas`].
//!
//! These tests are kept separate from the unit tests inside
//! `src/sprite.rs` because they exercise the *workspace-shipped*
//! placeholder manifest at `assets/sprites/atlas.json`. The unit tests
//! cover the parser semantics in isolation; this file's job is to make
//! sure the asset committed to the repo stays valid.

use beast_render::{Rect, SpriteAtlas, SpriteId};

/// Walk up from `crates/beast-render` until we find the workspace
/// root. Asserts that the resolved directory contains a `Cargo.toml`,
/// so a future move of the crate produces a useful failure instead of
/// silently pointing at the wrong place.
fn workspace_root() -> std::path::PathBuf {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = crate_dir
        .ancestors()
        .nth(2)
        .expect("workspace root is two levels up from the crate manifest")
        .to_path_buf();
    debug_assert!(
        candidate.join("Cargo.toml").is_file(),
        "expected workspace Cargo.toml at {}",
        candidate.display()
    );
    candidate
}

#[test]
fn shipped_atlas_loads_and_contains_canonical_biomes() {
    let path = workspace_root().join("assets/sprites/atlas.json");
    let atlas = SpriteAtlas::load(&path)
        .unwrap_or_else(|e| panic!("shipped atlas at {} failed to load: {e}", path.display()));

    // Every BiomeKind must have a matching glyph so the world map
    // renderer (S9.3) can blanket-tile any generated world.
    for biome in [
        "biome.ocean",
        "biome.forest",
        "biome.plains",
        "biome.desert",
        "biome.mountain",
        "biome.tundra",
    ] {
        let region = atlas.region(&SpriteId::from(biome));
        assert!(
            region.is_some(),
            "shipped atlas is missing required biome glyph `{biome}`"
        );
    }
}

#[test]
fn shipped_atlas_default_creature_glyph_is_present() {
    let path = workspace_root().join("assets/sprites/atlas.json");
    let atlas = SpriteAtlas::load(&path).expect("load");
    // The world map renderer falls back to `creature.glyph.default`
    // when a species-specific glyph isn't present. That fallback must
    // always exist.
    assert_eq!(
        atlas
            .region(&SpriteId::from("creature.glyph.default"))
            .map(|r| (r.w, r.h)),
        Some((16, 16))
    );
}

#[test]
fn shipped_atlas_entries_are_non_overlapping_within_rows() {
    // Defensive check that the placeholder grid layout doesn't drift —
    // a typo in `assets/sprites/atlas.json` could silently make two
    // sprites share pixel coordinates and the renderer would draw the
    // wrong glyph. We're not strict about *all* overlap — sprites in
    // different rows can share x ranges — only same-y rows. NOTE: the
    // adjacent-pair sweep below catches neighbour collisions only;
    // non-adjacent sprites on the same row could still overlap if the
    // grid drifts wildly. Full O(n²) overlap detection is overkill for
    // a hand-edited fixture; revisit if the atlas grows past ~50
    // entries or starts admitting sprites at arbitrary positions.
    let path = workspace_root().join("assets/sprites/atlas.json");
    let atlas = SpriteAtlas::load(&path).expect("load");

    let mut rects: Vec<(SpriteId, Rect)> = atlas.iter().map(|(id, r)| (id.clone(), *r)).collect();
    rects.sort_by_key(|(_, r)| (r.y, r.x));

    for window in rects.windows(2) {
        let (id_a, ra) = &window[0];
        let (id_b, rb) = &window[1];
        if ra.y != rb.y {
            continue;
        }
        let a_right = ra.x + ra.w;
        assert!(
            a_right <= rb.x,
            "atlas entries `{}` and `{}` overlap horizontally on row y={} \
             ({}..{} vs {}..{})",
            id_a.as_str(),
            id_b.as_str(),
            ra.y,
            ra.x,
            a_right,
            rb.x,
            rb.x + rb.w,
        );
    }
}
