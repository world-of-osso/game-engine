//! Player ground on the Stormwind Trade District WMO (docs/specs/wmo-floor-collision.md).

use std::time::Duration;

use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;

use super::portal_culling::{
    TRADE_DISTRICT_ROOT_FDID, load_trade_district, trade_district_placement,
};
use super::*;
use crate::camera::Player;
use crate::collision::{CharacterPhysics, CollisionPlugin, GroundProbe, WmoFloors, WorldGround};
use crate::game_state::GameState;
use crate::terrain_heightmap::TerrainHeightmap;

/// `azeroth_30_48.adt` (WDT MAID root FDID).
const TILE_30_48_ADT: &str = "data/terrain/777627.adt";
/// Inside the auction house (WoW -8820, 660), where the terrain lies 3.4 yd under the floor
/// (docs/wiki/investigations/stormwind-hilly-plaza.md).
const AUCTION_HOUSE_X: f32 = -8820.0;
const AUCTION_HOUSE_Z: f32 = -660.0;

fn trade_district_heightmap() -> TerrainHeightmap {
    let data = std::fs::read(TILE_30_48_ADT).expect("azeroth_30_48 root ADT in data/terrain");
    let adt = crate::asset::adt::load_adt_for_tile(&data, 30, 48).expect("parse azeroth_30_48");
    let mut heightmap = TerrainHeightmap::default();
    heightmap.insert_tile(30, 48, &adt);
    heightmap
}

/// The district's root spawned the way terrain streaming finishes a WMO root.
fn spawn_trade_district_floors(app: &mut App) {
    let placement = trade_district_placement();
    let (transform, root, groups) = load_trade_district();
    let floors = groups.iter().map(|group| group.collision.clone()).collect();
    let root_entity = app.world_mut().spawn(transform).id();
    let mut commands = app.world_mut().commands();
    finish_wmo_root(
        &mut commands,
        TRADE_DISTRICT_ROOT_FDID,
        root_entity,
        &root,
        &transform,
        floors,
        &placement,
    );
    app.world_mut().flush();
}

#[test]
fn auction_house_ground_is_its_floor_not_the_terrain_under_it() {
    let mut app = App::new();
    spawn_trade_district_floors(&mut app);
    let heightmap = trade_district_heightmap();
    let world = app.world_mut();
    let floors: Vec<_> = world
        .query::<&WmoFloors>()
        .iter(world)
        .map(|f| &f.0)
        .collect();
    let ground = WorldGround::from_parts(Some(&heightmap), floors);

    let terrain_y = heightmap
        .height_at(AUCTION_HOUSE_X, AUCTION_HOUSE_Z)
        .unwrap();
    let probe = ground.probe(Vec3::new(AUCTION_HOUSE_X, 98.1, AUCTION_HOUSE_Z));

    assert!((terrain_y - 94.7).abs() < 0.1, "terrain {terrain_y}");
    let GroundProbe::Supported(floor) = probe else {
        panic!("no ground on the auction house floor: {probe:?}");
    };
    assert_eq!(floor.surface, shared::ground::Surface::Wmo);
    assert!(
        (floor.height - 98.06).abs() < 0.05,
        "floor {}",
        floor.height
    );
}

/// A set-position onto the auction house floor puts the player a little above it; gravity must
/// land them on the floor, not drop them to the terrain 3.4 yd below.
#[test]
fn player_set_down_over_the_auction_house_floor_lands_on_it() {
    let mut app = App::new();
    app.add_plugins((bevy::time::TimePlugin, StatesPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            16,
        )))
        .insert_state(GameState::InWorld)
        .insert_resource(trade_district_heightmap())
        .add_plugins(CollisionPlugin);
    spawn_trade_district_floors(&mut app);
    let player = app
        .world_mut()
        .spawn((
            Player,
            CharacterPhysics::default(),
            Transform::from_xyz(AUCTION_HOUSE_X, 99.0, AUCTION_HOUSE_Z),
        ))
        .id();

    for _ in 0..120 {
        app.update();
    }

    let y = app.world().get::<Transform>(player).unwrap().translation.y;
    assert!((y - 98.06).abs() < 0.05, "player y {y}");
}
