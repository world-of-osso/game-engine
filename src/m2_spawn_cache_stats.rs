use crate::m2_texture_composite::COMPOSITED_TEXTURE_CACHE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompositedTextureCacheStats {
    pub entries: usize,
    pub est_cpu_bytes: u64,
}

pub fn composited_texture_cache_stats() -> CompositedTextureCacheStats {
    let Some(cache) = COMPOSITED_TEXTURE_CACHE.get() else {
        return CompositedTextureCacheStats::default();
    };
    let cache = cache.lock().unwrap();
    CompositedTextureCacheStats {
        entries: cache.len(),
        est_cpu_bytes: 0,
    }
}
