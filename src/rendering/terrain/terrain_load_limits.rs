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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn default_budget_is_one() {
        // Clear any env override so we get the default.
        unsafe { std::env::remove_var("GAME_ENGINE_MAX_TILES_PER_FRAME") };
        assert_eq!(max_tiles_per_frame(), 1);
    }

    #[test]
    fn default_pending_is_one() {
        unsafe { std::env::remove_var("GAME_ENGINE_MAX_PENDING_TILE_LOADS") };
        assert_eq!(max_pending_tile_loads(), 1);
    }

    /// Simulates `receive_loaded_tiles` budgeting: sending multiple items
    /// into a channel and draining only `budget` per "frame".
    #[test]
    fn budget_limits_tiles_consumed_per_frame() {
        let (tx, rx) = mpsc::channel::<u32>();
        // Enqueue 5 tiles.
        for i in 0..5 {
            tx.send(i).unwrap();
        }
        let budget = max_tiles_per_frame(); // 1
        let taken: Vec<_> = rx.try_iter().take(budget).collect();
        assert_eq!(taken.len(), 1, "should consume exactly budget tiles");

        // Remaining tiles are still in the channel for subsequent frames.
        let remaining: Vec<_> = rx.try_iter().collect();
        assert_eq!(remaining.len(), 4, "remaining tiles should stay queued");
    }
}
