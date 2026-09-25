use super::interior_gpu::{gpu_app, largest_triangle};
use super::*;
use crate::m2_effect_material::M2EffectMaterial;
use crate::retail_m2_material::M2Material;
use crate::water_material::WaterMaterial;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// `sw_tradedistrict.wmo` sets MOHD 0x02 (unified MapObj path). Its exterior groups carry
/// MOCV that is mostly near zero: baked light added to daylight, not the only light.
const TRADE_DISTRICT_ROOT_FDID: u32 = 322057;
const TRADE_DISTRICT_EXTERIOR_GROUP: usize = 37;
const CLEAR: [u8; 4] = [255, 0, 255, 255];
const SUN_ILLUMINANCE: f32 = 2500.0;

/// The Trade District buildings rendered as black silhouettes because the group's
/// MOCV was the whole (unlit) light. A wall facing the sun must be at least as bright
/// as the same texture lit by that sun alone.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn trade_district_exterior_wall_is_not_darker_than_daylight() {
    let (root, group) = load_trade_district_exterior_group();
    let wall = render_wall_center(&root, &group, WallShading::Wmo);
    let daylight = render_wall_center(&root, &group, WallShading::DaylightOnly);
    println!("wmo wall {wall:?}, daylight-only reference {daylight:?}");
    assert!(
        luminance(wall) + 3 >= luminance(daylight),
        "exterior WMO wall {wall:?} is darker than daylight alone {daylight:?}"
    );
}

#[derive(Clone, Copy)]
pub(super) enum WallShading {
    /// The production WMO group batch spawn.
    Wmo,
    /// Lit StandardMaterial with the batch texture and no vertex color.
    DaylightOnly,
    /// Retail interior light: unlit `texture * (ambient + 2 * fixed MOCV)`. Bevy
    /// multiplies linear vertex color into the base color, so the reference carries
    /// `srgb_to_linear(ambient + 2 * MOCV)`.
    RetailInterior { ambient: [f32; 3] },
}

fn load_trade_district_exterior_group() -> (wmo::WmoRootData, wmo::WmoGroupData) {
    let root_data = std::fs::read(format!("data/models/{TRADE_DISTRICT_ROOT_FDID}.wmo"))
        .expect("sw_tradedistrict root WMO in data/models");
    let root = wmo::load_wmo_root(&root_data).expect("parse sw_tradedistrict root");
    assert!(root.flags.use_unified_render_path);
    let group_fdid = root.group_file_data_ids[TRADE_DISTRICT_EXTERIOR_GROUP];
    let group_data = std::fs::read(format!("data/models/{group_fdid}.wmo"))
        .expect("sw_tradedistrict exterior group in data/models");
    let group = wmo::load_wmo_group_with_root(&group_data, Some(&root)).expect("parse group");
    assert!(group.header.group_flags.exterior);
    assert!(group.batches[0].has_vertex_color);
    (root, group)
}

fn render_wall_center(
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    shading: WallShading,
) -> [u8; 4] {
    render_batch_center(root, group, 0, shading)
}

pub(super) fn render_batch_center(
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    batch_index: usize,
    shading: WallShading,
) -> [u8; 4] {
    let batch = group.batches[batch_index].clone();
    let (centroid, normal) = largest_triangle(&batch.mesh);
    let up = if normal.y.abs() > 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let mut app = gpu_app();
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            64,
            64,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            clear_color: Color::srgba_u8(CLEAR[0], CLEAR[1], CLEAR[2], CLEAR[3]).into(),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        Transform::from_translation(centroid + normal * 0.3).looking_at(centroid, up),
        Msaa::Sample4,
        Tonemapping::None,
    ));
    app.world_mut().spawn((
        DirectionalLight {
            illuminance: SUN_ILLUMINANCE,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default().looking_to(-normal, up),
    ));
    match shading {
        WallShading::Wmo => spawn_production_batch(&mut app, root, group, batch),
        WallShading::DaylightOnly => spawn_daylight_reference(&mut app, root, batch),
        WallShading::RetailInterior { ambient } => {
            spawn_retail_interior_reference(&mut app, root, batch, ambient)
        }
    }
    capture_center_until_drawn(&mut app, target)
}

pub(super) fn spawn_production_batch(
    app: &mut App,
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    batch: wmo::WmoGroupBatch,
) {
    let group_entity = app
        .world_mut()
        .spawn((Transform::default(), Visibility::default()))
        .id();
    let interior_ambient = wmo_interior_ambient(root, 0);
    app.world_mut()
        .resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
            world.resource_scope(|world, mut materials: Mut<Assets<M2Material>>| {
                world.resource_scope(|world, mut water: Mut<Assets<WaterMaterial>>| {
                    world.resource_scope(|world, mut effects: Mut<Assets<M2EffectMaterial>>| {
                        world.resource_scope(
                            |world, mut bindposes: Mut<Assets<SkinnedMeshInverseBindposes>>| {
                                world.resource_scope(|world, mut images: Mut<Assets<Image>>| {
                                    let mut assets = WmoAssets {
                                        meshes: &mut meshes,
                                        materials: &mut materials,
                                        water_materials: &mut water,
                                        images: &mut images,
                                        effect_materials: &mut effects,
                                        inverse_bindposes: &mut bindposes,
                                    };
                                    let mut commands = world.commands();
                                    spawn_wmo_group_batches(
                                        &mut commands,
                                        &mut assets,
                                        root,
                                        &group.header,
                                        interior_ambient,
                                        group_entity,
                                        vec![batch],
                                    );
                                });
                            },
                        );
                    });
                });
            });
        });
    app.world_mut().flush();
}

fn spawn_daylight_reference(app: &mut App, root: &wmo::WmoRootData, mut batch: wmo::WmoGroupBatch) {
    batch.mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR);
    spawn_reference(app, root, batch, false);
}

fn spawn_retail_interior_reference(
    app: &mut App,
    root: &wmo::WmoRootData,
    mut batch: wmo::WmoGroupBatch,
    ambient: [f32; 3],
) {
    let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
        batch.mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
    else {
        panic!("batch has no MOCV");
    };
    for color in colors.iter_mut() {
        let [r, g, b] = [0, 1, 2].map(|channel| ambient[channel] + 2.0 * color[channel]);
        let light = Color::srgb(r, g, b).to_linear();
        *color = [light.red, light.green, light.blue, 1.0];
    }
    spawn_reference(app, root, batch, true);
}

fn spawn_reference(app: &mut App, root: &wmo::WmoRootData, batch: wmo::WmoGroupBatch, unlit: bool) {
    let props = wmo_material_props(root, batch.material_index);
    let texture = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        load_wmo_batch_material_image(&mut images, batch.material_index, &props)
            .expect("batch texture")
    };
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color_texture: Some(texture),
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            alpha_mode: AlphaMode::Mask(0.5),
            unlit,
            ..default()
        });
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(batch.mesh);
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::default()));
}

pub(super) fn luminance(pixel: [u8; 4]) -> u32 {
    (pixel[0] as u32 * 2126 + pixel[1] as u32 * 7152 + pixel[2] as u32 * 722) / 10000
}

/// Recaptures until the center pixel is no longer the clear color (pipelines compile
/// asynchronously), then returns it.
pub(super) fn capture_center_until_drawn(app: &mut App, target: Handle<Image>) -> [u8; 4] {
    let (sender, receiver) = mpsc::channel();
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut frames = 0;
    let mut capture_pending = false;
    let mut last = CLEAR;
    while Instant::now() < deadline {
        app.update();
        frames += 1;
        if frames >= 30 && !capture_pending {
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |capture: On<ScreenshotCaptured>| {
                    sender
                        .send(capture.image.clone())
                        .expect("test receiver exists");
                });
            capture_pending = true;
        }
        if let Ok(image) = receiver.try_recv() {
            let pixels = image
                .try_into_dynamic()
                .expect("readable screenshot")
                .to_rgba8();
            last = pixels.get_pixel(32, 32).0;
            if last != CLEAR {
                return last;
            }
            capture_pending = false;
        }
    }
    panic!("wall never drawn after {frames} frames; center stayed {last:?}");
}
