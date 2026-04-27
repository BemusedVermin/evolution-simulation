//! Sprite atlas: id → rectangle lookup loaded from a JSON manifest.
//!
//! S9.2 ships the *headless* atlas: a parsed, validated manifest that
//! resolves a [`SpriteId`] to a pixel-space [`Rect`]. The actual
//! GPU-side texture upload sits on top of the SDL backend and lands in
//! a follow-up — the world-map and encounter renderers (S9.3 / S9.4)
//! call into the SDL feature directly when they need to draw.
//!
//! # Manifest shape
//!
//! ```json
//! {
//!   "version": 1,
//!   "source": "assets/sprites/atlas.png",
//!   "entries": [
//!     { "id": "biome.forest", "x": 0,  "y": 0,  "w": 32, "h": 32 },
//!     { "id": "biome.ocean",  "x": 32, "y": 0,  "w": 32, "h": 32 }
//!   ]
//! }
//! ```
//!
//! `version` lets us evolve the schema without silently parsing old
//! files; today only `1` is accepted. Duplicate ids and zero-area
//! rectangles fail the load with a typed [`AtlasError`].

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable identifier for a sub-image within the atlas.
///
/// Identifiers are dotted strings so they're easy to namespace
/// (`"biome.forest"`, `"creature.glyph.elastic"`). The visual pipeline
/// and renderers reference sprites only via `SpriteId`; pixel
/// coordinates live in the manifest, never in code.
///
/// The inner field is private so future invariants on the id format
/// (charset, namespace rules) can be enforced in [`Self::new`] without
/// breaking callers that wrote the value directly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpriteId(String);

impl SpriteId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SpriteId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Pixel-space rectangle. Origin is top-left; `w`/`h` are positive.
///
/// `i32` rather than `u32` so the type round-trips through SDL3's
/// `sdl3::rect::Rect`, which uses signed coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
}

// ---------------------------------------------------------------------------
// Manifest types — the wire format
// ---------------------------------------------------------------------------

/// One atlas entry as it appears in the JSON manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtlasEntryWire {
    id: String,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

/// Full atlas manifest as it appears on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AtlasManifestWire {
    version: u32,
    /// Path to the atlas image. Resolved relative to the manifest file.
    source: String,
    entries: Vec<AtlasEntryWire>,
}

const MANIFEST_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum AtlasError {
    #[error("failed to read atlas manifest at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("invalid JSON in atlas manifest at {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error(
        "unsupported atlas manifest version {found} at {path}: only version {} is recognised",
        MANIFEST_VERSION
    )]
    UnsupportedVersion { path: PathBuf, found: u32 },

    #[error("invalid region for sprite `{id}` in {path}: w and h must be positive (got {w}x{h})")]
    InvalidRegion {
        path: PathBuf,
        id: String,
        w: i32,
        h: i32,
    },

    #[error(
        "invalid origin for sprite `{id}` in {path}: x and y must be non-negative (got {x},{y})"
    )]
    InvalidOrigin {
        path: PathBuf,
        id: String,
        x: i32,
        y: i32,
    },

    #[error("duplicate sprite id `{id}` in atlas manifest")]
    DuplicateId { id: String },

    #[error("empty source path in atlas manifest at {path}")]
    EmptySource { path: PathBuf },

    #[error("empty sprite id in atlas manifest at {path}")]
    EmptyId { path: PathBuf },
}

// ---------------------------------------------------------------------------
// Public type
// ---------------------------------------------------------------------------

/// Parsed, validated atlas. Holds id → [`Rect`] lookup + the manifest's
/// resolved source-image path. GPU-side texture handling is layered on
/// top of this type by the SDL backend.
#[derive(Debug, Clone)]
pub struct SpriteAtlas {
    /// Path to the source image, resolved relative to the manifest.
    source_path: PathBuf,
    /// Lookup table — `BTreeMap` so iteration is deterministic.
    regions: BTreeMap<SpriteId, Rect>,
}

impl SpriteAtlas {
    /// Parse and validate an atlas manifest at `path`. The atlas image
    /// itself is *not* opened here — that happens at GPU-upload time.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, AtlasError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| AtlasError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse(&bytes, path)
    }

    /// Parse an atlas manifest from raw JSON bytes. Path is used purely
    /// for error reporting + resolving the relative `source` field.
    pub fn parse(bytes: &[u8], path: impl AsRef<Path>) -> Result<Self, AtlasError> {
        let path = path.as_ref();
        let wire: AtlasManifestWire =
            serde_json::from_slice(bytes).map_err(|source| AtlasError::Json {
                path: path.to_path_buf(),
                source,
            })?;

        if wire.version != MANIFEST_VERSION {
            return Err(AtlasError::UnsupportedVersion {
                path: path.to_path_buf(),
                found: wire.version,
            });
        }

        if wire.source.trim().is_empty() {
            return Err(AtlasError::EmptySource {
                path: path.to_path_buf(),
            });
        }
        let source_path = path
            .parent()
            .map(|p| p.join(&wire.source))
            .unwrap_or_else(|| PathBuf::from(&wire.source));

        let mut regions = BTreeMap::new();
        for entry in wire.entries {
            let (id, rect) = validate_entry(entry, &regions, path)?;
            regions.insert(id, rect);
        }

        Ok(Self {
            source_path,
            regions,
        })
    }

    /// Resolved path to the atlas image (absolute or relative to CWD).
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    /// Look up a sprite's pixel rectangle.
    pub fn region(&self, id: &SpriteId) -> Option<Rect> {
        self.regions.get(id).copied()
    }

    /// Number of entries in the atlas.
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// True iff the atlas has zero entries.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Iterate `(SpriteId, Rect)` pairs in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&SpriteId, &Rect)> {
        self.regions.iter()
    }
}

/// Validate one wire-format atlas entry and convert it to its in-memory
/// `(SpriteId, Rect)` form.
///
/// Triage order: empty id → non-positive dimensions → negative origin →
/// duplicate id. The first check that fails wins; e.g. an entry with both
/// `w=0` and `x=-1` surfaces `InvalidRegion`. Callers fix one issue at a
/// time, so a single deterministic error per entry is more useful than a
/// collected list.
fn validate_entry(
    entry: AtlasEntryWire,
    regions: &BTreeMap<SpriteId, Rect>,
    path: &Path,
) -> Result<(SpriteId, Rect), AtlasError> {
    if entry.id.is_empty() {
        return Err(AtlasError::EmptyId {
            path: path.to_path_buf(),
        });
    }
    if entry.w <= 0 || entry.h <= 0 {
        return Err(AtlasError::InvalidRegion {
            path: path.to_path_buf(),
            id: entry.id,
            w: entry.w,
            h: entry.h,
        });
    }
    if entry.x < 0 || entry.y < 0 {
        return Err(AtlasError::InvalidOrigin {
            path: path.to_path_buf(),
            id: entry.id,
            x: entry.x,
            y: entry.y,
        });
    }
    // Duplicate-id check before constructing the final `SpriteId` so the
    // success path can move `entry.id` into `SpriteId::new` instead of
    // cloning it.
    if regions.contains_key(&SpriteId::new(&entry.id)) {
        return Err(AtlasError::DuplicateId { id: entry.id });
    }
    let rect = Rect::new(entry.x, entry.y, entry.w, entry.h);
    Ok((SpriteId::new(entry.id), rect))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_manifest() -> &'static str {
        r#"{
            "version": 1,
            "source": "atlas.png",
            "entries": [
                { "id": "biome.forest", "x": 0,  "y": 0,  "w": 32, "h": 32 },
                { "id": "biome.ocean",  "x": 32, "y": 0,  "w": 32, "h": 32 },
                { "id": "creature.glyph.default", "x": 0, "y": 32, "w": 16, "h": 16 }
            ]
        }"#
    }

    #[test]
    fn parses_valid_manifest() {
        let atlas =
            SpriteAtlas::parse(fixture_manifest().as_bytes(), "/tmp/atlas.json").expect("parse ok");
        assert_eq!(atlas.len(), 3);
        assert_eq!(
            atlas.region(&SpriteId::from("biome.forest")),
            Some(Rect::new(0, 0, 32, 32))
        );
        assert_eq!(
            atlas.region(&SpriteId::from("creature.glyph.default")),
            Some(Rect::new(0, 32, 16, 16))
        );
        assert_eq!(atlas.region(&SpriteId::from("does.not.exist")), None);
    }

    #[test]
    fn parses_empty_entries_list() {
        let manifest = r#"{"version":1,"source":"a.png","entries":[]}"#;
        let atlas = SpriteAtlas::parse(manifest.as_bytes(), "/tmp/a.json").expect("parse ok");
        assert!(atlas.is_empty());
    }

    #[test]
    fn rejects_zero_area_rect() {
        let manifest = r#"{"version":1,"source":"a.png","entries":[
            {"id":"x","x":0,"y":0,"w":0,"h":4}
        ]}"#;
        let err =
            SpriteAtlas::parse(manifest.as_bytes(), "/tmp/a.json").expect_err("zero w must error");
        assert!(matches!(err, AtlasError::InvalidRegion { .. }));
    }

    #[test]
    fn rejects_negative_origin() {
        let manifest = r#"{"version":1,"source":"a.png","entries":[
            {"id":"x","x":-1,"y":0,"w":4,"h":4}
        ]}"#;
        let err = SpriteAtlas::parse(manifest.as_bytes(), "/tmp/a.json")
            .expect_err("negative x must error");
        assert!(matches!(err, AtlasError::InvalidOrigin { .. }));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let manifest = r#"{"version":1,"source":"a.png","entries":[
            {"id":"x","x":0,"y":0,"w":4,"h":4},
            {"id":"x","x":4,"y":0,"w":4,"h":4}
        ]}"#;
        let err = SpriteAtlas::parse(manifest.as_bytes(), "/tmp/a.json")
            .expect_err("duplicate id must error");
        assert!(matches!(err, AtlasError::DuplicateId { .. }));
    }

    #[test]
    fn rejects_empty_id() {
        let manifest = r#"{"version":1,"source":"a.png","entries":[
            {"id":"","x":0,"y":0,"w":4,"h":4}
        ]}"#;
        let err = SpriteAtlas::parse(manifest.as_bytes(), "/tmp/a.json")
            .expect_err("empty id must error");
        assert!(matches!(err, AtlasError::EmptyId { .. }));
    }

    #[test]
    fn rejects_empty_source_path() {
        let manifest = r#"{"version":1,"source":"","entries":[]}"#;
        let err = SpriteAtlas::parse(manifest.as_bytes(), "/tmp/a.json")
            .expect_err("empty source must error");
        assert!(matches!(err, AtlasError::EmptySource { .. }));
    }

    #[test]
    fn rejects_unsupported_version() {
        let manifest = r#"{"version":99,"source":"a.png","entries":[]}"#;
        let err = SpriteAtlas::parse(manifest.as_bytes(), "/tmp/a.json")
            .expect_err("future version must error");
        assert!(matches!(err, AtlasError::UnsupportedVersion { .. }));
    }

    #[test]
    fn rejects_garbage_json() {
        let err = SpriteAtlas::parse(b"not json", "/tmp/a.json").expect_err("garbage must error");
        assert!(matches!(err, AtlasError::Json { .. }));
    }

    #[test]
    fn iter_is_sorted_by_sprite_id() {
        let manifest = r#"{"version":1,"source":"a.png","entries":[
            {"id":"zebra","x":0,"y":0,"w":4,"h":4},
            {"id":"alpha","x":0,"y":4,"w":4,"h":4},
            {"id":"mango","x":0,"y":8,"w":4,"h":4}
        ]}"#;
        let atlas = SpriteAtlas::parse(manifest.as_bytes(), "/tmp/a.json").expect("parse ok");
        let ids: Vec<_> = atlas
            .iter()
            .map(|(id, _)| id.as_str().to_string())
            .collect();
        assert_eq!(ids, ["alpha", "mango", "zebra"]);
    }

    #[test]
    fn source_path_is_resolved_relative_to_manifest() {
        let atlas = SpriteAtlas::parse(fixture_manifest().as_bytes(), "/some/dir/atlas.json")
            .expect("parse ok");
        assert_eq!(atlas.source_path(), Path::new("/some/dir/atlas.png"));
    }

    #[test]
    fn round_trip_via_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest_path = dir.path().join("atlas.json");
        std::fs::write(&manifest_path, fixture_manifest()).expect("write");
        let atlas = SpriteAtlas::load(&manifest_path).expect("load");
        assert_eq!(atlas.len(), 3);
        assert_eq!(
            atlas.region(&SpriteId::from("biome.forest")),
            Some(Rect::new(0, 0, 32, 32))
        );
    }
}
