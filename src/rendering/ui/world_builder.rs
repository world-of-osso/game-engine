use std::collections::{HashMap, HashSet};

use bevy::asset::AssetId;
use bevy::ecs::{entity_disabling::Disabled, query::Allow};
use bevy::input::{ButtonState, keyboard::KeyboardInput};
use bevy::prelude::*;
use game_engine::ui::{
    frame::WidgetData,
    input::find_frame_at,
    plugin::UiState,
    screens::world_builder_component::{
        WORLD_BUILDER_FILTER, WORLD_BUILDER_ROOT, WorldBuilderAction,
        WorldBuilderDirectionalLightState, WorldBuilderPointLightState, WorldBuilderPropertyState,
        WorldBuilderRow, WorldBuilderViewState, world_builder_directional_light_edit_name,
        world_builder_point_light_edit_name, world_builder_rotation_edit_name,
        world_builder_scale_edit_name, world_builder_screen, world_builder_transform_edit_name,
    },
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::{
    game_state::GameState,
    m2_effect_material::{
        M2EffectMaterial, M2EffectUvUpdatesEnabled, update_m2_effect_material_uv,
    },
};

const MIN_EDITABLE_SCALE: f32 = 0.001;
const MAX_EDITABLE_SCALE: f32 = 10_000.0;
const WORLD_BUILDER_PAGE_SIZE: usize = 6;

#[derive(Clone, Debug)]
struct SceneEntityNode {
    entity: Entity,
    parent: Option<Entity>,
    children: Vec<Entity>,
    label: String,
    component_names: Vec<String>,
    translation: Vec3,
    rotation_degrees: Vec3,
    scale: Vec3,
    point_light: Option<PointLightSnapshot>,
    directional_light: Option<DirectionalLightSnapshot>,
}

#[derive(Clone, Copy, Debug)]
struct PointLightSnapshot {
    intensity: f32,
    range: f32,
    shadows_enabled: bool,
}

#[derive(Clone, Copy, Debug)]
struct DirectionalLightSnapshot {
    illuminance: f32,
    shadows_enabled: bool,
}

#[derive(Clone, Debug, Default)]
struct SceneSnapshot {
    roots: Vec<Entity>,
    entities: HashMap<Entity, SceneEntityNode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VisibleSceneRow {
    entity: Entity,
    depth: usize,
}

#[derive(Component, Clone, Copy, Debug)]
struct WorldBuilderRenderSuppressed {
    previous: Visibility,
}

#[derive(Component, Clone, Copy, Debug)]
struct WorldBuilderProcessingSuspended {
    remove_disabled_on_restore: bool,
}

#[derive(Clone, Copy, Debug)]
struct TransformEdit {
    translation: Vec3,
    rotation_degrees: Vec3,
    scale: Vec3,
}

#[derive(Clone, Copy, Debug)]
struct PointLightEdit {
    intensity: f32,
    range: f32,
}

#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct WorldBuilderEnabled;

#[derive(Resource, Debug, Clone, Copy)]
struct WorldBuilderM2UvUpdatesEnabled(bool);

#[derive(Resource, Debug, Default)]
struct WorldBuilderSuppressedM2Materials(HashSet<AssetId<M2EffectMaterial>>);

pub struct WorldBuilderPlugin;

impl Plugin for WorldBuilderPlugin {
    fn build(&self, app: &mut App) {
        let m2_uv_updates_enabled = app
            .world()
            .get_resource::<M2EffectUvUpdatesEnabled>()
            .is_none_or(|enabled| enabled.0);
        app.insert_resource(WorldBuilderEnabled)
            .insert_resource(WorldBuilderM2UvUpdatesEnabled(m2_uv_updates_enabled))
            .insert_resource(M2EffectUvUpdatesEnabled(false))
            .init_resource::<WorldBuilderModel>()
            .init_resource::<WorldBuilderWorldActionQueue>()
            .init_resource::<WorldBuilderSuppressedM2Materials>()
            .add_message::<WorldBuilderClickEvent>()
            .add_systems(OnEnter(GameState::InWorld), build_world_builder_ui)
            .add_systems(OnExit(GameState::InWorld), exit_world_builder)
            .add_systems(
                Update,
                (
                    toggle_world_builder_hotkey,
                    world_builder_mouse_input,
                    world_builder_keyboard_input,
                    dispatch_world_builder_actions,
                    process_world_builder_world,
                    sync_world_builder_ui,
                )
                    .chain()
                    .run_if(in_state(GameState::InWorld)),
            )
            .add_systems(Update, update_world_builder_m2_effect_uvs)
            .add_systems(
                PostUpdate,
                enforce_render_suppression.run_if(in_state(GameState::InWorld)),
            );
    }
}

#[derive(Resource, Debug)]
struct WorldBuilderModel {
    open: bool,
    snapshot: SceneSnapshot,
    expanded: HashSet<Entity>,
    selected: Option<Entity>,
    render_roots: HashSet<Entity>,
    processing_roots: HashSet<Entity>,
    filter: String,
    page_index: usize,
    snapshot_dirty: bool,
    overrides_dirty: bool,
    status: String,
}

impl Default for WorldBuilderModel {
    fn default() -> Self {
        Self {
            open: true,
            snapshot: SceneSnapshot::default(),
            expanded: HashSet::new(),
            selected: None,
            render_roots: HashSet::new(),
            processing_roots: HashSet::new(),
            filter: String::new(),
            page_index: 0,
            snapshot_dirty: true,
            overrides_dirty: false,
            status: "Ready. Refresh before recording FPS.".to_string(),
        }
    }
}

struct WorldBuilderScreenRes {
    screen: Screen,
    shared: SharedContext,
}

struct WorldBuilderScreenWrap(WorldBuilderScreenRes);

#[derive(Message)]
struct WorldBuilderClickEvent(String);

#[derive(Debug)]
enum WorldBuilderWorldAction {
    ApplyTransform(Entity, TransformEdit),
    ApplyPointLight(Entity, PointLightEdit),
    ApplyDirectionalLight(Entity, f32),
    TogglePointLightShadows(Entity),
    ToggleDirectionalLightShadows(Entity),
}

#[derive(Resource, Debug, Default)]
struct WorldBuilderWorldActionQueue(Vec<WorldBuilderWorldAction>);

#[path = "world_builder/core.rs"]
mod core;
#[path = "world_builder/runtime.rs"]
mod runtime;

use core::*;
use runtime::*;

#[cfg(test)]
#[path = "world_builder/tests.rs"]
mod tests;
