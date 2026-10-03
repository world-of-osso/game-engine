//! Included through `#[path]` by tests that read local WoW assets from `data/`.

use std::path::Path;

/// Returns `path`, failing the test with how to obtain it when it is absent.
#[track_caller]
pub fn require_asset<P: AsRef<Path>>(path: P) -> P {
    let shown = path.as_ref();
    assert!(
        shown.exists(),
        "missing test asset {}: extract it from local CASC per docs/casc-extraction.md \
         (`cargo run --manifest-path ../asset-resolver/Cargo.toml --bin casc-local -- <fdid> -o <dir>`; \
         DB2 CSVs: `scripts/export_db2_csv.py`); Depot runs also need it in godot/depot-test-assets.txt",
        shown.display()
    );
    path
}
