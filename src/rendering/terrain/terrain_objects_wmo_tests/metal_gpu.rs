use super::interior_gpu::gpu_app;
use super::unified_gpu::{capture_center_until_drawn, luminance};
use super::*;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const CLEAR: [u8; 4] = [255, 0, 255, 255];
const MAP_OBJ_DIFFUSE: u32 = 0;
const MAP_OBJ_METAL: u32 = 2;
const MAP_OBJ_ENV_METAL: u32 = 5;
const GROUP_EXTERIOR: u32 = 0x8;

/// Retail MapObjMetal/EnvMetal keep the diffuse term (`matDiffuse = tex.rgb`) and add
/// specular or environment light on top (WebWowViewerCpp `caclWMOFragMat`); nothing
/// makes the surface metallic. Lit off-mirror, they are at least as bright as
/// MapObjDiffuse with the same texture.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn metal_shaders_keep_the_diffuse_term() {
    let diffuse = render_lit_texel(MAP_OBJ_DIFFUSE);
    for shader in [MAP_OBJ_METAL, MAP_OBJ_ENV_METAL] {
        let metal = render_lit_texel(shader);
        println!("shader {shader}: {metal:?}, diffuse {diffuse:?}");
        assert!(
            luminance(metal) + 3 >= luminance(diffuse),
            "shader {shader} {metal:?} is darker than diffuse {diffuse:?}"
        );
    }
}

/// A grey texel on a lit WMO quad, sun at 45 degrees off the view axis.
fn render_lit_texel(shader: u32) -> [u8; 4] {
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
    app.world_mut().spawn((
        DirectionalLight {
            illuminance: 2500.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default().looking_to(Vec3::new(1.0, 0.0, -1.0), Vec3::Y),
    ));
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
    let base = wmo_standard_material(Some(texel), 0, false, shader, None);
    let material = app
        .world_mut()
        .resource_mut::<Assets<WmoLitMaterial>>()
        .add(wmo_lit_material(
            base,
            GROUP_EXTERIOR,
            false,
            [0.0; 3],
            None,
        ));
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(10.0, 10.0));
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::default()));
    capture_center_until_drawn(&mut app, target)
}
