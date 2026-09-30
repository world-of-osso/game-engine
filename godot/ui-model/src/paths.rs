//! Data directory the shared item tables (`item_catalog`, `item_icons`) and ui-toolkit's
//! text measurement (`fonts/`) read from. The host sets it once, before the first lookup.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static DATA_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Set the data directory; a second call must name the same directory.
pub fn set_data_root(root: PathBuf) -> Result<(), String> {
    ui_toolkit::widgets::font_string::set_font_directory(root.join("fonts"))?;
    let current = DATA_ROOT.get_or_init(|| root.clone());
    if *current == root {
        Ok(())
    } else {
        Err(format!(
            "Item data root is {}, not {}",
            current.display(),
            root.display()
        ))
    }
}

pub fn resolve_data_path(relative: impl AsRef<Path>) -> PathBuf {
    DATA_ROOT.get_or_init(unset_data_root).join(relative)
}

#[cfg(not(test))]
fn unset_data_root() -> PathBuf {
    panic!("item data root read before the host set it")
}

/// Unit tests read the repository's data directory.
#[cfg(test)]
fn unset_data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}
