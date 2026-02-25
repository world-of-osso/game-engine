use std::path::Path;

use bevy::prelude::*;

use crate::asset::adt;

/// Marker component for the ADT terrain root entity.
#[derive(Component)]
pub struct AdtTerrain;

/// Result of spawning an ADT: camera and ground position for placing models.
pub struct AdtSpawnResult {
    pub camera: Transform,
    pub center: Vec3,
}

/// Load an ADT file, build meshes, and spawn them into the Bevy scene.
pub fn spawn_adt(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    adt_path: &Path,
) -> Result<AdtSpawnResult, String> {
    let data = std::fs::read(adt_path)
        .map_err(|e| format!("Failed to read {}: {e}", adt_path.display()))?;
    let adt_data = adt::load_adt(&data)?;

    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.55, 0.25),
        perceptual_roughness: 0.9,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    spawn_chunk_entities(commands, meshes, &material, &adt_data);

    let result = compute_spawn_result(&adt_data);
    eprintln!(
        "Spawned ADT terrain: {} chunks from {}",
        adt_data.chunks.len(),
        adt_path.display(),
    );
    Ok(result)
}

fn spawn_chunk_entities(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: &Handle<StandardMaterial>,
    adt_data: &adt::AdtData,
) {
    let root = commands
        .spawn((AdtTerrain, Transform::default(), Visibility::default()))
        .id();

    for chunk in &adt_data.chunks {
        let mesh_handle = meshes.add(chunk.mesh.clone());
        let child = commands
            .spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(material.clone()),
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        commands.entity(root).add_child(child);
    }
}

fn compute_spawn_result(adt_data: &adt::AdtData) -> AdtSpawnResult {
    let center: Vec3 = adt_data.center_surface.into();
    let (min, max) = adt_data.bounds();
    let extent = (max - min).length();

    let eye = Vec3::new(center.x, center.y + extent * 0.5, center.z + extent * 0.3);
    let camera = Transform::from_translation(eye).looking_at(center, Vec3::Y);

    AdtSpawnResult { camera, center }
}
