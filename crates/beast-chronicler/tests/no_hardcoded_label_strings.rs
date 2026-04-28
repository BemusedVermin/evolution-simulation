//! Compile-time-ish enforcement of INVARIANTS.md §2 (Mechanics-Label
//! Separation) for the S10.6 surface: no manifest-defined label id may
//! appear as a string literal anywhere in `crates/beast-chronicler/src`.
//!
//! `src/label.rs` holds the only structural string ("labels", "id",
//! "primitives", "min_confidence") needed for serde/JSON-Schema, none of
//! which collide with a manifest *id*. If this test ever fires, a label
//! id has leaked into Rust code — fix it by reading the id from the
//! manifest at runtime.
//!
//! The grep is shallow (raw byte scan, no AST) but that is the right
//! tradeoff for a determinism-class invariant: any false positive forces
//! a human review, which is what we want.

use std::path::Path;

const SHIPPED_LABEL_IDS: &[&str] = &["echolocation", "bioluminescence", "pack_hunting"];

fn collect_rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable src dir") {
        let entry = entry.expect("readable dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_shipped_label_id_appears_in_src() {
    // CARGO_MANIFEST_DIR points at the crate root at test compile time.
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_dir = crate_root.join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_dir, &mut files);
    assert!(
        !files.is_empty(),
        "expected at least one .rs file under {src_dir:?}"
    );

    let mut violations = Vec::new();
    for file in &files {
        let body = std::fs::read_to_string(file).expect("readable source file");
        for needle in SHIPPED_LABEL_IDS {
            if body.contains(needle) {
                violations.push(format!(
                    "{}: contains hardcoded label id `{}`",
                    file.display(),
                    needle
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "label ids leaked into src/ — INVARIANTS §2:\n{}",
        violations.join("\n")
    );
}
