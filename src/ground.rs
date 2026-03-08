use bevy::asset::RenderAssetUsages;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::asset;
use crate::creature_display;
use crate::m2_scene;

const HERB_MODELS: &[&str] = &[
    "data/models/bush_peacebloom01.m2",
    "data/models/bush_silverleaf01.m2",
];

/// Load the grass BLP texture with repeat tiling and spawn the ground plane.
pub fn spawn_ground_plane(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    use std::path::PathBuf;
    let grass_path = asset::casc_resolver::ensure_texture(187126)
        .unwrap_or_else(|| PathBuf::from("data/textures/187126.blp"));
    let mut grass_image = asset::blp::load_blp_gpu_image(&grass_path).unwrap_or_else(|e| {
        eprintln!("{e}");
        generate_grass_texture()
    });
    grass_image.sampler =
        bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
            address_mode_u: bevy::image::ImageAddressMode::Repeat,
            address_mode_v: bevy::image::ImageAddressMode::Repeat,
            ..bevy::image::ImageSamplerDescriptor::linear()
        });
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(grass_image)),
        perceptual_roughness: 0.9,
        ..default()
    });
    let mut mesh = Plane3d::default().mesh().size(100.0, 100.0).build();
    scale_mesh_uvs(&mut mesh, 20.0);
    commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material)));
}

/// Multiply all UV coordinates in a mesh by the given factor for texture tiling.
pub fn scale_mesh_uvs(mesh: &mut Mesh, factor: f32) {
    use bevy::mesh::VertexAttributeValues;
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for uv in uvs.iter_mut() {
            uv[0] *= factor;
            uv[1] *= factor;
        }
    }
}

/// Generate a 64x64 procedural grass texture with color variation.
pub fn generate_grass_texture() -> Image {
    const SIZE: u32 = 64;
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let hash = ((x.wrapping_mul(7919) ^ y.wrapping_mul(6271)).wrapping_mul(2903)) % 256;
            let noise = hash as f32 / 255.0;
            let r = (0.25 + noise * 0.1) * 255.0;
            let g = (0.45 + noise * 0.15) * 255.0;
            let b = (0.15 + noise * 0.08) * 255.0;
            pixels.extend_from_slice(&[r as u8, g as u8, b as u8, 255]);
        }
    }
    Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Compute a deterministic scatter position from index. Returns None if too close to origin.
pub fn scatter_position(i: u32) -> Option<(f32, f32, u32, u32)> {
    let hash1 = (i.wrapping_mul(7919).wrapping_add(1301)) % 10000;
    let hash2 = (i.wrapping_mul(6271).wrapping_add(3571)) % 10000;
    let x = (hash1 as f32 / 10000.0 - 0.5) * 60.0;
    let z = (hash2 as f32 / 10000.0 - 0.5) * 60.0;
    if x * x + z * z < 9.0 {
        return None;
    }
    Some((x, z, hash1, hash2))
}

/// Scatter rocks and herb models across the ground.
pub fn spawn_ground_clutter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    spawn_rock_clutter(commands, meshes, materials);
    spawn_herb_clutter(commands, meshes, materials, images, inverse_bindposes, creature_display_map);
}

fn spawn_rock_clutter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let rock_mesh = meshes.add(Sphere::new(0.15).mesh().ico(2).unwrap());
    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.42, 0.38),
        perceptual_roughness: 0.95,
        ..default()
    });
    let dark_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.33, 0.30),
        perceptual_roughness: 0.95,
        ..default()
    });
    for i in 0u32..30 {
        let Some((x, z, hash1, hash2)) = scatter_position(i) else {
            continue;
        };
        if i % 3 == 0 {
            continue;
        }
        let (mat, scale) = if i % 2 == 0 {
            (&dark_mat, 0.6 + (hash2 % 80) as f32 / 100.0)
        } else {
            (&rock_mat, 0.5 + (hash1 % 100) as f32 / 100.0)
        };
        commands.spawn((
            Mesh3d(rock_mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(x, 0.0, z).with_scale(Vec3::new(1.0, scale, 1.0)),
        ));
    }
}

fn spawn_herb_clutter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
) {
    use std::path::Path;
    for i in 0u32..15 {
        let Some((x, z, hash1, _)) = scatter_position(i.wrapping_mul(3).wrapping_add(7)) else {
            continue;
        };
        let herb_path = Path::new(HERB_MODELS[(hash1 as usize) % HERB_MODELS.len()]);
        let yaw = (hash1 % 628) as f32 / 100.0;
        let transform = Transform::from_xyz(x, 0.0, z)
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(0.3));
        m2_scene::spawn_static_m2(
            commands,
            meshes,
            materials,
            images,
            inverse_bindposes,
            herb_path,
            transform,
            creature_display_map,
        );
    }
}
