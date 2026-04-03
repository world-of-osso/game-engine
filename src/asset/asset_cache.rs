use std::path::{Path, PathBuf};

use super::asset_resolver::{AssetResolver, resolver};

fn ensure_cached_in_dir(
    resolver: &dyn AssetResolver,
    fdid: u32,
    dir: &str,
    ext: &str,
) -> Option<PathBuf> {
    let path = crate::paths::shared_data_path(dir).join(format!("{fdid}.{ext}"));
    resolver.ensure_cached(fdid, &path)
}

/// Return a local cached texture path, extracting from local WoW CASC on demand.
pub fn texture(fdid: u32) -> Option<PathBuf> {
    ensure_cached_in_dir(resolver(), fdid, "textures", "blp")
}

/// Return a local cached model path, extracting from local WoW CASC on demand.
pub fn model(fdid: u32) -> Option<PathBuf> {
    ensure_cached_in_dir(resolver(), fdid, "models", "m2")
}

/// Ensure the requested asset is cached at a specific local path and return that path.
pub fn file_at_path(fdid: u32, out_path: &Path) -> Option<PathBuf> {
    resolver().ensure_cached(fdid, out_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyResolver;

    impl AssetResolver for DummyResolver {
        fn resolve_bytes(&self, fdid: u32) -> Option<Vec<u8>> {
            Some(vec![fdid as u8])
        }

        fn ensure_cached(&self, fdid: u32, out_path: &Path) -> Option<PathBuf> {
            let _ = fdid;
            Some(out_path.to_path_buf())
        }

        fn resolve_path(&self, fdid: u32) -> Option<String> {
            Some(format!("dummy/{fdid}"))
        }

        fn lookup_path(&self, path: &str) -> Option<u32> {
            path.strip_prefix("dummy/")?.parse().ok()
        }
    }

    #[test]
    fn ensure_cached_in_dir_uses_shared_target_path() {
        let path = ensure_cached_in_dir(&DummyResolver, 42, "textures", "blp").unwrap();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("textures/42.blp"));
    }
}
