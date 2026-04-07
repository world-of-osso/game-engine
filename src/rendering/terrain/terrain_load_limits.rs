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

    // --- Background decode streaming: completion & error recovery ---

    /// Models the `TileLoadResult` success/failure flow without Bevy deps.
    #[derive(Debug, PartialEq)]
    enum MockTileResult {
        Success {
            tile_y: u32,
            tile_x: u32,
        },
        Failed {
            tile_y: u32,
            tile_x: u32,
            error: String,
        },
    }

    #[test]
    fn streaming_completion_delivered_through_channel() {
        let (tx, rx) = mpsc::channel::<MockTileResult>();
        // Simulate background thread completing a tile
        std::thread::spawn(move || {
            tx.send(MockTileResult::Success {
                tile_y: 32,
                tile_x: 48,
            })
            .unwrap();
        })
        .join()
        .unwrap();

        let results: Vec<_> = rx.try_iter().take(1).collect();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            MockTileResult::Success {
                tile_y: 32,
                tile_x: 48
            }
        );
    }

    #[test]
    fn error_recovery_failed_tile_removed_from_pending() {
        let (tx, rx) = mpsc::channel::<MockTileResult>();
        tx.send(MockTileResult::Failed {
            tile_y: 10,
            tile_x: 20,
            error: "file not found".into(),
        })
        .unwrap();

        // Simulate the main thread's handle_tile_result flow
        let mut pending: std::collections::HashSet<(u32, u32)> =
            [(10, 20), (30, 40)].into_iter().collect();

        for result in rx.try_iter().take(max_tiles_per_frame()) {
            match result {
                MockTileResult::Success { tile_y, tile_x } => {
                    pending.remove(&(tile_y, tile_x));
                }
                MockTileResult::Failed { tile_y, tile_x, .. } => {
                    pending.remove(&(tile_y, tile_x));
                }
            }
        }
        assert!(
            !pending.contains(&(10, 20)),
            "failed tile should be removed from pending"
        );
        assert!(pending.contains(&(30, 40)), "unrelated tile should remain");
    }

    #[test]
    fn mixed_success_and_failure_processed_in_order() {
        let (tx, rx) = mpsc::channel::<MockTileResult>();
        tx.send(MockTileResult::Success {
            tile_y: 1,
            tile_x: 1,
        })
        .unwrap();
        tx.send(MockTileResult::Failed {
            tile_y: 2,
            tile_x: 2,
            error: "corrupt".into(),
        })
        .unwrap();
        tx.send(MockTileResult::Success {
            tile_y: 3,
            tile_x: 3,
        })
        .unwrap();

        let results: Vec<_> = rx.try_iter().take(10).collect();
        assert_eq!(results.len(), 3);
        assert!(matches!(results[0], MockTileResult::Success { .. }));
        assert!(matches!(results[1], MockTileResult::Failed { .. }));
        assert!(matches!(results[2], MockTileResult::Success { .. }));
    }

    #[test]
    fn dropped_sender_signals_no_more_tiles() {
        let (tx, rx) = mpsc::channel::<MockTileResult>();
        drop(tx);
        let results: Vec<_> = rx.try_iter().collect();
        assert!(results.is_empty());
        // Subsequent try_iter also returns nothing (no panic)
        assert!(rx.try_iter().next().is_none());
    }
}
