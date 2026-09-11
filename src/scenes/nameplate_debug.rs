//! Offline fixtures using the same overhead renderers as the game world.
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;
use shared::casting::CastState;
use shared::components::Health;
use ui_toolkit::render::UI_RENDER_LAYER;

use crate::game_state::GameState;
use crate::rendering::nameplate_art::{NAME_FONT_SIZE, NameplateArtCache};
use game_engine::targeting::CurrentTarget;

#[derive(Component)]
pub(crate) struct PreviewUnit;

#[derive(Component)]
struct PreviewInstructions;

#[derive(Resource, Default)]
struct PreviewPlayback {
    paused: bool,
}

pub struct NameplateDebugPlugin;

impl Plugin for NameplateDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PreviewPlayback>()
            .init_resource::<CurrentTarget>()
            .add_systems(OnEnter(GameState::NameplateDebug), spawn_preview)
            .add_systems(OnExit(GameState::NameplateDebug), clear_preview_target)
            .add_systems(
                Update,
                (
                    toggle_preview_pause,
                    advance_preview_casts,
                    update_instructions,
                )
                    .chain()
                    .run_if(in_state(GameState::NameplateDebug)),
            );
    }
}

fn spawn_preview(
    mut commands: Commands,
    mut playback: ResMut<PreviewPlayback>,
    mut art: ResMut<NameplateArtCache>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
) {
    playback.paused = false;
    let font = art
        .load(&mut images, &mut fonts)
        .unwrap_or_else(|error| panic!("Cannot load nameplate preview artwork: {error}"))
        .font;
    spawn_preview_camera(&mut commands);
    for (x, name, health, channel) in [
        (-3.8, "Zolramus Sorcerer", 76.3, false),
        (0.0, "Channeling Adept", 45.0, true),
        (3.8, "Training Guardian", 100.0, false),
    ] {
        spawn_preview_unit(&mut commands, font.clone(), x, name, health, channel);
    }
    spawn_instructions(&mut commands);
}

fn spawn_preview_camera(commands: &mut Commands) {
    commands.spawn((
        Name::new("Nameplate preview camera"),
        Camera3d::default(),
        Camera {
            clear_color: Color::srgb(0.094, 0.082, 0.078).into(),
            ..default()
        },
        Transform::from_xyz(0.0, 2.5, 12.0).looking_at(Vec3::new(0.0, 2.5, 0.0), Vec3::Y),
        DespawnOnExit(GameState::NameplateDebug),
    ));
}

fn spawn_instructions(commands: &mut Commands) {
    commands.spawn((
        PreviewInstructions,
        Text2d::new("Nameplate preview — Space: pause/resume — Click a plate to select"),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Anchor::TOP_LEFT,
        RenderLayers::layer(UI_RENDER_LAYER),
        Transform::from_xyz(-400.0, 260.0, 10.0),
        DespawnOnExit(GameState::NameplateDebug),
    ));
}

fn spawn_preview_unit(
    commands: &mut Commands,
    font: Handle<Font>,
    x: f32,
    name: &str,
    health: f32,
    channel: bool,
) {
    let owner = commands
        .spawn((
            PreviewUnit,
            Name::new(name.to_owned()),
            Transform::from_xyz(x, 0.0, 0.0),
            Visibility::Visible,
            Health {
                current: health,
                max: 100.0,
            },
            demo_cast(channel),
            DespawnOnExit(GameState::NameplateDebug),
        ))
        .id();
    crate::nameplate::spawn_nameplate_entity(
        commands,
        owner,
        name,
        Color::WHITE,
        NAME_FONT_SIZE,
        font,
        2.5,
        crate::nameplate::NameplateKind::Npc,
    );
}

fn demo_cast(channel: bool) -> CastState {
    let mut cast = if channel {
        CastState::channel(5143, 0, 6.0, 1.0, true)
    } else {
        CastState::normal(133, 0, 5.0, true)
    };
    cast.spell_name = if channel {
        "Arcane Missiles"
    } else {
        "Necrotic Bolt"
    }
    .into();
    cast.elapsed = cast.duration * 0.4;
    cast
}

fn toggle_preview_pause(keys: Res<ButtonInput<KeyCode>>, mut playback: ResMut<PreviewPlayback>) {
    if keys.just_pressed(KeyCode::Space) {
        playback.paused = !playback.paused;
    }
}

fn advance_preview_casts(
    time: Res<Time>,
    playback: Res<PreviewPlayback>,
    mut casts: Query<&mut CastState, With<PreviewUnit>>,
) {
    if playback.paused {
        return;
    }
    for mut cast in &mut casts {
        cast.elapsed = (cast.elapsed + time.delta_secs()) % cast.duration;
    }
}

fn update_instructions(
    windows: Query<&Window, With<PrimaryWindow>>,
    playback: Res<PreviewPlayback>,
    target: Res<CurrentTarget>,
    units: Query<&Name, With<PreviewUnit>>,
    mut labels: Query<(&mut Text2d, &mut Transform), With<PreviewInstructions>>,
) {
    let selected = target
        .0
        .and_then(|entity| units.get(entity).ok())
        .map_or("none", Name::as_str);
    let state = if playback.paused { "Paused" } else { "Playing" };
    let caption = format!(
        "Nameplate preview — {state}\nSpace: pause/resume · Click a plate to select\nSelected: {selected}"
    );
    for (mut text, mut transform) in &mut labels {
        if text.0 != caption {
            text.0.clone_from(&caption);
        }
        if let Ok(window) = windows.single() {
            let position = Vec3::new(
                -window.width() / 2.0 + 24.0,
                window.height() / 2.0 - 24.0,
                10.0,
            );
            if transform.translation != position {
                transform.translation = position;
            }
        }
    }
}

fn clear_preview_target(mut target: ResMut<CurrentTarget>, units: Query<(), With<PreviewUnit>>) {
    if target.0.is_some_and(|entity| units.contains(entity)) {
        target.0 = None;
    }
}

#[cfg(test)]
#[path = "nameplate_debug_tests.rs"]
mod tests;
