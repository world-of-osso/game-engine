//! Render real M2Material batches and compare pixels with the Retail equation.
use super::*;
use crate::retail_light::{RetailSceneLight, retail_shade};
use crate::sky_lightdata::{interpolate_colors, load_light_data};
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Texel of the test texture, as authored sRGB bytes.
const TEXEL: [u8; 3] = [128, 100, 60];

fn noon_light() -> RetailSceneLight {
    let rows = load_light_data("data/LightData.ron", 12);
    RetailSceneLight::from_sky_colors(&interpolate_colors(&rows, 1440.0), 1440.0)
}

fn render_app() -> App {
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
    app.add_plugins(crate::retail_light::RetailLightingPlugin);
    app.insert_resource(noon_light());
    app.finish();
    app.cleanup();
    app
}

struct Case {
    render_flags: u16,
    fog: Option<DistanceFog>,
    tonemapping: Tonemapping,
    texel: [u8; 3],
}

fn spawn_case(app: &mut App, index: usize, case: &Case) -> Handle<Image> {
    let texture = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(crate::rgba_image(
            vec![case.texel[0], case.texel[1], case.texel[2], 255],
            1,
            1,
        ));
    let material = app
        .world_mut()
        .resource_mut::<Assets<M2Material>>()
        .add(retail_m2_material(
            StandardMaterial {
                base_color_texture: Some(texture),
                ..default()
            },
            case.render_flags,
            0,
        ));
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(40.0, 40.0));
    // Horizontal quad facing +Y, 5 yards below its camera.
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
    let mut camera = app.world_mut().spawn((
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
        case.tonemapping,
        DebandDither::Disabled,
    ));
    if let Some(fog) = case.fog.clone() {
        camera.insert(fog);
    }
    target
}

fn gamma_bytes(color: Vec3) -> [u8; 3] {
    color
        .to_array()
        .map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn texel_gamma() -> Vec3 {
    Vec3::from_array(TEXEL.map(|channel| channel as f32 / 255.0))
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

fn assert_close(actual: [u8; 3], expected: [u8; 3], label: &str) {
    for channel in 0..3 {
        assert!(
            actual[channel].abs_diff(expected[channel]) <= 2,
            "{label}: rendered {actual:?}, expected {expected:?}"
        );
    }
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn m2_batches_render_the_retail_equation_on_a_concrete_texel() {
    let mut app = render_app();
    let fog_color = Vec3::new(77.0, 120.0, 143.0) / 255.0;
    let cases = [
        Case {
            render_flags: 0,
            fog: None,
            tonemapping: Tonemapping::None,
            texel: TEXEL,
        },
        Case {
            render_flags: 0x01,
            fog: None,
            tonemapping: Tonemapping::None,
            texel: TEXEL,
        },
        // The quad centre is 5 yards away: halfway through a 0–10 yard fog.
        Case {
            render_flags: 0,
            fog: Some(DistanceFog {
                color: Color::srgb(fog_color.x, fog_color.y, fog_color.z),
                falloff: FogFalloff::Linear {
                    start: 0.0,
                    end: 10.0,
                },
                ..default()
            }),
            tonemapping: Tonemapping::None,
            texel: TEXEL,
        },
        Case {
            render_flags: 0x02,
            fog: Some(DistanceFog {
                color: Color::srgb(fog_color.x, fog_color.y, fog_color.z),
                falloff: FogFalloff::Linear {
                    start: 0.0,
                    end: 10.0,
                },
                ..default()
            }),
            tonemapping: Tonemapping::None,
            texel: TEXEL,
        },
    ];
    let targets: Vec<_> = cases
        .iter()
        .enumerate()
        .map(|(index, case)| spawn_case(&mut app, index, case))
        .collect();
    let pixels = capture_centres(&mut app, &targets);

    let lit = retail_shade(&noon_light(), texel_gamma(), Vec3::Y, 1.0);
    // Independent evaluation of calcLight for this texel at noon LightParams 12.
    assert_eq!(gamma_bytes(lit), [97, 80, 50]);
    let fogged = lit.lerp(fog_color, 0.5);
    let expected = [
        ("lit", gamma_bytes(lit)),
        ("unlit (render flag 0x1)", TEXEL),
        ("lit, fogged in authored space", gamma_bytes(fogged)),
        ("unfogged (render flag 0x2)", gamma_bytes(lit)),
    ];
    for ((label, expected), actual) in expected.into_iter().zip(pixels) {
        println!("{label}: rendered {actual:?}, expected {expected:?}");
        assert_close(actual, expected, label);
    }
}

/// The world camera's tonemapping must leave Retail's authored-space output
/// unchanged. Measured before the switch to `Tonemapping::None`: TonyMcMapface
/// rendered texels 200/180/150 as 173/158/135 and 250/245/235 as 195/192/186.
#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn world_camera_tonemapping_keeps_retail_output() {
    let mut app = render_app();
    let production = crate::camera::world_camera_tonemapping();
    let texels = [[60, 45, 30], TEXEL, [200, 180, 150], [250, 245, 235]];
    let cases: Vec<_> = texels
        .iter()
        .flat_map(|texel| {
            [production, Tonemapping::None].map(|tonemapping| Case {
                render_flags: 0x01,
                fog: None,
                tonemapping,
                texel: *texel,
            })
        })
        .collect();
    let targets: Vec<_> = cases
        .iter()
        .enumerate()
        .map(|(index, case)| spawn_case(&mut app, index, case))
        .collect();
    let pixels = capture_centres(&mut app, &targets);
    for (texel, pair) in texels.iter().zip(pixels.chunks(2)) {
        println!(
            "unlit texel {texel:?}: {production:?} {:?}, None {:?}",
            pair[0], pair[1]
        );
        assert_eq!(
            pair[1], *texel,
            "without tonemapping the texel is unchanged"
        );
        assert_eq!(pair[0], *texel, "the production camera must not tonemap");
    }
}
