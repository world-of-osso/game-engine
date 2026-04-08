use bevy::prelude::*;
use shared::protocol::WorldMapStateUpdate;

use crate::world_map_data::WorldMapState;

pub struct WorldMapPlugin;

impl Plugin for WorldMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldMapState>();
    }
}

pub fn apply_world_map_state_update(world_map: &mut WorldMapState, update: WorldMapStateUpdate) {
    if let Some(snapshot) = update.snapshot {
        world_map.fog.explored_zones = snapshot.discovered_zone_ids;
        world_map.fog.explored_zones.sort_unstable();
        world_map.fog.explored_zones.dedup();
    }
}

pub fn reset_runtime(world_map: &mut WorldMapState) {
    *world_map = WorldMapState::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_map_state_update_populates_explored_zones() {
        let mut world_map = WorldMapState::default();

        apply_world_map_state_update(
            &mut world_map,
            WorldMapStateUpdate {
                snapshot: Some(shared::protocol::WorldMapSnapshot {
                    discovered_zone_ids: vec![1519, 12, 12],
                }),
                message: Some("world map discovery updated".into()),
                error: None,
            },
        );

        assert_eq!(world_map.fog.explored_zones, vec![12, 1519]);
    }
}
