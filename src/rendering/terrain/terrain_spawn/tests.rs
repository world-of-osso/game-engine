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
fn waterfall_tile_loads_shadow_maps_from_texture_companion() {
    let path = PathBuf::from("data/terrain/2703_31_36.adt");
    let terrain = load_and_parse_adt(&path).expect("waterfall split tile must load");
    assert_eq!(terrain.chunks.len(), 256);
    assert_eq!(
        terrain
            .chunks
            .iter()
            .filter(|chunk| chunk.shadow_map.is_some())
            .count(),
        227,
        "retain every authored waterfall shadow map"
    );

    let companion = std::fs::read("data/terrain/2703_31_36_tex0.adt").unwrap();
    let mut checked = 0;
    for (index, (_, chunk)) in adt::ChunkIter::new(&companion)
        .map(Result::unwrap)
        .filter(|(tag, _)| *tag == b"KNCM")
        .enumerate()
    {
        for (tag, payload) in adt::ChunkIter::new(chunk).map(Result::unwrap) {
            if tag == b"HSCM" {
                assert_eq!(
                    terrain.chunks[index]
                        .shadow_map
                        .as_ref()
                        .unwrap()
                        .as_slice(),
                    &payload[..512]
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 227);
}

#[test]
fn no_terrain_meshes_preserves_logical_tile_without_render_assets() {
    let mut app = terrain_spawn_test_app(parse_fixture_tile());
    app.world_mut().resource_mut::<AdtManager>().render_terrain = false;
    app.update();

    let root = app
        .world()
        .resource::<SpawnedRoot>()
        .0
        .expect("logical tile root");
    assert!(app.world().get_entity(root).is_ok());
    assert_eq!(app.world().get::<AdtTile>(root).unwrap()._tile_y, 32);
    assert_eq!(app.world().get::<AdtTile>(root).unwrap()._tile_x, 48);
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
    assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 0);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), 0);
    assert_eq!(
        app.world()
            .resource::<ParsedFixture>()
            .0
            .adt_data
            .height_grids
            .len(),
        256
    );
}

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
    let (root, _) = spawn_parsed_tile(
        &mut refs,
        &params.heightmap,
        &fixture.0,
        false,
        params.adt_manager.render_terrain,
    );
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
