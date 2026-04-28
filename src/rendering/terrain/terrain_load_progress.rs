use crate::terrain::AdtManager;

const MAP_TILE_BOUNDS: i32 = 64;

pub fn should_report_initial_world_load(adt_manager: &AdtManager) -> bool {
    !adt_manager.initial_load_reported && !adt_manager.map_name.is_empty()
}

pub fn initial_desired_tiles(adt_manager: &AdtManager) -> Vec<(u32, u32)> {
    let radius = adt_manager.load_radius as i32;
    let (center_y, center_x) = adt_manager.initial_tile;
    let mut tiles = Vec::with_capacity(((2 * radius + 1) * (2 * radius + 1)) as usize);
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let ty = center_y as i32 + dy;
            let tx = center_x as i32 + dx;
            if tile_coordinate_in_bounds(ty) && tile_coordinate_in_bounds(tx) {
                tiles.push((ty as u32, tx as u32));
            }
        }
    }
    tiles
}

fn tile_coordinate_in_bounds(coord: i32) -> bool {
    (0..MAP_TILE_BOUNDS).contains(&coord)
}

pub fn count_initial_tile_progress(
    adt_manager: &AdtManager,
    desired_tiles: &[(u32, u32)],
) -> (usize, usize, usize) {
    desired_tiles
        .iter()
        .fold((0, 0, 0), |(loaded, failed, pending), tile| {
            (
                loaded + usize::from(adt_manager.loaded.contains_key(tile)),
                failed + usize::from(adt_manager.failed.contains(tile)),
                pending + usize::from(adt_manager.pending.contains(tile)),
            )
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bevy::prelude::Entity;

    use super::*;

    fn sample_manager(initial_tile: (u32, u32), radius: u32) -> AdtManager {
        let mut adt_manager = AdtManager::default();
        adt_manager.map_name = "azeroth".into();
        adt_manager.load_radius = radius;
        adt_manager.initial_tile = initial_tile;
        adt_manager
    }

    #[test]
    fn initial_desired_tiles_bounds_clip_at_map_edge() {
        let adt_manager = sample_manager((0, 0), 1);
        let tiles = initial_desired_tiles(&adt_manager);
        let unique: HashSet<_> = tiles.iter().copied().collect();

        assert_eq!(tiles.len(), 4);
        assert_eq!(unique.len(), 4);
        assert!(tiles.iter().all(|(y, x)| *y < 64 && *x < 64));
    }

    #[test]
    fn initial_desired_tiles_contains_full_square_away_from_edges() {
        let adt_manager = sample_manager((10, 10), 1);
        let tiles = initial_desired_tiles(&adt_manager);

        assert_eq!(tiles.len(), 9);
        assert!(tiles.contains(&(9, 9)));
        assert!(tiles.contains(&(10, 10)));
        assert!(tiles.contains(&(11, 11)));
    }

    #[test]
    fn count_initial_tile_progress_counts_loaded_failed_and_pending() {
        let mut adt_manager = sample_manager((10, 10), 1);
        adt_manager.loaded.insert((10, 10), Entity::PLACEHOLDER);
        adt_manager.failed.insert((9, 10));
        adt_manager.pending.insert((10, 9));
        let desired_tiles = vec![(10, 10), (9, 10), (10, 9), (11, 11)];

        let progress = count_initial_tile_progress(&adt_manager, &desired_tiles);
        assert_eq!(progress, (1, 1, 1));
    }
}
