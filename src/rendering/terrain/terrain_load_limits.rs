pub fn max_pending_tile_loads() -> usize {
    std::env::var("GAME_ENGINE_MAX_PENDING_TILE_LOADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

/// Maximum number of completed tiles to apply per frame.
/// Prevents micro-freezes when multiple tiles finish simultaneously.
pub fn max_tiles_per_frame() -> usize {
    std::env::var("GAME_ENGINE_MAX_TILES_PER_FRAME")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}
