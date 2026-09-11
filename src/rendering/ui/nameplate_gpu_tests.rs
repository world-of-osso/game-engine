//! Pixel proof for the real replicated-NPC observer and projected UI text.

const HEALTH_NAME_FAILURE_IMAGE: &str =
    "data/diagnostics/equipment-nameplate-alignment-20260909/health-name-gpu-failure.png";
use super::*;
use bevy::camera::RenderTarget;
use bevy::ecs::system::RunSystemOnce;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn render_app() -> App {
    configured_render_app(|_| {})
}

fn configured_render_app(configure: impl FnOnce(&mut App)) -> App {
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
    app.insert_resource(HudOptions {
        nameplate_health_thickness: NameplateBarThickness::Thin,
        ..default()
    });
    app.init_state::<GameState>();
    app.insert_resource(State::new(GameState::InWorld));
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.add_plugins(NameplatePlugin);
    configure(&mut app);
    app.finish();
    app.cleanup();
    app
}

fn cameras_and_wolf(app: &mut App) -> Handle<Image> {
    let target = render_cameras(app);
    app.world_mut().spawn((
        Npc {
            template_id: 299,
            name: "Diseased Young Wolf".into(),
        },
        Transform::from_xyz(-9000.0, 0.0, 0.0),
        Visibility::Visible,
    ));
    target
}

fn render_cameras(app: &mut App) -> Handle<Image> {
    render_cameras_sized(app, 512, 192)
}

fn render_cameras_sized(app: &mut App, width: u32, height: u32) -> Handle<Image> {
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            width,
            height,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            clear_color: Color::srgb(24.0 / 255.0, 21.0 / 255.0, 20.0 / 255.0).into(),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        Transform::from_xyz(-9000.0, NPC_NAMEPLATE_Y, 10.0)
            .looking_at(Vec3::new(-9000.0, NPC_NAMEPLATE_Y, 0.0), Vec3::Y),
        Msaa::Off,
    ));
    app.world_mut()
        .run_system_once(ui_toolkit::render::setup_ui_camera)
        .unwrap();
    let overlay = app
        .world_mut()
        .query_filtered::<Entity, With<ui_toolkit::render::UiCamera>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(overlay)
        .insert((RenderTarget::Image(target.clone().into()), Msaa::Off));
    target
}

#[derive(Debug)]
struct ColoredPixels {
    count: usize,
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
}

fn colored_pixels(image: &Image, matches: impl Fn(&[u8]) -> bool) -> Option<ColoredPixels> {
    let width = image.width() as usize;
    let mut bounds = None;
    for (index, pixel) in image
        .data
        .as_ref()
        .expect("captured pixels")
        .chunks_exact(4)
        .enumerate()
    {
        if !matches(pixel) {
            continue;
        }
        let y = index / width;
        let x = index % width;
        let bounds = bounds.get_or_insert(ColoredPixels {
            count: 0,
            top: y,
            bottom: y,
            left: x,
            right: x,
        });
        bounds.count += 1;
        bounds.top = bounds.top.min(y);
        bounds.bottom = bounds.bottom.max(y);
        bounds.left = bounds.left.min(x);
        bounds.right = bounds.right.max(x);
    }
    bounds
}

fn compact_health_name_pixels(image: &Image) -> bool {
    let name = colored_pixels(image, |p| p[0] > 220 && p[1] > 220 && p[2] > 220);
    let bar = colored_pixels(image, |p| p[0] > 100 && p[1] < 60 && p[2] < 60);
    match (name, bar) {
        (Some(name), Some(bar)) => {
            // Glyph ink may stop above the text-layout bottom (font descender padding).
            let gap = bar.top.checked_sub(name.bottom + 1);
            name.count > 100 && bar.count > 10 && gap.is_some_and(|gap| (1..=16).contains(&gap))
        }
        _ => false,
    }
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn nameplate_gpu_health_before_npc_renders_name_above_health_bar() {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut app = configured_render_app(|app| {
        app.add_plugins(crate::health_bar::HealthBarPlugin);
    });
    let target = render_cameras(&mut app);
    let owner = app
        .world_mut()
        .spawn((
            Transform::from_xyz(-9000.0, 0.0, 0.0),
            Visibility::Visible,
            shared::components::Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    app.world_mut().entity_mut(owner).insert(Npc {
        template_id: 299,
        name: "Diseased Young Wolf".into(),
    });
    let (sender, receiver) = mpsc::channel();
    let mut pending = false;
    let mut last_image = None;
    while Instant::now() < deadline {
        app.update();
        if !pending {
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |capture: On<ScreenshotCaptured>| {
                    sender
                        .send(capture.image.clone())
                        .expect("pixel receiver alive");
                });
            pending = true;
        }
        if let Ok(image) = receiver.try_recv() {
            pending = false;
            if compact_health_name_pixels(&image) {
                return;
            }
            last_image = Some(image);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let image = last_image.expect("GPU must return a frame within the fixture deadline");
    let name = colored_pixels(&image, |p| p[0] > 220 && p[1] > 220 && p[2] > 220);
    let bar = colored_pixels(&image, |p| p[0] > 100 && p[1] < 60 && p[2] < 60);
    let path = std::path::Path::new(HEALTH_NAME_FAILURE_IMAGE);
    std::fs::create_dir_all(path.parent().unwrap()).expect("create diagnostic directory");
    image
        .try_into_dynamic()
        .expect("RGBA screenshot")
        .save(path)
        .expect("save failure image");
    panic!(
        "expected white name above red health bar with compact gap: name={name:?}, bar={bar:?}; image={HEALTH_NAME_FAILURE_IMAGE}"
    );
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn nameplate_gpu_zoom_preserves_bar_and_text_pixel_dimensions() {
    let mut app = configured_render_app(|app| {
        app.add_plugins(crate::health_bar::HealthBarPlugin);
    });
    let target = render_cameras(&mut app);
    app.world_mut().spawn((
        Transform::from_xyz(-9000.0, 0.0, 0.0),
        Visibility::Visible,
        shared::components::Health {
            current: 100.0,
            max: 100.0,
        },
        Npc {
            template_id: 299,
            name: "Wolf".into(),
        },
    ));
    let camera = app
        .world_mut()
        .query_filtered::<Entity, With<Camera3d>>()
        .single(app.world())
        .unwrap();
    let mut name_size = None;
    for distance in [5.0, 10.0, 20.0] {
        app.world_mut()
            .get_mut::<Transform>(camera)
            .unwrap()
            .translation
            .z = distance;
        for _ in 0..5 {
            app.update();
        }
        let image = capture_zoom_frame(&mut app, &target);
        let bar = colored_pixels(&image, |p| p[0] > 100 && p[1] < 60 && p[2] < 60)
            .expect("red health bar pixels");
        let name = colored_pixels(&image, |p| p[0] > 220 && p[1] > 220 && p[2] > 220)
            .expect("white name pixels");
        let bar_size = (bar.right - bar.left + 1, bar.bottom - bar.top + 1);
        assert!(
            bar_size.0.abs_diff(382) <= 1 && bar_size.1.abs_diff(18) <= 1,
            "zoom distance {distance}: expected 382x18px red interior, got {bar_size:?}"
        );
        let current_name_size = (name.right - name.left + 1, name.bottom - name.top + 1);
        if let Some(expected) = name_size {
            assert_eq!(current_name_size, expected);
        }
        name_size = Some(current_name_size);
        assert!(
            compact_health_name_pixels(&image),
            "compact visible name/bar gap at distance {distance}"
        );
    }
}

fn capture_zoom_frame(app: &mut App, target: &Handle<Image>) -> Image {
    let (sender, receiver) = mpsc::channel();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut pending = false;
    while Instant::now() < deadline {
        app.update();
        if !pending {
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |capture: On<ScreenshotCaptured>| {
                    sender
                        .send(capture.image.clone())
                        .expect("zoom capture receiver alive");
                });
            pending = true;
        }
        if let Ok(image) = receiver.try_recv() {
            pending = false;
            let has_bar =
                colored_pixels(&image, |p| p[0] > 100 && p[1] < 60 && p[2] < 60).is_some();
            let has_name =
                colored_pixels(&image, |p| p[0] > 220 && p[1] > 220 && p[2] > 220).is_some();
            if has_bar && has_name {
                return image;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("GPU zoom capture timed out");
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn nameplate_gpu_reference_thickness_combinations() {
    use crate::rendering::nameplate_cast_bar::NameplateCastBarPlugin;
    use shared::casting::CastState;
    let mut app = configured_render_app(|app| {
        app.add_plugins((crate::health_bar::HealthBarPlugin, NameplateCastBarPlugin));
    });
    let target = render_cameras_sized(&mut app, 979, 364);
    app.update();
    let mut cast = CastState::normal(133, 0, 4.0, true);
    cast.spell_name = "Necrotic Bolt".into();
    cast.elapsed = 1.6;
    let owner = app
        .world_mut()
        .spawn((
            Transform::from_xyz(-9000.0, 0.0, 0.0),
            Visibility::Visible,
            shared::components::Health {
                current: 75.0,
                max: 100.0,
            },
            Npc {
                template_id: 299,
                name: "Zolramus Sorcerer".into(),
            },
            cast,
        ))
        .id();
    let directory = std::path::Path::new("data/diagnostics/nameplate-pixel-match/rendered");
    std::fs::create_dir_all(directory).unwrap();
    for (health, spell, label, center) in [
        (
            NameplateBarThickness::Thick,
            NameplateBarThickness::Thin,
            "thick-thin",
            Vec2::new(250.0, 73.0),
        ),
        (
            NameplateBarThickness::Thick,
            NameplateBarThickness::Thick,
            "thick-thick",
            Vec2::new(252.0, 271.0),
        ),
        (
            NameplateBarThickness::Thin,
            NameplateBarThickness::Thin,
            "thin-thin",
            Vec2::new(721.0, 82.0),
        ),
        (
            NameplateBarThickness::Thin,
            NameplateBarThickness::Thick,
            "thin-thick",
            Vec2::new(723.0, 282.0),
        ),
    ] {
        {
            let mut hud = app.world_mut().resource_mut::<HudOptions>();
            hud.nameplate_health_thickness = health;
            hud.nameplate_spellbar_thickness = spell;
        }
        let camera = app
            .world_mut()
            .query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>()
            .single(app.world())
            .unwrap();
        let ray = camera.0.viewport_to_world(camera.1, center).unwrap();
        let anchor = ray.origin + ray.direction * (-ray.origin.z / ray.direction.z);
        app.world_mut()
            .get_mut::<Transform>(owner)
            .unwrap()
            .translation = anchor - Vec3::Y * NPC_NAMEPLATE_Y;
        for _ in 0..8 {
            app.update();
        }
        let image = capture_zoom_frame(&mut app, &target);
        let red = colored_pixels(&image, |p| p[0] > 100 && p[1] < 60 && p[2] < 60)
            .expect("red health fill");
        let width = image.width() as usize;
        let gold_below_health = image
            .data
            .as_ref()
            .unwrap()
            .chunks_exact(4)
            .enumerate()
            .filter(|(i, p)| i / width > red.bottom + 2 && p[0] > 100 && p[1] > 60 && p[2] < 100)
            .count();
        image
            .try_into_dynamic()
            .unwrap()
            .save(directory.join(format!("{label}.png")))
            .unwrap();
        assert!(
            gold_below_health > 30,
            "{label}: missing gold spellbar below health ({gold_below_health} pixels)"
        );
    }
}

fn white_glyph_pixels(image: &Image) -> usize {
    let pixels = image.data.as_ref().expect("captured pixels");
    let width = image.width() as usize;
    pixels
        .chunks_exact(4)
        .enumerate()
        .filter(|(index, rgba)| {
            let x = index % width;
            let y = index / width;
            (12..width - 12).contains(&x)
                && (56..136).contains(&y)
                && rgba[0] > 220
                && rgba[1] > 220
                && rgba[2] > 220
        })
        .count()
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn nameplate_gpu_world_observer_renders_glyphs_at_projected_anchor() {
    let mut app = render_app();
    let target = cameras_and_wolf(&mut app);
    let (sender, receiver) = mpsc::channel();
    let mut pending = false;
    let mut maximum_glyph_pixels = 0;
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        app.update();
        if !pending {
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |capture: On<ScreenshotCaptured>| {
                    sender.send(capture.image.clone()).unwrap();
                });
            pending = true;
        }
        if let Ok(image) = receiver.try_recv() {
            pending = false;
            maximum_glyph_pixels = maximum_glyph_pixels.max(white_glyph_pixels(&image));
            if maximum_glyph_pixels > 100 {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!(
        "world NPC nameplate produced only {maximum_glyph_pixels} white glyph pixels at its projected anchor"
    );
}
