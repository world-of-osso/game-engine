//! Ground queries for the terrain currently loaded by the native host.

use game_engine_core::player_physics_data::{
    GroundSample, GroundState, WALL_MARGIN, clamp_movement_to_walls, is_walkable_slope,
    validate_movement_slope,
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
    /// Wall interval between admitted feet and the existing low ray; support reach is Y.
    pub step_wall_blocked: &'a dyn Fn(Vec3, Vec3, f32) -> bool,
    /// Highest active replicated game-object collision face under these feet.
    pub game_object_floor: &'a dyn Fn(Vec3) -> Option<f32>,
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

    fn static_ground(&self, feet: Vec3, walkable_only: bool) -> Option<Ground> {
        let wmos = self
            .terrain
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
            );
        if walkable_only {
            shared::ground::ground_at(feet, self.terrain_height(feet), wmos)
        } else {
            shared::ground::collision_height(feet, self.terrain_height(feet), wmos)
        }
    }

    fn m2_ground(&self, feet: Vec3, walkable_only: bool) -> Option<Ground> {
        let walkable = self
            .terrain
            .map_wdt
            .as_ref()
            .into_iter()
            .flat_map(|map| &map.m2_floors)
            .chain(
                self.terrain
                    .parsed_tiles
                    .values()
                    .flat_map(|tile| &tile.m2_floors),
            )
            .filter_map(|tile| tile.floor_height(feet));
        let solid = self
            .terrain
            .map_wdt
            .as_ref()
            .into_iter()
            .flat_map(|map| &map.m2_solids)
            .chain(
                self.terrain
                    .parsed_tiles
                    .values()
                    .flat_map(|tile| &tile.m2_solids),
            )
            .filter_map(|tile| (!walkable_only).then(|| tile.solid_height(feet)).flatten());
        walkable
            .chain(solid)
            .max_by(f32::total_cmp)
            .map(|height| Ground {
                height,
                surface: Surface::M2,
            })
    }

    fn sample(&self, feet: Vec3) -> Option<Ground> {
        let object = (self.game_object_floor)(feet).map(|height| Ground {
            height,
            surface: Surface::M2,
        });
        self.static_ground(feet, false)
            .into_iter()
            .chain(self.m2_ground(feet, false))
            .chain(object)
            .max_by(|a, b| a.height.total_cmp(&b.height))
    }

    /// Describe the selected ground and its WMO group for an enabled diagnostic trace.
    pub fn trace_support(&self, feet: Vec3) -> Option<crate::movement_trace::Support> {
        let ground = self.sample(feet)?;
        let mut support = crate::movement_trace::Support {
            height: ground.height,
            source: match ground.surface {
                Surface::Terrain => "terrain",
                Surface::Wmo => "wmo",
                Surface::M2
                    if (self.game_object_floor)(feet)
                        .is_some_and(|height| (height - ground.height).abs() < 0.001) =>
                {
                    "game_object"
                }
                Surface::M2 => "m2",
            },
            wmo_placement: None,
            wmo_group: None,
        };
        if ground.surface != Surface::Wmo {
            return Some(support);
        }
        let global = self
            .terrain
            .map_wdt
            .as_ref()
            .and_then(|map| map.global_wmo.as_ref());
        let wmos = global
            .map(|wmo| (wmo.placement.unique_id, &wmo.collision))
            .into_iter()
            .chain(self.terrain.parsed_tiles.values().flat_map(|tile| {
                tile.wmo_floors
                    .iter()
                    .map(|(id, collision)| (*id, collision))
            }));
        for (id, wmo) in wmos {
            for (index, group) in wmo.groups().iter().enumerate() {
                let single =
                    shared::ground::WmoCollision::new(wmo.world_from_local(), vec![group.clone()]);
                let mut heights = Vec::new();
                single.collision_heights(
                    feet.x,
                    feet.z,
                    feet.y + STEP_UP_HEIGHT,
                    feet.y - FLOOR_SEARCH_DEPTH,
                    &mut heights,
                );
                if heights
                    .iter()
                    .any(|height| (height - ground.height).abs() < 0.001)
                {
                    support.wmo_placement = Some(id);
                    support.wmo_group = Some(index);
                    return Some(support);
                }
            }
        }
        Some(support)
    }

    /// Admit the destination floor before casting the wall ray at the walking height.
    /// A move to where no ground is within reach but ground lies above (a hillside, or a WMO
    /// floor over a terrain hole) stops, as an underwater cliff stops a swimmer: walking on
    /// would put the feet under the world's surface, with nothing to land on.
    pub fn validate_move(&self, current: Vec3, proposed: Vec3, snap: bool) -> Vec3 {
        let target_feet = proposed.with_y(current.y);
        let target = self.sample(target_feet);
        if snap && self.steep_solid_blocks_ascent(current, target_feet, target) {
            return current;
        }
        let candidate = self.apply_floor_rules(current, proposed, target, snap);
        let clamped = clamp_movement_to_walls(current, proposed, self.walls);
        let reached = clamped.x == proposed.x && clamped.z == proposed.z;
        if reached {
            return candidate;
        }
        if snap && self.step_past_wall_is_clear(current, proposed, candidate) {
            return candidate;
        }
        // A blocked step keeps the ordinary wall clamp. Never retain the height
        // of a landing that the player did not actually reach.
        let target = self.sample(clamped.with_y(current.y));
        self.apply_floor_rules(current, clamped, target, snap)
    }

    fn steep_solid_blocks_ascent(
        &self,
        current: Vec3,
        target: Vec3,
        solid: Option<Ground>,
    ) -> bool {
        let Some(solid) =
            solid.filter(|ground| matches!(ground.surface, Surface::Wmo | Surface::M2))
        else {
            return false;
        };
        if solid.surface == Surface::M2
            && (self.game_object_floor)(target).is_some_and(|height| height >= solid.height)
        {
            return false;
        }
        let walkable = self
            .static_ground(target, true)
            .into_iter()
            .chain(self.m2_ground(target, true))
            .max_by(|a, b| a.height.total_cmp(&b.height));
        if walkable.is_some_and(|ground| ground.height >= solid.height) {
            return false;
        }
        let rise = solid.height - current.y;
        let distance = (target - current).with_y(0.0).length();
        rise > 0.0 && !is_walkable_slope(rise, distance, MAX_SLOPE_ANGLE)
    }

    /// Only a wall-blocked move attempts a step: unobstructed ramps keep their
    /// existing low-ray corridor, including authored floor detail below that ray.
    fn step_past_wall_is_clear(&self, current: Vec3, proposed: Vec3, candidate: Vec3) -> bool {
        // The wall ray sees WALL_MARGIN beyond the requested displacement. Admit
        // support over that same reach, otherwise short frames stop before the lip
        // reaches the destination column. This probe never adds player movement.
        let movement = (proposed - current).with_y(0.0);
        let reach = proposed.with_y(current.y) + movement.normalize() * WALL_MARGIN;
        let target = self.sample(reach);
        let reached_support = self.apply_floor_rules(current, reach, target, true);
        let landing = if target.is_some() && reached_support.y > candidate.y {
            reached_support
        } else {
            candidate
        };
        if landing.y <= current.y {
            return false;
        }
        let raised = current.with_y(landing.y);
        let destination = proposed.with_y(landing.y);
        let across = clamp_movement_to_walls(raised, destination, self.walls);
        let reached = across.x == destination.x && across.z == destination.z;
        reached && !(self.step_wall_blocked)(raised, landing, current.y + STEP_UP_HEIGHT)
    }

    fn apply_floor_rules(
        &self,
        current: Vec3,
        proposed: Vec3,
        target: Option<Ground>,
        snap: bool,
    ) -> Vec3 {
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
#[path = "ground_step_tests.rs"]
mod step_tests;

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{Duration, Instant},
    };

    use super::*;

    #[test]
    fn solid_m2_support_prevents_fall_and_blocks_steep_ascent() {
        use crate::terrain::NativeTerrainTile;
        use shared::los::LosTile;
        let mut terrain = StreamedTerrain::new(PathBuf::new());
        let floor = LosTile::build(
            32,
            32,
            vec![
                Vec3::new(-4.0, 12.0, -4.0),
                Vec3::new(4.0, 12.0, -4.0),
                Vec3::new(0.0, 12.0, 4.0),
            ],
            vec![[0, 1, 2]],
        );
        let solid = LosTile::build(
            32,
            32,
            vec![
                Vec3::new(-4.0, 8.88, -4.0),
                Vec3::new(4.0, 19.12, -4.0),
                Vec3::new(0.0, 14.0, 4.0),
            ],
            vec![[0, 1, 2]],
        );
        terrain.parsed_tiles.insert(
            (32, 32),
            NativeTerrainTile {
                root_path: PathBuf::new(),
                tex_path: None,
                obj_path: None,
                root: game_engine_core::adt::Root {
                    chunks: Vec::new(),
                    height_grids: Vec::new(),
                    center_surface: [0.0; 3],
                    chunk_positions: vec![[0.0; 3]],
                    blend_mesh: None,
                    flight_bounds: None,
                    water: None,
                    water_error: None,
                },
                tex: None,
                obj: None,
                textures: Default::default(),
                chunk_surfaces: Default::default(),
                wmo_floors: Vec::new(),
                m2_floors: vec![floor],
                m2_solids: vec![solid],
                wmo_surfaces: Vec::new(),
                liquid_materials: Default::default(),
            },
        );
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
            step_wall_blocked: &|_, _, _| false,
            game_object_floor: &|_| None,
        };
        let feet = Vec3::new(0.0, 14.0, 0.0);
        assert_eq!(ground.probe(feet), GroundState::Supported(14.0));
        assert_eq!(
            ground.probe(feet.with_y(12.0)),
            GroundState::Supported(12.0)
        );
        let mut movement = crate::gameplay::PlayerMovement::default();
        let frame = crate::gameplay::MovementFrame {
            direction: [1.0, 0.0, 0.0],
            speed: 7.0,
            vertical: 0.0,
        };
        let next = movement.predict(feet, frame, false, &ground, &mut |_, to, _| to, 1.0 / 60.0);
        assert_eq!(
            next, feet,
            "steep M2 ascent must remain blocked and supported"
        );
    }

    fn load_floor_map(map: &str, tile: (u32, u32)) -> StreamedTerrain {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut terrain = StreamedTerrain::new(data_root);
        terrain.request_map(map.into(), tile).unwrap();
        let deadline = Instant::now() + Duration::from_secs(90);
        while !terrain.parsed_tiles.contains_key(&tile) {
            terrain.poll().expect("terrain worker alive");
            assert!(
                Instant::now() < deadline,
                "{map}: {:?}",
                terrain
                    .state()
                    .failures
                    .iter()
                    .map(|f| &f.error)
                    .collect::<Vec<_>>()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        terrain
    }

    fn stand_and_walk(terrain: &StreamedTerrain, start: Vec3, expected_height: f32, distance: f32) {
        let ground = TerrainGround {
            terrain,
            walls: &|_, _, _| None,
            step_wall_blocked: &|_, _, _| false,
            game_object_floor: &|_| None,
        };
        match ground.probe(start) {
            GroundState::Supported(height) => {
                assert!((height - expected_height).abs() < 1.0, "M2 floor: {height}")
            }
            other => panic!("M2 floor unsupported: {other:?}"),
        }
        let mut movement = crate::gameplay::PlayerMovement::default();
        let mut feet = start;
        for _ in 0..300 {
            feet = movement.predict(
                feet,
                crate::gameplay::MovementFrame {
                    direction: [0.0; 3],
                    speed: 0.0,
                    vertical: 0.0,
                },
                false,
                &ground,
                &mut |_, to, _| to,
                1.0 / 60.0,
            );
        }
        assert!(
            (feet.y - expected_height).abs() < 1.0,
            "fell while standing: {feet}"
        );
        for _ in 0..60 {
            let frame = crate::gameplay::MovementFrame {
                direction: [1.0, 0.0, 0.0],
                speed: distance,
                vertical: 0.0,
            };
            feet = movement.predict(feet, frame, false, &ground, &mut |_, to, _| to, 1.0 / 60.0);
        }
        assert!(feet.x > start.x + distance * 0.9, "failed to walk: {feet}");
        assert!(
            (feet.y - expected_height).abs() < 2.0,
            "fell while walking: {feet}"
        );
    }

    #[test]
    fn arena_cage_authored_live_trapdoor_supports_walk_then_opens_into_a_drop() {
        use std::cell::Cell;
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
        let model = game_engine_core::m2::parse_model(
            &std::fs::read(data.join("6339134.m2")).unwrap(),
            &std::fs::read(data.join("6339702.skin")).unwrap(),
        )
        .unwrap();
        // TDB1210 entry505683/guid30001717, display99831, exact source transform.
        let transform = glam::Affine3A::from_scale_rotation_translation(
            Vec3::ONE,
            glam::Quat::from_rotation_y(1.5708),
            Vec3::new(443.59, -24.8806, -321.262),
        );
        let faces = crate::terrain::doodad_collision::collision_triangles(
            model.collision.as_ref().unwrap(),
        );
        let vertices: Vec<_> = faces
            .iter()
            .flatten()
            .map(|point| transform.transform_point3(*point))
            .collect();
        let triangles = (0..faces.len() as u32)
            .map(|index| [index * 3, index * 3 + 1, index * 3 + 2])
            .collect();
        let tile = shared::los::LosTile::build(32, 31, vertices, triangles);
        let start = Vec3::new(443.59, -24.5936, -321.262);
        let terrain = load_floor_map("2759", bevy_to_tile_coords(start.x, start.z));
        let closed = Cell::new(true);
        let live_floor = |feet| {
            if closed.get() {
                tile.floor_height(feet)
            } else {
                None
            }
        };
        assert!(
            live_floor(Vec3::new(443.59, -24.5936, -321.262)).is_some(),
            "authored dedicated collision supplies a floor"
        );
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
            step_wall_blocked: &|_, _, _| false,
            game_object_floor: &live_floor,
        };
        let mut movement = crate::gameplay::PlayerMovement::default();
        let mut feet = start;
        for _ in 0..60 {
            feet = movement.predict(
                feet,
                crate::gameplay::MovementFrame {
                    direction: [0.0, 0.0, -1.0],
                    speed: 2.0,
                    vertical: 0.0,
                },
                false,
                &ground,
                &mut |_, to, _| to,
                1.0 / 60.0,
            );
        }
        assert!(
            feet.y > -30.0,
            "closed authored trapdoor must support the player: {feet}"
        );
        closed.set(false);
        for _ in 0..120 {
            feet = movement.predict(
                feet,
                crate::gameplay::MovementFrame {
                    direction: [0.0; 3],
                    speed: 0.0,
                    vertical: 0.0,
                },
                false,
                &ground,
                &mut |_, to, _| to,
                1.0 / 60.0,
            );
        }
        assert!(
            feet.y < -30.0,
            "opening removes only the live floor; gravity reaches the arena: {feet}"
        );
    }

    #[test]
    fn m2_floor_eots_real_start_stands_and_walks_before_render_spawns() {
        let terrain = load_floor_map("NetherstormBG", (29, 27));
        stand_and_walk(&terrain, Vec3::new(2523.76, 1247.7, -1596.13), 1247.7, 3.0);
    }

    #[test]
    fn m2_floor_northshire_stonepyre_stands_and_walks_before_render_spawns() {
        let terrain = load_floor_map("Azeroth", (32, 48));
        stand_and_walk(&terrain, Vec3::new(-8922.681, 87.57, 139.566), 87.57, 0.8);
    }

    #[test]
    fn stockade_worker_floor_support_respects_bounds_and_reset() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut terrain = StreamedTerrain::new(data_root.clone());
        // (103, 76) is inside Jail18, under barrel01.m2 (FDID 199563), MODD 523
        // of WMO 108631's selected/default set 0. Keep the bare WMO floor probe
        // off its authored collision hull, whose top is -33.924946.
        let inside = Vec3::new(104.5, -34.5, -77.3);
        let barrel = Vec3::new(103.0, -34.5, -76.0);
        assert_eq!(
            TerrainGround {
                terrain: &terrain,
                walls: &|_, _, _| None,
                step_wall_blocked: &|_, _, _| false,
                game_object_floor: &|_| None,
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
            step_wall_blocked: &|_, _, _| false,
            game_object_floor: &|_| None,
        };
        match ground.probe(inside) {
            GroundState::Supported(height) => assert!((height + 34.9).abs() < 0.2, "{height}"),
            other => panic!("Stockade floor unsupported: {other:?}"),
        }
        match ground.probe(barrel) {
            GroundState::Supported(height) => {
                assert!((height + 33.924946).abs() < 0.01, "{height}")
            }
            other => panic!("Stockade barrel floor unsupported: {other:?}"),
        }
        // These columns have no terrain, WMO or M2 face with the full bake.
        for (x, y) in [(97.0, 76.0), (99.0, 77.0), (101.0, 78.0), (74.0, -16.0)] {
            for z in [-40.0, -35.0, -30.0, -26.0, -20.0, -10.0, 0.0, 10.0] {
                assert_eq!(
                    ground.probe(Vec3::new(x, z, -y)),
                    GroundState::Unsupported,
                    "({x}, {y}, {z})"
                );
            }
        }
        terrain.reset().unwrap();
        assert_eq!(
            TerrainGround {
                terrain: &terrain,
                walls: &|_, _, _| None,
                step_wall_blocked: &|_, _, _| false,
                game_object_floor: &|_| None,
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
            step_wall_blocked: &|_, _, _| false,
            game_object_floor: &|_| None,
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
            feet = movement.predict(feet, frame, false, &ground, &mut |_, to, _| to, 1.0 / 60.0);
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
