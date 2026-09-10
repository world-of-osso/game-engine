use super::*;
use bevy::camera::RenderTarget;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use ui_toolkit::render::UiCamera;

const OUTPUT: &str = "data/diagnostics/player-frame-fit-20260910";

fn frame_app() -> (App, Handle<Image>, Screen, SharedContext) {
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
    app.add_plugins(ui_toolkit::plugin::UiPlugin);
    app.world_mut().resource_mut::<UiState>().registry =
        ui_toolkit::registry::FrameRegistry::new(800.0, 400.0);
    app.finish();
    app.cleanup();
    app.update();
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            800,
            400,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    let camera = app
        .world_mut()
        .query_filtered::<Entity, With<UiCamera>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(camera)
        .insert((RenderTarget::Image(target.clone().into()), Msaa::Off));
    app.world_mut()
        .get_mut::<Camera>(camera)
        .unwrap()
        .clear_color = Color::srgb(0.04, 0.07, 0.06).into();
    let player = NetPlayer {
        name: "Theron".into(),
        race: 1,
        class: 1,
        appearance: default(),
    };
    let health = NetHealth {
        current: 80.0,
        max: 100.0,
    };
    let mana = NetMana {
        current: 30.0,
        max: 60.0,
    };
    let state = build_player_state(
        None,
        (Some(&player), Some(&health), Some(&mana), None, None, None),
    );
    let mut shared = SharedContext::new();
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: false,
        player: state,
        target: None,
    });
    let mut screen = Screen::new(inworld_unit_frames_screen);
    screen.sync(
        &shared,
        &mut app.world_mut().resource_mut::<UiState>().registry,
    );
    (app, target, screen, shared)
}

fn capture(app: &mut App, target: &Handle<Image>) -> Image {
    for _ in 0..6 {
        app.update();
    }
    let deadline = Instant::now() + Duration::from_secs(8);
    let (sender, receiver) = mpsc::channel();
    let mut pending = false;
    while Instant::now() < deadline {
        app.update();
        if !pending {
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |event: On<ScreenshotCaptured>| {
                    sender
                        .send(event.image.clone())
                        .expect("frame capture receiver alive");
                });
            pending = true;
        }
        if let Ok(image) = receiver.try_recv() {
            pending = false;
            let gold = image
                .data
                .as_ref()
                .unwrap()
                .chunks_exact(4)
                .filter(|p| p[0] > 100 && p[0] > p[1] && p[1] > p[2].saturating_add(20))
                .count();
            if gold > 100 {
                return image;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("player frame artwork failed to render");
}

fn rect(app: &App, name: &str) -> ui_toolkit::layout::LayoutRect {
    let registry = &app.world().resource::<UiState>().registry;
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
        .layout_rect
        .clone()
        .unwrap()
}

fn save(image: &Image, name: &str) {
    std::fs::create_dir_all(OUTPUT).unwrap();
    image
        .clone()
        .try_into_dynamic()
        .unwrap()
        .save(format!("{OUTPUT}/{name}.png"))
        .unwrap();
}

fn green_fill_width(image: &Image, area: &ui_toolkit::layout::LayoutRect) -> usize {
    let mut left = usize::MAX;
    let mut right = 0;
    for y in area.y.ceil() as usize..(area.y + area.height).floor() as usize {
        for x in area.x.ceil() as usize..(area.x + area.width).floor() as usize {
            let offset = (y * image.width() as usize + x) * 4;
            let p = &image.data.as_ref().unwrap()[offset..offset + 4];
            if p[1] > p[0].saturating_add(30) && p[1] > p[2].saturating_add(20) {
                left = left.min(x);
                right = right.max(x);
            }
        }
    }
    if left == usize::MAX {
        0
    } else {
        right - left + 1
    }
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn player_frame_gpu_contents_fit_artwork_and_health_updates() {
    let (mut app, target, mut screen, mut shared) = frame_app();
    let image = capture(&mut app, &target);
    save(&image, "player-frame-rendered");
    let root = rect(&app, "PlayerFrame");
    let shell = rect(&app, "PlayerFrameTexture");
    assert!(
        (root.width - 297.0).abs() < 0.01 && (root.height - 106.5).abs() < 0.01,
        "smaller complete frame: {root:?}"
    );
    assert_eq!(
        root, shell,
        "artwork and contents must share one coordinate space"
    );
    let portrait = rect(&app, "PlayerPortraitTexture");
    let mut white = 0;
    let mut colored = 0;
    for y in portrait.y.ceil() as usize..(portrait.y + portrait.height).floor() as usize {
        for x in portrait.x.ceil() as usize..(portrait.x + portrait.width).floor() as usize {
            let offset = (y * image.width() as usize + x) * 4;
            let p = &image.data.as_ref().unwrap()[offset..offset + 4];
            white += usize::from(p[0] > 240 && p[1] > 240 && p[2] > 240);
            colored += usize::from(p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]) > 30);
        }
    }
    assert!(
        white < (portrait.width * portrait.height * 0.1) as usize && colored > 300,
        "portrait must contain cached artwork, not white placeholder: white={white} colored={colored}"
    );
    let full_width = rect(&app, "PlayerHealthBar").width;
    let first = rect(&app, "PlayerHealthBarFill");
    assert!((first.width - full_width * 0.8).abs() < 0.01);
    let mut state = shared.get::<InWorldUnitFramesState>().unwrap().clone();
    state.player.health_fill_width = full_width * 0.2;
    state.player.health_text = "20 / 100".into();
    shared.insert(state);
    screen.sync(
        &shared,
        &mut app.world_mut().resource_mut::<UiState>().registry,
    );
    let updated = capture(&mut app, &target);
    save(&updated, "player-frame-health-update");
    let final_fill = rect(&app, "PlayerHealthBarFill");
    assert!((final_fill.width - full_width * 0.2).abs() < 0.01);
    let health_area = rect(&app, "PlayerHealthBar");
    let first_pixels = green_fill_width(&image, &health_area);
    let final_pixels = green_fill_width(&updated, &health_area);
    assert!(
        (first_pixels as f32 - full_width * 0.8).abs() < 3.0,
        "80% rendered fill width: {first_pixels}"
    );
    assert!(
        (final_pixels as f32 - full_width * 0.2).abs() < 3.0,
        "20% rendered fill width: {final_pixels}"
    );
    let mut empty = shared.get::<InWorldUnitFramesState>().unwrap().clone();
    empty.player.health_fill_width = 0.0;
    empty.player.health_text = "0 / 100".into();
    shared.insert(empty);
    screen.sync(
        &shared,
        &mut app.world_mut().resource_mut::<UiState>().registry,
    );
    let empty_image = capture(&mut app, &target);
    save(&empty_image, "player-frame-empty-health");
    assert_eq!(
        green_fill_width(&empty_image, &health_area),
        0,
        "empty health must not draw a stale fill"
    );
    let mut full = shared.get::<InWorldUnitFramesState>().unwrap().clone();
    full.player.health_fill_width = full_width;
    full.player.mana_fill_width = rect(&app, "PlayerManaBar").width;
    full.player.health_text = "100 / 100".into();
    full.player.mana_text = "60 / 60".into();
    shared.insert(full);
    screen.sync(
        &shared,
        &mut app.world_mut().resource_mut::<UiState>().registry,
    );
    let full_image = capture(&mut app, &target);
    save(&full_image, "player-frame-full");
    assert!(
        (green_fill_width(&full_image, &health_area) as f32 - full_width).abs() < 3.0,
        "full health must reach the artwork opening's right edge"
    );
}
