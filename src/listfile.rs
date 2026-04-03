#[cfg(feature = "casc")]
pub use osso_asset_resolver::listfile::{CachedListfile, Listfile};
#[cfg(feature = "casc")]
pub use osso_asset_resolver::listfile_cache;

use crate::asset::asset_resolver::resolver;

/// Look up a WoW internal path by FileDataID.
pub fn lookup_fdid(fdid: u32) -> Option<&'static str> {
    resolver()
        .resolve_path(fdid)
        .map(|path| Box::leak(path.into_boxed_str()) as &'static str)
}

/// Look up a FileDataID by WoW internal path (case-insensitive).
pub fn lookup_path(path: &str) -> Option<u32> {
    resolver().lookup_path(path)
}

#[cfg(test)]
#[path = "../tests/unit/listfile_tests.rs"]
mod tests;
