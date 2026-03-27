use super::COMPOSITED_TEXTURE_CACHE;

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
        est_cpu_bytes: cache
            .values()
            .filter_map(|result| result.as_ref().ok())
            .map(|image| image.data.as_ref().map_or(0, |data| data.len() as u64))
            .sum(),
    }
}
