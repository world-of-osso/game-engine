use std::path::PathBuf;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use super::super::terrain_background_parse::parse_tile_background;
use super::super::{AdtManager, DoodadLod, LoadedTileSpawnParams, ParsedTile, TileLoadResult};
use super::*;

#[derive(Resource)]
struct ParsedFixture(ParsedTile);

#[derive(Resource, Default)]
struct SpawnedRoot(Option<Entity>);

#[test]
fn no_terrain_textures_skips_image_assets_and_preserves_chunk_geometry() {
    let parsed = parse_fixture_tile();
    let expected_positions = first_chunk_positions(&parsed);
    let mut app = terrain_spawn_test_app(parsed);

    app.update();

    assert_eq!(app.world().resource::<Assets<Image>>().len(), 0);
    let root = app
        .world()
        .resource::<SpawnedRoot>()
        .0
        .expect("terrain root spawned");
    assert!(app.world().get_entity(root).is_ok());
    assert_eq!(
        app.world()
            .get::<Children>(root)
            .expect("terrain root children")
            .len(),
        256,
        "fixture terrain root must retain every terrain chunk"
    );
    assert_eq!(
        first_spawned_chunk_positions(&mut app),
        expected_positions,
        "flat terrain mode must retain the parsed terrain geometry"
    );
}

fn parse_fixture_tile() -> ParsedTile {
    let path = PathBuf::from("data/terrain/azeroth_32_48.adt");
    match parse_tile_background(
        "azeroth".into(),
        32,
        48,
        path,
        DoodadLod::Full,
        false,
        false,
    ) {
        TileLoadResult::Success(parsed) => *parsed,
        TileLoadResult::Failed { error, .. } => panic!("fixture terrain did not parse: {error}"),
    }
}

fn terrain_spawn_test_app(parsed: ParsedTile) -> App {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<M2EffectMaterial>>()
        .init_resource::<Assets<TerrainMaterial>>()
        .init_resource::<Assets<WaterMaterial>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
        .init_resource::<AdtManager>()
        .init_resource::<TerrainHeightmap>()
        .insert_resource(ParsedFixture(parsed))
        .init_resource::<SpawnedRoot>()
        .add_systems(Update, spawn_fixture_without_terrain_textures);
    app
}

fn spawn_fixture_without_terrain_textures(
    mut params: LoadedTileSpawnParams,
    fixture: Res<ParsedFixture>,
    mut spawned_root: ResMut<SpawnedRoot>,
) {
    let mut refs = SpawnRefs {
        commands: &mut params.commands,
        meshes: &mut params.meshes,
        materials: &mut params.materials,
        effect_materials: &mut params.effect_materials,
        terrain_materials: &mut params.terrain_mats,
        water_materials: &mut params.water_mats,
        images: &mut params.images,
        inverse_bp: &mut params.inverse_bp,
    };
    let (root, _) = spawn_parsed_tile(&mut refs, &params.heightmap, &fixture.0, false);
    spawned_root.0 = Some(root);
}

fn first_chunk_positions(parsed: &ParsedTile) -> Vec<[f32; 3]> {
    parsed.adt_data.chunks[0]
        .mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .expect("fixture terrain positions")
        .as_float3()
        .expect("three-dimensional terrain positions")
        .to_vec()
}

fn first_spawned_chunk_positions(app: &mut App) -> Vec<[f32; 3]> {
    let mut children_query = app.world_mut().query::<&Children>();
    let root = app
        .world()
        .resource::<SpawnedRoot>()
        .0
        .expect("terrain root spawned");
    let chunk = children_query
        .get(app.world(), root)
        .expect("terrain root children")[0];
    let mesh_handle = app
        .world()
        .get::<Mesh3d>(chunk)
        .expect("terrain chunk mesh")
        .0
        .clone();
    app.world()
        .resource::<Assets<Mesh>>()
        .get(&mesh_handle)
        .expect("spawned terrain mesh")
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .expect("spawned terrain positions")
        .as_float3()
        .expect("three-dimensional spawned positions")
        .to_vec()
}
