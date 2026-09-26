//! Render the terrain shader on a concrete texel and compare with Retail's
//! calcLight plus the ADT layer-alpha specular term.
use super::{TerrainMaterial, TerrainMaterialPlugin, TerrainMaterialSettings};
use crate::retail_light::{RETAIL_SCENE_LIGHT_BUFFER, RetailSceneLight, retail_shade};
use crate::sky_lightdata::{interpolate_colors, load_light_data};
use bevy::asset::RenderAssetUsages;
use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const TEXEL: [u8; 3] = [128, 100, 60];

fn noon_light() -> RetailSceneLight {
    let rows = load_light_data("data/LightData.ron", 12);
    RetailSceneLight::from_sky_colors(&interpolate_colors(&rows, 1440.0), 1440.0)
}

fn test_app() -> App {
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
    app.add_plugins((
        TerrainMaterialPlugin,
        crate::retail_light::RetailLightingPlugin,
    ));
    app.insert_resource(noon_light());
    app.finish();
    app.cleanup();
    app
}

fn solid(app: &mut App, rgba: [u8; 4], cube: bool) -> Handle<Image> {
    let layers = if cube { 6 } else { 1 };
    let mut image = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: layers,
        },
        TextureDimension::D2,
        &rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    if cube {
        image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..default()
        });
    }
    app.world_mut().resource_mut::<Assets<Image>>().add(image)
}

fn spawn_case(app: &mut App, index: usize, texel_alpha: u8) -> Handle<Image> {
    let ground = solid(app, [TEXEL[0], TEXEL[1], TEXEL[2], texel_alpha], false);
    let white = solid(app, [255; 4], false);
    let environment = solid(app, [255; 4], true);
    let material = app
        .world_mut()
        .resource_mut::<Assets<TerrainMaterial>>()
        .add(TerrainMaterial {
            settings: TerrainMaterialSettings {
                config: Vec4::new(1.0, 0.0, 1.0, 0.0),
                layer_params_0: Vec4::new(1.0, 0.0, 0.0, 1.0),
                layer_params_1: Vec4::new(1.0, 0.0, 0.0, 1.0),
                layer_params_2: Vec4::new(1.0, 0.0, 0.0, 1.0),
                layer_params_3: Vec4::new(1.0, 0.0, 0.0, 1.0),
                animation_params_0: Vec4::ZERO,
                animation_params_1: Vec4::ZERO,
                animation_params_2: Vec4::ZERO,
                animation_params_3: Vec4::ZERO,
            },
            ground_0: ground.clone(),
            ground_1: ground.clone(),
            ground_2: ground.clone(),
            ground_3: ground,
            height_0: white.clone(),
            height_1: white.clone(),
            height_2: white.clone(),
            height_3: white.clone(),
            alpha_packed: white.clone(),
            environment_map: environment,
            scene_light: RETAIL_SCENE_LIGHT_BUFFER,
        });
    // Neutral MCCV (0x7F / 127 = 1.0) on a horizontal quad facing +Y.
    let mut mesh = Mesh::from(Rectangle::new(40.0, 40.0));
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0, 1.0, 1.0, 1.0]; 4]);
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        RenderLayers::layer(index),
    ));
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            17,
            17,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            order: index as isize,
            clear_color: Color::srgb(1.0, 0.0, 1.0).into(),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        Transform::from_xyz(0.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
        RenderLayers::layer(index),
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
    ));
    target
}

fn capture_centres(app: &mut App, targets: &[Handle<Image>]) -> Vec<[u8; 3]> {
    let (sender, receiver) = mpsc::channel();
    let mut pending = vec![false; targets.len()];
    let mut pixels: Vec<Option<[u8; 3]>> = vec![None; targets.len()];
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut frames = 0;
    while Instant::now() < deadline && pixels.iter().any(Option::is_none) {
        app.update();
        frames += 1;
        if frames < 10 {
            continue;
        }
        for (index, target) in targets.iter().enumerate() {
            if pending[index] || pixels[index].is_some() {
                continue;
            }
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |capture: On<ScreenshotCaptured>| {
                    sender
                        .send((index, capture.image.clone()))
                        .expect("test receiver exists");
                });
            pending[index] = true;
        }
        for (index, image) in receiver.try_iter() {
            pending[index] = false;
            let [r, g, b, _] = image
                .try_into_dynamic()
                .expect("readable capture")
                .to_rgba8()
                .get_pixel(8, 8)
                .0;
            if [r, g, b] != [255, 0, 255] && [r, g, b] != [0, 0, 0] {
                pixels[index] = Some([r, g, b]);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    pixels
        .into_iter()
        .enumerate()
        .map(|(index, pixel)| pixel.unwrap_or_else(|| panic!("case {index} never rendered")))
        .collect()
}

fn gamma_bytes(color: Vec3) -> [u8; 3] {
    color
        .to_array()
        .map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn terrain_renders_retail_light_and_layer_alpha_specular() {
    let mut app = test_app();
    let targets = [spawn_case(&mut app, 0, 0), spawn_case(&mut app, 1, 255)];
    let pixels = capture_centres(&mut app, &targets);

    let light = noon_light();
    let texel = Vec3::from_array(TEXEL.map(|channel| channel as f32 / 255.0));
    let lit = retail_shade(&light, texel, Vec3::Y, 1.0);
    // The camera looks straight down: half vector of the sun and the view ray.
    let view = Vec3::Y;
    let half = -(light.sun_direction - view).normalize();
    let highlight = half.dot(Vec3::Y).max(0.0).powf(20.0);
    let specular = light.direct * highlight;
    let expected = [
        ("layer alpha 0: no specular", gamma_bytes(lit)),
        ("layer alpha 1: full specular", gamma_bytes(lit + specular)),
    ];
    assert_eq!(
        expected[0].1,
        [97, 80, 50],
        "calcLight for this texel at noon"
    );
    for ((label, expected), actual) in expected.into_iter().zip(pixels) {
        println!("{label}: rendered {actual:?}, expected {expected:?}");
        for channel in 0..3 {
            assert!(
                actual[channel].abs_diff(expected[channel]) <= 2,
                "{label}: rendered {actual:?}, expected {expected:?}"
            );
        }
    }
}
