use std::path::{Path, PathBuf};

pub trait AssetResolver: Send + Sync {
    fn resolve_bytes(&self, fdid: u32) -> Option<Vec<u8>>;
    fn ensure_cached(&self, fdid: u32, out_path: &Path) -> Option<PathBuf>;
    fn resolve_path(&self, fdid: u32) -> Option<String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CascAssetResolver;

impl AssetResolver for CascAssetResolver {
    fn resolve_bytes(&self, fdid: u32) -> Option<Vec<u8>> {
        super::casc_resolver::resolve_bytes(fdid)
    }

    fn ensure_cached(&self, fdid: u32, out_path: &Path) -> Option<PathBuf> {
        super::casc_resolver::ensure_file_cached_at_path(fdid, out_path)
    }

    fn resolve_path(&self, fdid: u32) -> Option<String> {
        crate::listfile::lookup_fdid(fdid).map(str::to_owned)
    }
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
            Some(out_path.join(format!("{fdid}.bin")))
        }

        fn resolve_path(&self, fdid: u32) -> Option<String> {
            Some(format!("dummy/{fdid}"))
        }
    }

    fn exercise_resolver(resolver: &dyn AssetResolver) -> (Vec<u8>, PathBuf, String) {
        (
            resolver.resolve_bytes(7).unwrap(),
            resolver.ensure_cached(7, Path::new("cache")).unwrap(),
            resolver.resolve_path(7).unwrap(),
        )
    }

    #[test]
    fn asset_resolver_trait_surface_is_object_safe() {
        let (bytes, path, wow_path) = exercise_resolver(&DummyResolver);
        assert_eq!(bytes, vec![7]);
        assert_eq!(path, PathBuf::from("cache/7.bin"));
        assert_eq!(wow_path, "dummy/7");
    }
}
