use std::path::{Path, PathBuf};

/// Return a local cached texture path, extracting from local WoW CASC on demand.
pub fn texture(fdid: u32) -> Option<PathBuf> {
    super::casc_resolver::ensure_texture_cached(fdid)
}

/// Return a local cached model path, extracting from local WoW CASC on demand.
pub fn model(fdid: u32) -> Option<PathBuf> {
    super::casc_resolver::ensure_model_cached(fdid)
}

/// Ensure the requested asset is cached at a specific local path and return that path.
pub fn file_at_path(fdid: u32, out_path: &Path) -> Option<PathBuf> {
    super::casc_resolver::ensure_file_cached_at_path(fdid, out_path)
}
