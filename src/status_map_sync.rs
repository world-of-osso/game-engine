use bevy::prelude::Transform;

use game_engine::status::MapStatusSnapshot;

pub fn fill_map_status_snapshot(
    snapshot: &mut MapStatusSnapshot,
    zone_id: u32,
    player_transform: Option<&Transform>,
) {
    snapshot.zone_id = zone_id;
    if let Some(transform) = player_transform {
        snapshot.player_x = transform.translation.x;
        snapshot.player_z = transform.translation.z;
    }
}
