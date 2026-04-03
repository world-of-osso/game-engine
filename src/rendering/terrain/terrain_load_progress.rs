use crate::terrain::AdtManager;

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
            if (0..64).contains(&ty) && (0..64).contains(&tx) {
                tiles.push((ty as u32, tx as u32));
            }
        }
    }
    tiles
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
