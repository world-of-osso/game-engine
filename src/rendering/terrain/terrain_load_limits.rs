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

    #[test]
    fn budget_drains_queue_over_multiple_frames() {
        let (tx, rx) = mpsc::channel::<u32>();
        for i in 0..3 {
            tx.send(i).unwrap();
        }
        drop(tx);

        let budget = 1;
        let mut all_taken = Vec::new();
        // Simulate 5 frames — should drain all 3 tiles
        for _ in 0..5 {
            let taken: Vec<_> = rx.try_iter().take(budget).collect();
            all_taken.extend(taken);
        }
        assert_eq!(all_taken, vec![0, 1, 2]);
    }

    #[test]
    fn budget_larger_than_queue_takes_all() {
        let (tx, rx) = mpsc::channel::<u32>();
        tx.send(10).unwrap();
        tx.send(20).unwrap();
        drop(tx);

        let budget = 10;
        let taken: Vec<_> = rx.try_iter().take(budget).collect();
        assert_eq!(taken, vec![10, 20]);
    }

    #[test]
    fn budget_on_empty_queue_takes_nothing() {
        let (_tx, rx) = mpsc::channel::<u32>();
        let budget = max_tiles_per_frame();
        let taken: Vec<_> = rx.try_iter().take(budget).collect();
        assert!(taken.is_empty());
    }

    #[test]
    fn pending_limit_caps_dispatched_loads() {
        let pending_limit = max_pending_tile_loads(); // 1
        // Simulate dispatch: only start loads up to pending_limit
        let desired = vec![1, 2, 3, 4, 5];
        let dispatched: Vec<_> = desired.into_iter().take(pending_limit).collect();
        assert_eq!(dispatched.len(), 1);
    }

    #[test]
    fn budget_preserves_order() {
        let (tx, rx) = mpsc::channel::<u32>();
        for i in 0..5 {
            tx.send(i * 10).unwrap();
        }

        let budget = 2;
        let frame1: Vec<_> = rx.try_iter().take(budget).collect();
        let frame2: Vec<_> = rx.try_iter().take(budget).collect();
        let frame3: Vec<_> = rx.try_iter().take(budget).collect();
        assert_eq!(frame1, vec![0, 10]);
        assert_eq!(frame2, vec![20, 30]);
        assert_eq!(frame3, vec![40]);
    }
}
