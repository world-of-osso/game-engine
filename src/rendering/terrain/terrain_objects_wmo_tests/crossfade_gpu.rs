use super::interior_gpu::{gpu_app, set_sun};
use super::unified_gpu::{capture_center_until_drawn, luminance};
use super::*;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const CLEAR: [u8; 4] = [255, 0, 255, 255];
const GROUP_INTERIOR: u32 = 0x2000;
/// Dim interior ambient against a bright sun, so the two lights differ clearly.
const INTERIOR_AMBIENT: [f32; 3] = [0.1, 0.1, 0.1];

/// Retail cross-fades transition-batch vertices between interior and exterior light by
/// their MOCV alpha (WebWowViewerCpp `interiorExteriorBlend = vColor.w`) in a single
/// pass: alpha 0 is interior light, 1 exterior, and 0.5 is their gamma-space midpoint.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn transition_vertices_crossfade_interior_and_exterior_light_by_mocv_alpha() {
    let interior = luminance(render_interior_group_texel(0.0));
    let half = luminance(render_interior_group_texel(0.5));
    let exterior = luminance(render_interior_group_texel(1.0));
    println!("interior {interior}, half {half}, exterior {exterior}");
    assert!(
        interior + 20 < exterior,
        "sun must outshine the interior ambient"
    );
    let midpoint = (interior + exterior) / 2;
    assert!(
        half.abs_diff(midpoint) <= 4,
        "half blend {half} is not the gamma midpoint {midpoint}"
    );
}

/// A grey texel on a quad of an interior WMO group whose MOCV is black with `alpha`.
fn render_interior_group_texel(alpha: f32) -> [u8; 4] {
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
    set_sun(&mut app, Vec3::NEG_Z);
    let texel = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new(
            Extent3d::default(),
            TextureDimension::D2,
            vec![160, 160, 160, 255],
            TextureFormat::Rgba8UnormSrgb,
            default(),
        ));
    let base = wmo_standard_material(Some(texel), 0, false, 0, None);
    let material = wmo_lit_material(
        base,
        GROUP_INTERIOR,
        WmoLitSurface::default(),
        INTERIOR_AMBIENT,
        None,
    );
    let material = app
        .world_mut()
        .resource_mut::<Assets<WmoLitMaterial>>()
        .add(material);
    let mut mesh = Mesh::from(Rectangle::new(10.0, 10.0));
    let count = mesh.count_vertices();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0, 0.0, 0.0, alpha]; count]);
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::default()));
    capture_center_until_drawn(&mut app, target)
}
