use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::ecs::system::RunSystemOnce;
use bevy::mesh::{Indices, VertexAttributeValues, skinning::SkinnedMeshInverseBindposes};
use bevy::prelude::*;

use super::*;

const FIXTURE_MODEL: &str = "data/models/1011653.m2";
const FIXTURE_SKIN: &str = "data/models/101165300.skin";
const FIXTURE_SKELETON: &str = "data/models/1011653.skel";
static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct ModelFixture {
    directory: PathBuf,
    model_path: PathBuf,
}

impl ModelFixture {
    fn copy() -> Self {
        let nonce = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "game-engine-npc-model-cache-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("create fixture directory");
        let model_path = directory.join("npc.m2");
        fs::copy(FIXTURE_MODEL, &model_path).expect("copy M2 fixture");
        fs::copy(FIXTURE_SKIN, directory.join("npc00.skin")).expect("copy skin fixture");
        fs::copy(FIXTURE_SKELETON, directory.join("npc.skel")).expect("copy skeleton fixture");
        Self {
            directory,
            model_path,
        }
    }
}

impl Drop for ModelFixture {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.directory) {
            eprintln!(
                "failed to remove M2 fixture directory {}: {error}",
                self.directory.display()
            );
        }
    }
}

#[derive(Resource)]
struct SpawnPath(PathBuf);

fn spawn_fixture_model(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inverse_bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    path: Res<SpawnPath>,
) -> Entity {
    let root = commands.spawn_empty().id();
    let mut assets = SpawnAssets {
        meshes: &mut meshes,
        materials: &mut materials,
        effect_materials: &mut effect_materials,
        skybox_materials: None,
        images: &mut images,
        inverse_bindposes: &mut inverse_bindposes,
    };
    assert!(
        spawn_m2_on_entity(&mut commands, &mut assets, &path.0, root, &[0, 0, 0]),
        "spawn M2 fixture {}",
        path.0.display()
    );
    root
}

fn mesh_geometry(world: &World, root: Entity) -> Vec<(Vec<[f32; 3]>, Option<Indices>)> {
    let mut entities = vec![root];
    let mut geometry = Vec::new();
    while let Some(entity) = entities.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            entities.extend(children.iter());
        }
        let Some(mesh_handle) = world.get::<Mesh3d>(entity) else {
            continue;
        };
        let meshes = world.resource::<Assets<Mesh>>();
        let mesh = meshes.get(mesh_handle).expect("spawned mesh asset");
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("spawned mesh must have Float32x3 positions");
        };
        geometry.push((positions.clone(), mesh.indices().cloned()));
    }
    geometry
        .sort_by_key(|(positions, indices)| (positions.len(), indices.as_ref().map(Indices::len)));
    geometry
}

#[test]
fn repeated_npc_model_spawn_reuses_loaded_model_after_source_is_removed() {
    let fixture = ModelFixture::copy();
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<M2EffectMaterial>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
        .insert_resource(SpawnPath(fixture.model_path.clone()));

    let first_root = app
        .world_mut()
        .run_system_once(spawn_fixture_model)
        .expect("first model spawn system");
    app.update();
    let first_geometry = mesh_geometry(app.world(), first_root);
    assert!(
        !first_geometry.is_empty(),
        "first spawn should emit mesh geometry"
    );

    fs::remove_file(&fixture.model_path).expect("remove source M2 after first spawn");
    let second_root = app
        .world_mut()
        .run_system_once(spawn_fixture_model)
        .expect("second model spawn system");
    app.update();

    assert_ne!(
        first_root, second_root,
        "each NPC needs its own root entity"
    );
    assert_eq!(
        mesh_geometry(app.world(), second_root),
        first_geometry,
        "second NPC should reuse the already loaded M2 after its source file is removed",
    );
}
