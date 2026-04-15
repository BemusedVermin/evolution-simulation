//! Schema-drift guard: walks `documentation/schemas/examples/` at test time
//! and parses every `.json` file through the runtime loader.
//!
//! The companion test in `example_manifests.rs` pins five specific examples
//! via `include_str!`. That hardcoded list only protects the files we
//! remembered to add to it — a sixth example landing without a corresponding
//! entry would slip through unchecked. This test iterates the directory at
//! runtime, so *any* new JSON fixture must parse or the build goes red.

use std::fs;
use std::path::{Path, PathBuf};

use beast_channels::ChannelManifest;

/// Path to `documentation/schemas/examples/` relative to the workspace root,
/// resolved via `CARGO_MANIFEST_DIR` so the test works from any cwd.
fn examples_dir() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("..")
        .join("..")
        .join("documentation")
        .join("schemas")
        .join("examples")
}

#[test]
fn every_example_manifest_parses() {
    let dir = examples_dir();
    let entries =
        fs::read_dir(&dir).unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()));

    let mut checked = 0usize;
    for entry in entries {
        let entry = entry.expect("valid DirEntry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        ChannelManifest::from_json_str(&source).unwrap_or_else(|e| {
            panic!(
                "{} failed to parse as ChannelManifest:\n{e}",
                path.display()
            )
        });
        checked += 1;
    }

    assert!(
        checked > 0,
        "no .json fixtures found under {} — did the docs directory move?",
        dir.display()
    );
}

/// Parallel guard: if the canonical JSON Schema file on disk ever diverges
/// from the one embedded into the crate via `include_str!`, this test will
/// fail. Catches the pathological case where someone copies the schema into
/// the crate directory and edits one copy without touching the other.
#[test]
fn embedded_schema_matches_canonical_file() {
    let canonical = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("documentation")
        .join("schemas")
        .join("channel_manifest.schema.json");
    let on_disk = fs::read_to_string(&canonical)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", canonical.display()));
    assert_eq!(
        on_disk,
        beast_channels::CHANNEL_MANIFEST_SCHEMA,
        "embedded schema has drifted from {}",
        canonical.display(),
    );
}
