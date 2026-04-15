//! Schema-drift guard: mirrors `beast-channels::tests::schema_drift` for the
//! primitive vocabulary. Walks `documentation/schemas/primitive_vocabulary/`
//! at test time and parses every `.json` through
//! `PrimitiveManifest::from_json_str`.

use std::fs;
use std::path::{Path, PathBuf};

use beast_primitives::PrimitiveManifest;

fn vocabulary_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("documentation")
        .join("schemas")
        .join("primitive_vocabulary")
}

#[test]
fn every_vocabulary_manifest_parses() {
    let dir = vocabulary_dir();
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
        PrimitiveManifest::from_json_str(&source).unwrap_or_else(|e| {
            panic!(
                "{} failed to parse as PrimitiveManifest:\n{e}",
                path.display()
            )
        });
        checked += 1;
    }

    // The starter vocabulary is exactly 16 primitives (see
    // `documentation/schemas/README.md`). If that count ever changes, update
    // this assertion in the same PR that moves the catalogue — drifting the
    // number silently would be a reviewer footgun.
    assert_eq!(
        checked,
        16,
        "expected 16 starter primitives under {}, found {checked}",
        dir.display(),
    );
}

#[test]
fn embedded_schema_matches_canonical_file() {
    let canonical = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("documentation")
        .join("schemas")
        .join("primitive_manifest.schema.json");
    let on_disk = fs::read_to_string(&canonical)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", canonical.display()));
    assert_eq!(
        on_disk,
        beast_primitives::PRIMITIVE_MANIFEST_SCHEMA,
        "embedded schema has drifted from {}",
        canonical.display(),
    );
}
