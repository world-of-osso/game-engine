use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CascResolverPaths {
    data_root: PathBuf,
    shared_data_root: PathBuf,
    cache_root: PathBuf,
}

pub trait AssetResolver: Send + Sync {
    fn resolve_bytes(&self, fdid: u32) -> Option<Vec<u8>>;
    fn ensure_cached(&self, fdid: u32, out_path: &Path) -> Option<PathBuf>;
    fn resolve_path(&self, fdid: u32) -> Option<String>;
    fn lookup_path(&self, path: &str) -> Option<u32>;
}

static ASSET_RESOLVER: OnceLock<Box<dyn AssetResolver>> = OnceLock::new();

#[cfg(feature = "casc")]
impl AssetResolver for osso_asset_resolver::CascListfileResolver {
    fn resolve_bytes(&self, fdid: u32) -> Option<Vec<u8>> {
        osso_asset_resolver::CascListfileResolver::resolve_bytes(self, fdid)
    }

    fn ensure_cached(&self, fdid: u32, out_path: &Path) -> Option<PathBuf> {
        osso_asset_resolver::CascListfileResolver::ensure_cached(self, fdid, out_path)
    }

    fn resolve_path(&self, fdid: u32) -> Option<String> {
        osso_asset_resolver::CascListfileResolver::resolve_path(self, fdid)
    }

    fn lookup_path(&self, path: &str) -> Option<u32> {
        osso_asset_resolver::CascListfileResolver::lookup_path(self, path)
    }
}

#[cfg(not(feature = "casc"))]
#[derive(Debug, Default, Clone, Copy)]
struct NullAssetResolver;

#[cfg(not(feature = "casc"))]
impl AssetResolver for NullAssetResolver {
    fn resolve_bytes(&self, _fdid: u32) -> Option<Vec<u8>> {
        None
    }

    fn ensure_cached(&self, _fdid: u32, _out_path: &Path) -> Option<PathBuf> {
        None
    }

    fn resolve_path(&self, _fdid: u32) -> Option<String> {
        None
    }

    fn lookup_path(&self, _path: &str) -> Option<u32> {
        None
    }
}

fn default_resolver() -> Box<dyn AssetResolver> {
    #[cfg(feature = "casc")]
    {
        Box::new(configured_casc_resolver())
    }
    #[cfg(not(feature = "casc"))]
    {
        Box::new(NullAssetResolver)
    }
}

#[cfg(feature = "casc")]
fn default_casc_resolver_paths() -> CascResolverPaths {
    let shared_data_root = crate::paths::shared_data_root();
    CascResolverPaths {
        data_root: crate::paths::worktree_data_root(),
        shared_data_root: shared_data_root.clone(),
        cache_root: shared_data_root,
    }
}

#[cfg(feature = "casc")]
fn configured_casc_resolver() -> osso_asset_resolver::CascListfileResolver {
    let paths = default_casc_resolver_paths();
    let config = osso_asset_resolver::AssetResolverConfig::new()
        .with_data_root(paths.data_root)
        .with_shared_data_root(paths.shared_data_root)
        .with_cache_root(paths.cache_root);

    osso_asset_resolver::CascListfileResolver::new(config)
}

pub fn set_resolver(resolver: Box<dyn AssetResolver>) -> Result<(), Box<dyn AssetResolver>> {
    ASSET_RESOLVER.set(resolver)
}

pub fn resolver() -> &'static dyn AssetResolver {
    ASSET_RESOLVER.get_or_init(default_resolver).as_ref()
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

        fn lookup_path(&self, path: &str) -> Option<u32> {
            path.strip_prefix("dummy/")?.parse().ok()
        }
    }

    fn exercise_resolver(resolver: &dyn AssetResolver) -> (Vec<u8>, PathBuf, String, u32) {
        (
            resolver.resolve_bytes(7).unwrap(),
            resolver.ensure_cached(7, Path::new("cache")).unwrap(),
            resolver.resolve_path(7).unwrap(),
            resolver.lookup_path("dummy/7").unwrap(),
        )
    }

    #[test]
    fn default_casc_resolver_paths_use_game_engine_data_roots() {
        let paths = default_casc_resolver_paths();

        assert_eq!(paths.data_root, crate::paths::worktree_data_root());
        assert_eq!(paths.shared_data_root, crate::paths::shared_data_root());
        assert_eq!(paths.cache_root, crate::paths::shared_data_root());
    }

    #[test]
    fn asset_resolver_trait_surface_is_object_safe() {
        let (bytes, path, wow_path, fdid) = exercise_resolver(&DummyResolver);
        assert_eq!(bytes, vec![7]);
        assert_eq!(path, PathBuf::from("cache/7.bin"));
        assert_eq!(wow_path, "dummy/7");
        assert_eq!(fdid, 7);
    }
}
