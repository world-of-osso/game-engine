use std::f32::consts::PI;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin};
use bevy::prelude::*;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use wow_engine::ipc::IpcPlugin;

mod animation;
mod asset;
mod camera;

use animation::{AnimationPlugin, BonePivot, M2AnimData, M2AnimPlayer};
use camera::{CharacterFacing, MovementState, Player, WowCamera, WowCameraPlugin};

const DEFAULT_M2: &str =
    "/syncthing/Sync/Projects/wow/reference-addons.new/TomTom/Images/Arrow.m2";

#[derive(Resource)]
struct DumpTreeFlag;

#[derive(Resource)]
struct ScreenshotRequest {
    output: PathBuf,
    frames_remaining: u32,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let screenshot = parse_screenshot_args(&args);
    let dump_tree = args.iter().any(|a| a == "--dump-tree");

    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(IpcPlugin)
        .add_plugins(WowCameraPlugin)
        .add_plugins(AnimationPlugin)
        .add_plugins(FpsOverlayPlugin {
            config: FpsOverlayConfig {
                refresh_interval: Duration::from_millis(500),
                ..default()
            },
        })
        .add_systems(Startup, setup);

    if dump_tree {
        app.insert_resource(DumpTreeFlag);
        app.add_systems(PostStartup, dump_tree_and_exit);
    }

    if let Some(req) = screenshot {
        app.insert_resource(req);
        app.add_systems(Update, take_screenshot);
    }

    app.run();
}

/// Parse `screenshot <output> [model]` from args. Returns None if not a screenshot command.
fn parse_screenshot_args(args: &[String]) -> Option<ScreenshotRequest> {
    if args.first().map(|s| s.as_str()) != Some("screenshot") {
        return None;
    }
    let output = args.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("screenshot.webp"));
    Some(ScreenshotRequest { output, frames_remaining: 3 })
}

/// Find the model path from CLI args.
/// Normal: `wow-engine [model.m2] [--flags]`
/// Screenshot: `wow-engine screenshot [output.webp] [model.m2]`
fn parse_model_path() -> PathBuf {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("screenshot") {
        // Third arg (index 2) is the model path
        args.get(2).map(PathBuf::from)
    } else {
        args.iter().find(|a| !a.starts_with("--")).map(PathBuf::from)
    }
    .unwrap_or_else(|| PathBuf::from(DEFAULT_M2))
}

fn take_screenshot(mut commands: Commands, req: Option<ResMut<ScreenshotRequest>>) {
    let Some(mut req) = req else { return };
    if req.frames_remaining > 0 {
        req.frames_remaining -= 1;
        return;
    }
    commands.remove_resource::<ScreenshotRequest>();
    let output = req.output.clone();
    commands
        .spawn(Screenshot::primary_window())
        .observe(move |trigger: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            save_screenshot(&trigger.image, &output);
            exit.write(AppExit::Success);
        });
}

fn save_screenshot(img: &bevy::image::Image, output: &PathBuf) {
    let Some(data) = img.data.as_ref() else {
        eprintln!("Screenshot has no pixel data");
        return;
    };
    let size = img.size();
    let encoder = webp::Encoder::from_rgba(data, size.x, size.y);
    let webp_data = encoder.encode(15.0);
    std::fs::write(output, &*webp_data)
        .unwrap_or_else(|e| eprintln!("Failed to write {}: {e}", output.display()));
    println!("Saved {} ({} bytes)", output.display(), webp_data.len());
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut skinned_mesh_inverse_bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    spawn_scene_environment(&mut commands, &mut meshes, &mut materials, &mut images);
    let m2_path = parse_model_path();
    spawn_m2_model(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        &mut skinned_mesh_inverse_bindposes,
        &m2_path,
    );

    // Static reference object (chest) so you can see movement relative to the world
    let chest_path = Path::new("data/models/chest01.m2");
    if chest_path.exists() {
        spawn_static_m2(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
            &mut skinned_mesh_inverse_bindposes,
            chest_path,
            Transform::from_xyz(5.0, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        );
    }
}

fn spawn_scene_environment(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::default(),
        WowCamera::default(),
        AmbientLight {
            color: Color::WHITE,
            brightness: 150.0,
            ..default()
        },
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_x(-PI / 4.0)),
    ));

    let grass_texture = generate_grass_texture();
    let grass_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(grass_texture)),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(100.0, 100.0))),
        MeshMaterial3d(grass_material),
    ));

    spawn_ground_clutter(commands, meshes, materials);
}

/// Generate a 64x64 procedural grass texture with color variation.
fn generate_grass_texture() -> Image {
    const SIZE: u32 = 64;
    let mut pixels = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    // Simple hash for deterministic pseudo-random variation
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
        Extent3d { width: SIZE, height: SIZE, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Scatter small rock/bush meshes across the ground for visual reference.
fn spawn_ground_clutter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let rock_mesh = meshes.add(Sphere::new(0.15).mesh().ico(2).unwrap());
    let bush_mesh = meshes.add(Sphere::new(0.3).mesh().ico(1).unwrap());

    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.42, 0.38),
        perceptual_roughness: 0.95,
        ..default()
    });
    let dark_rock_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.33, 0.30),
        perceptual_roughness: 0.95,
        ..default()
    });
    let bush_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 0.15),
        perceptual_roughness: 0.85,
        ..default()
    });

    // Deterministic scatter positions using simple hash
    for i in 0u32..40 {
        let hash1 = (i.wrapping_mul(7919).wrapping_add(1301)) % 10000;
        let hash2 = (i.wrapping_mul(6271).wrapping_add(3571)) % 10000;
        let x = (hash1 as f32 / 10000.0 - 0.5) * 60.0;
        let z = (hash2 as f32 / 10000.0 - 0.5) * 60.0;

        // Skip clutter too close to origin (where the player starts)
        if x * x + z * z < 9.0 {
            continue;
        }

        let (mesh, mat, y_scale) = if i % 5 == 0 {
            // Bush (every 5th)
            let scale_var = 0.7 + (hash1 % 60) as f32 / 100.0;
            (bush_mesh.clone(), bush_mat.clone(), scale_var)
        } else if i % 3 == 0 {
            (rock_mesh.clone(), dark_rock_mat.clone(), 0.6 + (hash2 % 80) as f32 / 100.0)
        } else {
            (rock_mesh.clone(), rock_mat.clone(), 0.5 + (hash1 % 100) as f32 / 100.0)
        };

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_xyz(x, 0.0, z)
                .with_scale(Vec3::new(1.0, y_scale, 1.0)),
        ));
    }
}

fn spawn_m2_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    skinned_mesh_inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
) {
    let model = match asset::m2::load_m2(m2_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load M2 {}: {e}", m2_path.display());
            return;
        }
    };

    // Destructure to avoid partial-move issues when consuming batches in the loop.
    let asset::m2::M2Model { batches, bones, sequences, bone_tracks, global_sequences } = model;

    let name = m2_path.file_stem().and_then(|s| s.to_str()).unwrap_or("m2_model");
    let model_entity = commands
        .spawn((
            Name::new(name.to_owned()),
            Player,
            MovementState::default(),
            CharacterFacing::default(),
            Transform::from_xyz(0.0, 0.0, 0.0)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
            Visibility::default(),
        ))
        .id();

    let skinning = spawn_skeleton(commands, skinned_mesh_inverse_bindposes, &bones, model_entity);
    let joint_entities = attach_bone_pivots_and_player(commands, &bones, &sequences, &skinning, model_entity);

    for (i, batch) in batches.into_iter().enumerate() {
        let material = load_batch_material(&batch, i, images, materials);
        let mut entity_cmd = commands.spawn((Mesh3d(meshes.add(batch.mesh)), MeshMaterial3d(material)));
        entity_cmd.set_parent_in_place(model_entity);
        if let Some((ref inv_bp, ref joints)) = skinning {
            entity_cmd.insert(SkinnedMesh { inverse_bindposes: inv_bp.clone(), joints: joints.clone() });
        }
    }

    if let Some(joints) = joint_entities {
        commands.insert_resource(M2AnimData { sequences, bone_tracks, global_sequences, joint_entities: joints });
    }
}

/// Spawn a static (non-player) M2 model as a scene prop.
fn spawn_static_m2(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    skinned_mesh_inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    m2_path: &Path,
    transform: Transform,
) {
    let model = match asset::m2::load_m2(m2_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load M2 {}: {e}", m2_path.display());
            return;
        }
    };

    let name = m2_path.file_stem().and_then(|s| s.to_str()).unwrap_or("prop");
    let root = commands
        .spawn((Name::new(name.to_owned()), transform, Visibility::default()))
        .id();

    let skinning = spawn_skeleton(commands, skinned_mesh_inverse_bindposes, &model.bones, root);
    for (i, batch) in model.batches.into_iter().enumerate() {
        let mat = load_batch_material(&batch, i, images, materials);
        let mut cmd = commands.spawn((Mesh3d(meshes.add(batch.mesh)), MeshMaterial3d(mat)));
        cmd.set_parent_in_place(root);
        if let Some((ref inv_bp, ref joints)) = skinning {
            cmd.insert(SkinnedMesh { inverse_bindposes: inv_bp.clone(), joints: joints.clone() });
        }
    }
}

/// Attach BonePivot components to joint entities and insert M2AnimPlayer on the model.
/// Returns the joint entity list if animation is active, otherwise None.
fn attach_bone_pivots_and_player(
    commands: &mut Commands,
    bones: &[asset::m2_anim::M2Bone],
    sequences: &[asset::m2_anim::M2AnimSequence],
    skinning: &Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)>,
    model_entity: Entity,
) -> Option<Vec<Entity>> {
    let (_, joints) = skinning.as_ref()?;
    for (i, bone) in bones.iter().enumerate() {
        let p = bone.pivot;
        commands.entity(joints[i]).insert(BonePivot(Vec3::new(p[0], p[2], -p[1])));
    }
    if sequences.is_empty() {
        return None;
    }
    let stand_idx = sequences.iter().position(|s| s.id == 0).unwrap_or(0);
    commands.entity(model_entity).insert(M2AnimPlayer { current_seq_idx: stand_idx, time_ms: 0.0, looping: true, transition: None });
    Some(joints.clone())
}

const PLACEHOLDER_COLORS: &[Color] = &[
    Color::srgb(0.8, 0.5, 0.3), // skin tone
    Color::srgb(0.3, 0.5, 0.8), // blue
    Color::srgb(0.3, 0.8, 0.3), // green
    Color::srgb(0.8, 0.3, 0.3), // red
    Color::srgb(0.7, 0.7, 0.3), // yellow
    Color::srgb(0.6, 0.3, 0.7), // purple
];

/// Spawn bone entities in parent-child hierarchy and create inverse bind poses.
/// Returns None if the model has no bones (static mesh).
fn spawn_skeleton(
    commands: &mut Commands,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    bones: &[asset::m2_anim::M2Bone],
    model_entity: Entity,
) -> Option<(Handle<SkinnedMeshInverseBindposes>, Vec<Entity>)> {
    if bones.is_empty() {
        return None;
    }

    let joint_entities: Vec<Entity> = bones
        .iter()
        .map(|_| commands.spawn(Transform::IDENTITY).id())
        .collect();

    for (i, bone) in bones.iter().enumerate() {
        let parent = if bone.parent_bone_id >= 0 {
            joint_entities[bone.parent_bone_id as usize]
        } else {
            model_entity
        };
        commands.entity(joint_entities[i]).set_parent_in_place(parent);
    }

    let inv_bp = inverse_bindposes.add(SkinnedMeshInverseBindposes::from(
        vec![Mat4::IDENTITY; bones.len()],
    ));

    Some((inv_bp, joint_entities))
}

fn load_batch_material(
    batch: &asset::m2::M2RenderBatch,
    index: usize,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    let texture_dir = PathBuf::from("data/textures");
    if let Some(fdid) = batch.texture_fdid {
        let blp_path = texture_dir.join(format!("{fdid}.blp"));
        if blp_path.exists() {
            if let Some(image) = load_composited_texture(&blp_path, &batch.overlays, &texture_dir)
            {
                return materials.add(StandardMaterial {
                    base_color_texture: Some(images.add(image)),
                    ..default()
                });
            }
        } else {
            eprintln!("Missing texture: data/textures/{fdid}.blp (download with casc-extract)");
        }
    }
    let color = PLACEHOLDER_COLORS[index % PLACEHOLDER_COLORS.len()];
    materials.add(StandardMaterial {
        base_color: color,
        ..default()
    })
}

fn rgba_image(pixels: Vec<u8>, w: u32, h: u32) -> Image {
    Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Load a BLP file as raw RGBA pixels and wrap them into a Bevy Image.
fn load_blp_as_image(path: &Path) -> Result<Image, String> {
    let (pixels, w, h) = asset::blp::load_blp_rgba(path)
        .map_err(|e| format!("Failed to load BLP {}: {e}", path.display()))?;
    Ok(rgba_image(pixels, w, h))
}

/// Blit one overlay onto the base pixel buffer, applying the requested scaling.
fn composite_overlay(
    pixels: &mut Vec<u8>,
    base_width: u32,
    ov: &asset::m2::TextureOverlay,
    texture_dir: &Path,
) {
    use asset::m2::OverlayScale;
    let ov_path = texture_dir.join(format!("{}.blp", ov.fdid));
    match asset::blp::load_blp_rgba(&ov_path) {
        Ok((ov_pixels, ov_w, ov_h)) => match ov.scale {
            OverlayScale::None => {
                asset::blp::blit_region(pixels, base_width, &ov_pixels, ov_w, ov_h, ov.x, ov.y);
            }
            OverlayScale::Uniform2x => {
                let (scaled, sw, sh) = asset::blp::scale_2x(&ov_pixels, ov_w, ov_h);
                asset::blp::blit_region(pixels, base_width, &scaled, sw, sh, ov.x, ov.y);
            }
        },
        Err(e) => eprintln!("Failed to load overlay {}: {e}", ov_path.display()),
    }
}

/// Load a base BLP texture and composite any region overlays on top.
fn load_composited_texture(
    base_path: &Path,
    overlays: &[asset::m2::TextureOverlay],
    texture_dir: &Path,
) -> Option<Image> {
    let (mut pixels, w, h) = asset::blp::load_blp_rgba(base_path)
        .map_err(|e| eprintln!("Failed to load BLP {}: {e}", base_path.display()))
        .ok()?;
    for ov in overlays {
        composite_overlay(&mut pixels, w, ov, texture_dir);
    }
    Some(rgba_image(pixels, w, h))
}

#[allow(clippy::type_complexity)]
fn dump_tree_and_exit(
    tree_query: Query<(
        Entity,
        Option<&Name>,
        Option<&Children>,
        Option<&Visibility>,
        &Transform,
    )>,
    parent_query: Query<&ChildOf>,
    mut exit: MessageWriter<AppExit>,
) {
    let tree = wow_engine::dump::build_tree(&tree_query, &parent_query, None);
    println!("{tree}");
    exit.write(AppExit::Success);
}
