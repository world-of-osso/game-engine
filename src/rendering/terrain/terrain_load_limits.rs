pub fn max_pending_tile_loads() -> usize {
    std::env::var("GAME_ENGINE_MAX_PENDING_TILE_LOADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}
