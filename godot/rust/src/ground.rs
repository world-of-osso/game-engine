//! Ground queries for the terrain currently loaded by the native host.

use game_engine_core::player_physics_data::{
    GroundSample, GroundState, clamp_movement_to_walls, validate_movement_slope,
};
use game_engine_core::terrain_height_data::bevy_to_tile_coords;
use glam::Vec3;
use shared::ground::{FLOOR_SEARCH_DEPTH, Ground, STEP_UP_HEIGHT, Surface};
use shared::movement::{MAX_SLOPE_ANGLE, is_swimming};

use crate::terrain::streaming::StreamedTerrain;

pub(crate) struct TerrainGround<'a> {
    pub terrain: &'a StreamedTerrain,
    /// Distance to the first WMO wall on a ray `(origin, direction, length)`.
    pub walls: &'a dyn Fn(Vec3, Vec3, f32) -> Option<f32>,
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

    /// The terrain height under `feet`; a map made of one global WMO has none.
    fn terrain_height(&self, feet: Vec3) -> Option<f32> {
        if self.is_global_wmo() {
            None
        } else {
            self.terrain.height_at(feet.x, feet.z)
        }
    }

    /// Whether terrain, or a WMO floor within `FLOOR_SEARCH_DEPTH`, lies over `feet`.
    fn covered(&self, feet: Vec3) -> bool {
        self.terrain_height(feet)
            .is_some_and(|height| height > feet.y)
            || self
                .sample(feet + Vec3::Y * FLOOR_SEARCH_DEPTH)
                .is_some_and(|ground| ground.height > feet.y)
    }

    fn sample(&self, feet: Vec3) -> Option<Ground> {
        shared::ground::ground_at(
            feet,
            self.terrain_height(feet),
            self.terrain
                .map_wdt
                .as_ref()
                .and_then(|map| map.global_wmo.as_ref())
                .map(|wmo| &wmo.collision)
                .into_iter()
                .chain(
                    self.terrain
                        .parsed_tiles
                        .values()
                        .flat_map(|tile| tile.wmo_floors.iter().map(|(_, wmo)| wmo)),
                ),
        )
    }

    /// The original order: stop short of a WMO wall, then apply the slope and step rules.
    /// A move to where no ground is within reach but ground lies above (a hillside, or a WMO
    /// floor over a terrain hole) stops, as an underwater cliff stops a swimmer: walking on
    /// would put the feet under the world's surface, with nothing to land on.
    pub fn validate_move(&self, current: Vec3, proposed: Vec3, snap: bool) -> Vec3 {
        let proposed = clamp_movement_to_walls(current, proposed, self.walls);
        let target = self.sample(proposed.with_y(current.y));
        if target.is_none() && self.covered(proposed.with_y(current.y)) {
            return current;
        }
        validate_movement_slope(
            current,
            proposed,
            self.sample(current).map(ground_sample),
            target.map(ground_sample),
            snap,
            MAX_SLOPE_ANGLE,
            STEP_UP_HEIGHT,
        )
    }

    /// Swimming where the water stands `SWIM_DEPTH` over the feet, above the ground.
    pub fn swimming(&self, feet: Vec3) -> bool {
        self.sample(feet)
            .is_some_and(|ground| is_swimming(feet.y, ground.height, self.water_surface(feet)))
    }

    pub fn water_surface(&self, feet: Vec3) -> Option<f32> {
        self.terrain.water_surface_at(feet.x, feet.z)
    }

    /// A level swim move: stop short of a WMO wall and of ground beyond step reach (an
    /// underwater cliff); ground rising over the feet lifts the swimmer onto it (the shore).
    pub fn validate_swim_move(&self, current: Vec3, proposed: Vec3) -> Vec3 {
        let proposed = clamp_movement_to_walls(current, proposed, self.walls);
        match self.sample(proposed) {
            Some(ground) => proposed.with_y(proposed.y.max(ground.height)),
            None if self.terrain.height_at(proposed.x, proposed.z).is_some() => current,
            None => proposed,
        }
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
        let mut terrain = StreamedTerrain::new(data_root.clone());
        let inside = Vec3::new(103.0, -34.5, -76.0);
        let outside = Vec3::new(99.0, -34.5, -77.0);
        assert_eq!(
            TerrainGround {
                terrain: &terrain,
                walls: &|_, _, _| None,
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
            walls: &|_, _, _| None,
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
                walls: &|_, _, _| None,
            }
            .probe(inside),
            GroundState::Unloaded
        );
    }

    /// Stormwind's Stockade entrance (`sw_magicdistrict` 321999 on azeroth_30_48): the room
    /// floor at 97.63 and the stairwell down to the Stockade door are WMO floors, over flat
    /// terrain at 86.21. They are ground as soon as the tile is parsed, before any WMO node
    /// of the tile is spawned (the in-world object queue takes minutes).
    #[test]
    fn stockade_entrance_ground_is_the_placed_wmo_floor_and_stairs() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut terrain = StreamedTerrain::new(data_root.clone());
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
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
        };
        // Where the server stood the player: WoW (-8785.926, 820.665, 97.652).
        let room = Vec3::new(-8785.926, 97.652, -820.665);
        match ground.probe(room) {
            GroundState::Supported(height) => assert!((height - 97.63).abs() < 0.05, "{height}"),
            other => panic!("room floor unsupported: {other:?}"),
        }
        // The Stockade exit arrival (world_safe_locs 3618) on the doorway floor at 88.
        match ground.probe(Vec3::new(-8766.11, 88.0, -845.5)) {
            GroundState::Supported(height) => assert!((height - 88.0).abs() < 0.3, "{height}"),
            other => panic!("doorway floor unsupported: {other:?}"),
        }
        // Run east down the stairwell from its top step (WoW y 836) for two seconds.
        let mut movement = crate::gameplay::PlayerMovement::default();
        let mut feet = Vec3::new(-8786.0, 96.1, -836.0);
        let mut lowest = feet.y;
        for _ in 0..120 {
            let frame = crate::gameplay::MovementFrame {
                direction: [1.0, 0.0, 0.0],
                speed: shared::movement::RUN_SPEED,
                vertical: 0.0,
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
