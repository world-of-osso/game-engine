use super::interior_gpu::gpu_app;
use super::unified_gpu::capture_center_until_drawn;
use super::*;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const CLEAR: [u8; 4] = [255, 0, 255, 255];
const BLEND_OPAQUE: u32 = 0;
const BLEND_ALPHA_KEY: u32 = 1;

/// MOMT blend 0 (GxBlend_Opaque) has no alpha test: a texel with low alpha
/// still covers what is behind it.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn opaque_blend_draws_low_alpha_texels() {
    let pixel = render_surface_center(BLEND_OPAQUE, 60);
    assert!(
        !is_green(pixel),
        "opaque WMO texel was discarded: {pixel:?}"
    );
}

/// Retail MOMT blend 1 (GxBlend_AlphaKey) discards texels below 128/255
/// (WebWowViewerCpp `caclWMOFragMat`: `tex.a - 0.501960814 < 0`).
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn alpha_key_blend_discards_texels_below_128() {
    let below = render_surface_center(BLEND_ALPHA_KEY, 100);
    assert!(is_green(below), "alpha 100 texel was kept: {below:?}");
    let above = render_surface_center(BLEND_ALPHA_KEY, 160);
    assert!(!is_green(above), "alpha 160 texel was discarded: {above:?}");
}

fn is_green(pixel: [u8; 4]) -> bool {
    pixel[1] > 200 && pixel[0] < 40 && pixel[2] < 40
}

/// A red texel with `alpha` on a WMO surface in front of a green plane.
fn render_surface_center(blend_mode: u32, alpha: u8) -> [u8; 4] {
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
        Transform::from_xyz(0.0, 0.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Sample4,
        Tonemapping::None,
    ));
    let texel = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new(
            Extent3d::default(),
            TextureDimension::D2,
            vec![255, 0, 0, alpha],
            TextureFormat::Rgba8UnormSrgb,
            default(),
        ));
    let mut wmo = wmo_standard_material(Some(texel), blend_mode, false, 0, None);
    wmo.unlit = true;
    let background = StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 0.0),
        unlit: true,
        ..default()
    };
    spawn_quad(&mut app, wmo, 0.0);
    spawn_quad(&mut app, background, -1.0);
    capture_center_until_drawn(&mut app, target)
}

fn spawn_quad(app: &mut App, material: StandardMaterial, z: f32) {
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(10.0, 10.0));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(material);
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, z),
    ));
}
