use super::interior_gpu::gpu_app;
use super::unified_gpu::capture_center_until_drawn;
use super::*;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const CLEAR: [u8; 4] = [255, 0, 255, 255];
const GROUP_EXTERIOR: u32 = 0x8;
const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// Retail MOMT 13 (TwoLayerDiffuseOpaque): `mix(texture2, texture1, second MOCV alpha)`.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn two_layer_diffuse_opaque_blends_by_second_mocv_alpha() {
    assert_close(render_two_layer(13, BLUE, 0.0), BLUE);
    assert_close(render_two_layer(13, BLUE, 1.0), RED);
}

/// Retail MOMT 6 (TwoLayerDiffuse): `mix(mix(texture1, texture2, texture2 alpha),
/// texture1, second MOCV alpha)`, in gamma space.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn two_layer_diffuse_blends_by_texture_alpha_then_second_mocv_alpha() {
    let half_blue = [0, 0, 255, 128];
    assert_close(render_two_layer(6, half_blue, 0.0), [127, 0, 128, 255]);
    assert_close(render_two_layer(6, half_blue, 1.0), RED);
}

/// The world camera carries a depth/normal prepass; its pipelines keep Bevy's
/// vertex stage and must still build for two-layer meshes.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn two_layer_diffuse_opaque_renders_with_prepass_camera() {
    assert_close(render_two_layer_with(13, BLUE, 0.0, true), BLUE);
}

fn assert_close(pixel: [u8; 4], expected: [u8; 4]) {
    let close = pixel
        .iter()
        .zip(expected)
        .all(|(pixel, expected)| pixel.abs_diff(expected) <= 3);
    assert!(close, "pixel {pixel:?}, expected {expected:?}");
}

/// A red first layer and `second` layer on an unlit WMO quad whose second MOCV alpha
/// is `second_mocv_alpha` everywhere.
fn render_two_layer(shader: u32, second: [u8; 4], second_mocv_alpha: f32) -> [u8; 4] {
    render_two_layer_with(shader, second, second_mocv_alpha, false)
}

fn render_two_layer_with(
    shader: u32,
    second: [u8; 4],
    second_mocv_alpha: f32,
    prepass: bool,
) -> [u8; 4] {
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
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Camera {
                clear_color: Color::srgba_u8(CLEAR[0], CLEAR[1], CLEAR[2], CLEAR[3]).into(),
                ..default()
            },
            RenderTarget::Image(target.clone().into()),
            Transform::from_xyz(0.0, 0.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            Msaa::Sample4,
            Tonemapping::None,
        ))
        .id();
    if prepass {
        app.world_mut().entity_mut(camera).insert((
            bevy::core_pipeline::prepass::DepthPrepass,
            bevy::core_pipeline::prepass::NormalPrepass,
        ));
    }
    let first = add_texel(&mut app, RED);
    let second = add_texel(&mut app, second);
    let base = wmo_standard_material(Some(first), 0, false, shader, None);
    let material = wmo_lit_material(
        base,
        GROUP_EXTERIOR,
        WmoLitSurface {
            unlit: true,
            ..default()
        },
        [0.0; 3],
        Some(WmoSecondLayer {
            shader,
            texture: second,
        }),
    );
    let material = app
        .world_mut()
        .resource_mut::<Assets<WmoLitMaterial>>()
        .add(material);
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(two_layer_quad(second_mocv_alpha));
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::default()));
    capture_center_until_drawn(&mut app, target)
}

fn add_texel(app: &mut App, rgba: [u8; 4]) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new(
            Extent3d::default(),
            TextureDimension::D2,
            rgba.to_vec(),
            TextureFormat::Rgba8UnormSrgb,
            default(),
        ))
}

fn two_layer_quad(second_mocv_alpha: f32) -> Mesh {
    let mut mesh = Mesh::from(Rectangle::new(10.0, 10.0));
    let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
        mesh.attribute(Mesh::ATTRIBUTE_UV_0).cloned()
    else {
        panic!("rectangle has uvs");
    };
    let count = uvs.len();
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0, 0.0, 0.0, 1.0]; count]);
    mesh.insert_attribute(
        wmo::WMO_BLEND_ALPHA_ATTRIBUTE,
        vec![second_mocv_alpha; count],
    );
    mesh
}

/// `sw_tradedistrict` group 33 batch 18 is MOMT 46 (shader 13, `strmwnd_crenlatn`) with MOCV2.
/// The production spawn binds MOMT texture_2, `strmwnd_crenlatn_burned` (464811), as the
/// second layer.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn trade_district_two_layer_batch_binds_its_momt_second_texture() {
    let root_data = std::fs::read("data/models/322057.wmo").expect("sw_tradedistrict root WMO");
    let root = wmo::load_wmo_root(&root_data).expect("parse root");
    let group_data = std::fs::read(format!("data/models/{}.wmo", root.group_file_data_ids[33]))
        .expect("sw_tradedistrict group 33");
    let group = wmo::load_wmo_group_with_root(&group_data, Some(&root)).expect("group 33");
    let batch = group.batches[18].clone();
    assert_eq!(batch.material_index, 46);
    let mut app = gpu_app();

    super::unified_gpu::spawn_production_batch(&mut app, &root, &group, batch);

    let world = app.world_mut();
    let handle = world
        .query::<&MeshMaterial3d<WmoLitMaterial>>()
        .single(world)
        .expect("one WMO batch")
        .0
        .clone();
    let second = world
        .resource::<Assets<WmoLitMaterial>>()
        .get(&handle)
        .and_then(|material| material.extension.second_texture.clone())
        .expect("two-layer batch has a second texture");
    let bound = world
        .resource::<Assets<Image>>()
        .get(&second)
        .and_then(|image| image.data.clone())
        .expect("second texture pixels");
    let path = crate::asset::asset_cache::texture(464811).expect("strmwnd_crenlatn_burned");
    let (expected, _, _) = crate::asset::blp::load_blp_rgba(&path).expect("decode 464811");
    assert!(
        bound == expected,
        "second layer is not strmwnd_crenlatn_burned"
    );
}
