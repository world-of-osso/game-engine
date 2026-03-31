use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;

use crate::creature_display;
use crate::m2_effect_material::M2EffectMaterial;
use crate::m2_scene;
use crate::target::{TargetCircleStyle, available_circle_styles};
use game_engine::targeting::CurrentTarget;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::inworld_selection_debug_component::{
    InWorldSelectionDebugAction, InWorldSelectionDebugEntry, InWorldSelectionDebugState,
    inworld_selection_debug_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::char_select_input::{cursor_pos, find_clicked_action};
use crate::game_state::GameState;
use crate::networking::RemoteEntity;

#[derive(Component)]
struct InWorldSelectionDebugScene;

#[derive(Debug, Clone, Resource)]
struct InWorldSelectionDebugModel {
    entries: Vec<InWorldSelectionDebugEntry>,
    selected_index: usize,
    pinned: bool,
    last_action: String,
    active_circle_style: usize,
}

struct InWorldSelectionDebugScreenRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for InWorldSelectionDebugScreenRes {}
unsafe impl Sync for InWorldSelectionDebugScreenRes {}

#[derive(Resource)]
struct InWorldSelectionDebugScreenWrap(InWorldSelectionDebugScreenRes);

#[derive(Message)]
struct InWorldSelectionDebugClickEvent(String);

pub struct InWorldSelectionDebugScreenPlugin;

impl Plugin for InWorldSelectionDebugScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<InWorldSelectionDebugClickEvent>();
        app.add_systems(
            OnEnter(GameState::InWorldSelectionDebug),
            (
                build_inworld_selection_debug_ui,
                setup_inworld_selection_debug_scene,
            ),
        );
        app.add_systems(
            OnExit(GameState::InWorldSelectionDebug),
            (
                teardown_inworld_selection_debug_ui,
                teardown_inworld_selection_debug_scene,
            ),
        );
        app.add_systems(
            Update,
            (
                inworld_selection_debug_mouse_input,
                inworld_selection_debug_keyboard_input,
                dispatch_inworld_selection_debug_action,
                sync_inworld_selection_debug_ui,
            )
                .chain()
                .run_if(in_state(GameState::InWorldSelectionDebug)),
        );
    }
}

impl Default for InWorldSelectionDebugModel {
    fn default() -> Self {
        Self {
            entries: vec![
                InWorldSelectionDebugEntry {
                    label: "Enemy Creature".to_string(),
                    category: "Hostile unit".to_string(),
                    target_rule: "required".to_string(),
                    detail: "Validates target ring visibility, hostile target naming, and the expected spell path when a cast requires CurrentTarget.".to_string(),
                },
                InWorldSelectionDebugEntry {
                    label: "Quest NPC".to_string(),
                    category: "Friendly unit".to_string(),
                    target_rule: "optional".to_string(),
                    detail: "Useful for checking non-hostile selection visuals, talk interaction affordances, and whether the target frame still reads as selected instead of hovered.".to_string(),
                },
                InWorldSelectionDebugEntry {
                    label: "World Object".to_string(),
                    category: "Interactable prop".to_string(),
                    target_rule: "optional".to_string(),
                    detail: "Checks mailbox or chest targeting semantics where selection exists but combat-style target handling should stay conservative.".to_string(),
                },
                InWorldSelectionDebugEntry {
                    label: "Invalid / Stale Entity".to_string(),
                    category: "Selection edge case".to_string(),
                    target_rule: "forbidden".to_string(),
                    detail: "Represents an entity id that was previously selected but no longer resolves cleanly, which is where stale selection cleanup bugs usually surface.".to_string(),
                },
            ],
            selected_index: 0,
            pinned: false,
            last_action: "Initialized in-world selection debug screen".to_string(),
            active_circle_style: 0,
        }
    }
}

fn build_inworld_selection_debug_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);

    let model = InWorldSelectionDebugModel::default();
    let mut shared = SharedContext::new();
    shared.insert(inworld_selection_debug_state(&model));
    let mut screen = Screen::new(inworld_selection_debug_screen);
    screen.sync(&shared, &mut ui.registry);

    commands.insert_resource(InWorldSelectionDebugScreenWrap(
        InWorldSelectionDebugScreenRes { screen, shared },
    ));
    commands.insert_resource(model);
}

fn setup_inworld_selection_debug_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inverse_bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<creature_display::CreatureDisplayMap>,
    mut current_target: ResMut<CurrentTarget>,
) {
    commands.insert_resource(ClearColor(Color::srgb(0.03, 0.04, 0.06)));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.8, 0.85, 0.95),
        brightness: 150.0,
        ..default()
    });
    spawn_debug_camera(&mut commands);
    spawn_debug_light(&mut commands);
    spawn_debug_ground(&mut commands, &mut meshes, &mut materials);
    current_target.0 = spawn_debug_wolf(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut effect_materials,
        &mut images,
        &mut inverse_bindposes,
        &creature_display_map,
    );
}

fn spawn_debug_camera(commands: &mut Commands) {
    commands.spawn((
        Name::new("InWorldSelectionDebugCamera"),
        InWorldSelectionDebugScene,
        Camera3d::default(),
        Transform::from_xyz(-3.2, 2.6, 6.6).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));
}

fn spawn_debug_light(commands: &mut Commands) {
    commands.spawn((
        Name::new("InWorldSelectionDebugLight"),
        InWorldSelectionDebugScene,
        DirectionalLight {
            illuminance: 9000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, -0.6, 0.0)),
    ));
}

fn spawn_debug_ground(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Name::new("InWorldSelectionDebugGround"),
        InWorldSelectionDebugScene,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0).build())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.14, 0.17),
            perceptual_roughness: 0.95,
            metallic: 0.05,
            ..default()
        })),
    ));
}

fn spawn_debug_wolf(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    images: &mut Assets<Image>,
    inverse_bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    creature_display_map: &creature_display::CreatureDisplayMap,
) -> Option<Entity> {
    let wolf_path = std::path::Path::new("data/models/126487.m2");
    let spawned = m2_scene::spawn_animated_static_m2_parts(
        commands,
        meshes,
        materials,
        effect_materials,
        images,
        inverse_bindposes,
        wolf_path,
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        creature_display_map,
    )?;
    commands.entity(spawned.root).insert((
        InWorldSelectionDebugScene,
        RemoteEntity,
        Name::new("InWorldSelectionDebugWolf"),
    ));
    commands
        .entity(spawned.model_root)
        .insert(InWorldSelectionDebugScene);
    Some(spawned.root)
}

fn teardown_inworld_selection_debug_ui(
    mut ui: ResMut<UiState>,
    mut screen: Option<ResMut<InWorldSelectionDebugScreenWrap>>,
    mut commands: Commands,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<InWorldSelectionDebugScreenWrap>();
    commands.remove_resource::<InWorldSelectionDebugModel>();
    ui.focused_frame = None;
}

fn teardown_inworld_selection_debug_scene(
    mut commands: Commands,
    scene_entities: Query<Entity, With<InWorldSelectionDebugScene>>,
    mut current_target: ResMut<CurrentTarget>,
) {
    current_target.0 = None;
    for entity in &scene_entities {
        commands.entity(entity).despawn();
    }
}

fn inworld_selection_debug_state(model: &InWorldSelectionDebugModel) -> InWorldSelectionDebugState {
    InWorldSelectionDebugState {
        entries: model.entries.clone(),
        selected_index: model.selected_index,
        pinned: model.pinned,
        last_action: model.last_action.clone(),
        circle_styles: available_circle_styles().iter().map(|s| s.label().to_string()).collect(),
        active_circle_style: model.active_circle_style,
    }
}

fn inworld_selection_debug_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui: Res<UiState>,
    mut events: MessageWriter<InWorldSelectionDebugClickEvent>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = cursor_pos(&windows) else {
        return;
    };
    if let Some(action) = find_clicked_action(&ui, cursor.x, cursor.y) {
        events.write(InWorldSelectionDebugClickEvent(action));
    }
}

fn inworld_selection_debug_keyboard_input(
    mut key_events: MessageReader<KeyboardInput>,
    mut events: MessageWriter<InWorldSelectionDebugClickEvent>,
) {
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        let action = match event.key_code {
            KeyCode::ArrowUp | KeyCode::ArrowLeft => Some(InWorldSelectionDebugAction::Prev),
            KeyCode::ArrowDown | KeyCode::ArrowRight => Some(InWorldSelectionDebugAction::Next),
            KeyCode::Enter | KeyCode::Space => Some(InWorldSelectionDebugAction::TogglePinned),
            KeyCode::Escape => Some(InWorldSelectionDebugAction::Back),
            _ => None,
        };
        if let Some(action) = action {
            events.write(InWorldSelectionDebugClickEvent(action.to_string()));
        }
    }
}

fn dispatch_inworld_selection_debug_action(
    mut events: MessageReader<InWorldSelectionDebugClickEvent>,
    mut model: ResMut<InWorldSelectionDebugModel>,
    mut next_state: ResMut<NextState<GameState>>,
    mut circle_style: ResMut<TargetCircleStyle>,
) {
    for event in events.read() {
        match InWorldSelectionDebugAction::parse(&event.0) {
            Some(InWorldSelectionDebugAction::SelectEntry(index)) => {
                select_entry(&mut model, index)
            }
            Some(InWorldSelectionDebugAction::SelectCircleStyle(index)) => {
                apply_circle_style(&mut model, &mut circle_style, index);
            }
            Some(InWorldSelectionDebugAction::Prev) => cycle_entry(&mut model, -1),
            Some(InWorldSelectionDebugAction::Next) => cycle_entry(&mut model, 1),
            Some(InWorldSelectionDebugAction::TogglePinned) => toggle_pin(&mut model),
            Some(InWorldSelectionDebugAction::Back) => next_state.set(GameState::Login),
            None => {}
        }
    }
}

fn apply_circle_style(
    model: &mut InWorldSelectionDebugModel,
    circle_style: &mut TargetCircleStyle,
    index: usize,
) {
    let styles = available_circle_styles();
    if let Some(style) = styles.into_iter().nth(index) {
        model.active_circle_style = index;
        model.last_action = format!("Circle: {}", style.label());
        *circle_style = style;
    }
}

fn select_entry(model: &mut InWorldSelectionDebugModel, index: usize) {
    if index >= model.entries.len() {
        return;
    }
    model.selected_index = index;
    model.last_action = format!("Selected {}", current_label(model));
}

fn cycle_entry(model: &mut InWorldSelectionDebugModel, delta: isize) {
    let count = model.entries.len();
    if count == 0 {
        return;
    }
    let count = count as isize;
    let current = model.selected_index as isize;
    let next = (current + delta).rem_euclid(count) as usize;
    model.selected_index = next;
    model.last_action = format!("Focused {}", current_label(model));
}

fn toggle_pin(model: &mut InWorldSelectionDebugModel) {
    model.pinned = !model.pinned;
    let label = current_label(model);
    model.last_action = if model.pinned {
        format!("Pinned {label}")
    } else {
        format!("Unpinned {label}")
    };
}

fn current_label(model: &InWorldSelectionDebugModel) -> &str {
    model
        .entries
        .get(model.selected_index)
        .map(|entry| entry.label.as_str())
        .unwrap_or("Unknown")
}

fn sync_inworld_selection_debug_ui(
    model: Res<InWorldSelectionDebugModel>,
    mut screen: Option<ResMut<InWorldSelectionDebugScreenWrap>>,
    mut ui: ResMut<UiState>,
) {
    if !model.is_changed() {
        return;
    }
    let Some(screen) = screen.as_mut() else {
        return;
    };
    let wrap = &mut screen.0;
    wrap.shared.insert(inworld_selection_debug_state(&model));
    let shared = &wrap.shared;
    wrap.screen.sync(shared, &mut ui.registry);
    ui.focused_frame = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_entry_wraps() {
        let mut model = InWorldSelectionDebugModel::default();
        cycle_entry(&mut model, -1);
        assert_eq!(model.selected_index, model.entries.len() - 1);
        cycle_entry(&mut model, 1);
        assert_eq!(model.selected_index, 0);
    }

    #[test]
    fn toggle_pin_updates_last_action() {
        let mut model = InWorldSelectionDebugModel::default();
        toggle_pin(&mut model);
        assert!(model.pinned);
        assert_eq!(model.last_action, "Pinned Enemy Creature");
    }
}
