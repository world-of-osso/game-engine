//! Ground queries for the terrain currently loaded by the native host.

use game_engine_core::player_physics_data::{GroundSample, GroundState, validate_movement_slope};
use game_engine_core::terrain_height_data::bevy_to_tile_coords;
use glam::Vec3;
use shared::ground::{Ground, STEP_UP_HEIGHT, Surface};
use shared::movement::MAX_SLOPE_ANGLE;

use crate::terrain::streaming::StreamedTerrain;

pub(crate) struct TerrainGround<'a> {
    pub terrain: &'a StreamedTerrain,
}

impl TerrainGround<'_> {
    pub fn probe(&self, feet: Vec3) -> GroundState {
        if !self.is_global_wmo()
            && !self
                .terrain
                .parsed_tiles
                .contains_key(&bevy_to_tile_coords(feet.x, feet.z))
        {
            return GroundState::Unloaded;
        }
        match self.sample(feet) {
            Some(ground) => GroundState::Supported(ground.height),
            None => GroundState::Unsupported,
        }
    }

    fn is_global_wmo(&self) -> bool {
        self.terrain
            .map_wdt
            .as_ref()
            .is_some_and(|map| map.global_wmo.is_some())
    }

    fn sample(&self, feet: Vec3) -> Option<Ground> {
        let terrain = if self.is_global_wmo() {
            None
        } else {
            self.terrain.height_at(feet.x, feet.z)
        };
        shared::ground::ground_at(
            feet,
            terrain,
            self.terrain
                .map_wdt
                .as_ref()
                .and_then(|map| map.global_wmo.as_ref())
                .map(|wmo| &wmo.collision),
        )
    }

    pub fn validate_move(&self, current: Vec3, proposed: Vec3, snap: bool) -> Vec3 {
        validate_movement_slope(
            current,
            proposed,
            self.sample(current).map(ground_sample),
            self.sample(proposed.with_y(current.y)).map(ground_sample),
            snap,
            MAX_SLOPE_ANGLE,
            STEP_UP_HEIGHT,
        )
    }

    pub fn swimming(&self, feet: Vec3) -> bool {
        let Some(ground) = self.sample(feet) else {
            return false;
        };
        self.terrain
            .water_surface_at(feet.x, feet.z)
            .is_some_and(|water| water > ground.height && water - ground.height >= 1.25)
    }
}

fn ground_sample(ground: Ground) -> GroundSample {
    GroundSample {
        height: ground.height,
        is_terrain: ground.surface == Surface::Terrain,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{Duration, Instant},
    };

    use super::*;

    #[test]
    fn stockade_worker_floor_support_respects_bounds_and_reset() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut terrain = StreamedTerrain::new(data_root.clone(), data_root.join("cache"));
        let inside = Vec3::new(103.0, -34.5, -76.0);
        let outside = Vec3::new(99.0, -34.5, -77.0);
        assert_eq!(
            TerrainGround { terrain: &terrain }.probe(inside),
            GroundState::Unloaded
        );
        terrain
            .request_map("stormwindjail".into(), (32, 32))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        while terrain.state().pending_map {
            terrain.poll().expect("Stockade worker alive");
            assert!(
                Instant::now() < deadline,
                "Stockade worker timed out: {:?}",
                terrain.state().map_error
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            terrain.state().map_error.is_none(),
            "{:?}",
            terrain.state().map_error
        );
        let ground = TerrainGround { terrain: &terrain };
        match ground.probe(inside) {
            GroundState::Supported(height) => assert!((height + 34.9).abs() < 0.2, "{height}"),
            other => panic!("Stockade floor unsupported: {other:?}"),
        }
        assert_eq!(ground.probe(outside), GroundState::Unsupported);
        terrain.reset().unwrap();
        assert_eq!(
            TerrainGround { terrain: &terrain }.probe(inside),
            GroundState::Unloaded
        );
    }
}
