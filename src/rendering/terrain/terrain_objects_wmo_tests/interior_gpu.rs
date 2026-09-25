use super::*;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::mesh::VertexAttributeValues;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const NORTHSHIRE_ABBEY_ROOT_FDID: u32 = 107074;
const CLEAR: [u8; 4] = [255, 0, 255, 255];

/// Northshire Abbey group 0 is an interior group: its MOCV alpha is the indoor
/// lighting blend (fixed to 0), not opacity. A wall built from it must be drawn
/// in the color pass, with or without a depth/normal prepass on the camera.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn abbey_interior_wall_renders_without_prepass() {
    assert_wall_drawn(render_abbey_wall_center(false));
}

/// With a prepass, a wall discarded in the color pass would also leave its
/// depth behind and hide the background, showing only the clear color.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn abbey_interior_wall_renders_through_world_camera_prepass() {
    assert_wall_drawn(render_abbey_wall_center(true));
}

fn assert_wall_drawn(wall: [u8; 4]) {
    assert_ne!(wall, CLEAR, "interior wall left prepass depth but no color");
    assert!(
        !is_background(wall),
        "interior wall is missing; background shows through: {wall:?}"
    );
}

fn render_abbey_wall_center(prepass: bool) -> [u8; 4] {
    let root_data = std::fs::read(format!("data/models/{NORTHSHIRE_ABBEY_ROOT_FDID}.wmo"))
        .expect("Northshire Abbey root WMO in data/models");
    let root = wmo::load_wmo_root(&root_data).expect("parse abbey root");
    let group_fdid = root.group_file_data_ids[0];
    let group_data = std::fs::read(format!("data/models/{group_fdid}.wmo"))
        .expect("Northshire Abbey group 0 WMO in data/models");
    let group = wmo::load_wmo_group_with_root(&group_data, Some(&root)).expect("parse group 0");
    assert!(group.header.group_flags.interior);
    let batch = group.batches[0].clone();
    assert!(batch.has_vertex_color);
    let (centroid, normal) = largest_triangle(&batch.mesh);

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
    let up = if normal.y.abs() > 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Camera {
                clear_color: Color::srgba_u8(CLEAR[0], CLEAR[1], CLEAR[2], CLEAR[3]).into(),
                ..default()
            },
            RenderTarget::Image(target.clone().into()),
            Transform::from_translation(centroid + normal * 0.3).looking_at(centroid, up),
            Msaa::Sample4,
            Tonemapping::None,
        ))
        .id();
    if prepass {
        app.world_mut()
            .entity_mut(camera)
            .insert((DepthPrepass, NormalPrepass));
    }
    spawn_green_background(&mut app, centroid - normal, normal, up);
    spawn_wmo_batch(&mut app, &root, &group, batch);

    capture_center_until_wall(&mut app, target)
}

pub(super) fn gpu_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .disable::<bevy::audio::AudioPlugin>()
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(),
    );
    app.add_plugins(WmoLitMaterialPlugin)
        .init_asset::<crate::water_material::WaterMaterial>()
        .init_asset::<crate::m2_effect_material::M2EffectMaterial>()
        .init_asset::<crate::retail_m2_material::M2Material>();
    app.finish();
    app.cleanup();
    app
}

pub(super) fn largest_triangle(mesh: &Mesh) -> (Vec3, Vec3) {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("WMO batch has no positions");
    };
    let indices: Vec<usize> = mesh.indices().expect("indexed batch").iter().collect();
    indices
        .chunks_exact(3)
        .map(|tri| {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(positions[tri[i]]));
            let cross = (b - a).cross(c - a);
            (cross.length(), (a + b + c) / 3.0, cross.normalize_or_zero())
        })
        .max_by(|x, y| x.0.total_cmp(&y.0))
        .map(|(_, centroid, normal)| (centroid, normal))
        .expect("batch has triangles")
}

fn spawn_green_background(app: &mut App, position: Vec3, normal: Vec3, up: Vec3) {
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(100.0, 100.0));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgb(0.0, 1.0, 0.0),
            unlit: true,
            ..default()
        });
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(position).looking_to(-normal, up),
    ));
}

fn spawn_wmo_batch(
    app: &mut App,
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    batch: wmo::WmoGroupBatch,
) {
    super::unified_gpu::spawn_production_batch(app, root, group, batch);
}

fn is_background(pixel: [u8; 4]) -> bool {
    pixel[1] > 200 && pixel[0] < 40 && pixel[2] < 40
}

/// Recaptures until the center pixel shows neither the clear color nor the
/// background (pipelines compile asynchronously); returns the last center pixel
/// at the deadline otherwise.
fn capture_center_until_wall(app: &mut App, target: Handle<Image>) -> [u8; 4] {
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
            if last != CLEAR && !is_background(last) {
                println!("abbey interior wall pixel {last:?} after {frames} frames");
                return last;
            }
            capture_pending = false;
        }
    }
    println!("abbey interior wall pixel stayed {last:?} for {frames} frames");
    last
}
