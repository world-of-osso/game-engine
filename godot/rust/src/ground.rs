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
        // Native WMO placement/floor resources are not loaded yet. This query covers
        // authored terrain only; it does not establish WMO/doodad collision parity.
        shared::ground::select_ground(feet.y, terrain, std::iter::empty())
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
