//! Ground queries for the terrain currently loaded by the native host.

use game_engine_core::player_physics_data::{GroundSample, GroundState, validate_movement_slope};
use game_engine_core::terrain_height_data::bevy_to_tile_coords;
use glam::Vec3;
use shared::ground::{Ground, STEP_UP_HEIGHT, Surface, WmoCollision};
use shared::movement::MAX_SLOPE_ANGLE;

use crate::terrain::streaming::StreamedTerrain;

pub(crate) struct TerrainGround<'a> {
    pub terrain: &'a StreamedTerrain,
    /// Floors of the ADT-placed WMOs spawned so far.
    pub wmos: &'a [WmoCollision],
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
                .map(|wmo| &wmo.collision)
                .into_iter()
                .chain(self.wmos),
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
            TerrainGround {
                terrain: &terrain,
                wmos: &[],
            }
            .probe(inside),
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
        let ground = TerrainGround {
            terrain: &terrain,
            wmos: &[],
        };
        match ground.probe(inside) {
            GroundState::Supported(height) => assert!((height + 34.9).abs() < 0.2, "{height}"),
            other => panic!("Stockade floor unsupported: {other:?}"),
        }
        assert_eq!(ground.probe(outside), GroundState::Unsupported);
        terrain.reset().unwrap();
        assert_eq!(
            TerrainGround {
                terrain: &terrain,
                wmos: &[],
            }
            .probe(inside),
            GroundState::Unloaded
        );
    }

    /// Stormwind's Stockade entrance (`sw_magicdistrict` 321999 on azeroth_30_48): the room
    /// floor at 97.63 and the stairwell down to the Stockade door are WMO floors, over flat
    /// terrain at 86.21.
    #[test]
    fn stockade_entrance_ground_is_the_placed_wmo_floor_and_stairs() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let cache_root = data_root.join("cache");
        let mut terrain = StreamedTerrain::new(data_root.clone(), cache_root.clone());
        terrain.request_map("azeroth".into(), (30, 48)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        while !terrain.parsed_tiles.contains_key(&(30, 48)) {
            terrain.poll().expect("terrain worker alive");
            assert!(
                Instant::now() < deadline,
                "azeroth_30_48 timed out: {:?}",
                terrain.state().map_error
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let placement = terrain.parsed_tiles[&(30, 48)]
            .obj
            .as_ref()
            .expect("azeroth_30_48_obj0")
            .wmos
            .iter()
            .find(|placement| placement.fdid == Some(321_999))
            .expect("sw_magicdistrict MODF")
            .clone();
        let resolver = crate::assets::creature::local_resolver(&data_root, &cache_root);
        let asset = crate::wmo::assets::read_placement(&resolver, &data_root, &placement).unwrap();
        let floors = [crate::wmo::placement::adt_wmo_collision(
            &placement,
            (30, 48),
            &asset,
        )];
        let ground = TerrainGround {
            terrain: &terrain,
            wmos: &floors,
        };
        // Where the server stood the player: WoW (-8785.926, 820.665, 97.652).
        let room = Vec3::new(-8785.926, 97.652, -820.665);
        match ground.probe(room) {
            GroundState::Supported(height) => assert!((height - 97.63).abs() < 0.05, "{height}"),
            other => panic!("room floor unsupported: {other:?}"),
        }
        // Run east down the stairwell from its top step (WoW y 836) for two seconds.
        let mut movement = crate::gameplay::PlayerMovement::default();
        let mut feet = Vec3::new(-8786.0, 96.1, -836.0);
        let mut lowest = feet.y;
        for _ in 0..120 {
            let frame = crate::gameplay::MovementFrame {
                direction: [1.0, 0.0, 0.0],
                speed: shared::movement::RUN_SPEED,
            };
            feet = movement.predict(feet, frame, false, &ground, 1.0 / 60.0);
            lowest = lowest.min(feet.y);
        }
        assert!(feet.x > -8773.0, "{feet}");
        assert!(
            (90.5..92.5).contains(&feet.y),
            "not on the lower steps: {feet}"
        );
        assert!(lowest > 90.5, "fell below the stairs: {lowest}");
    }
}
