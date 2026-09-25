use bevy::ecs::system::SystemParam;
use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::loot_state::{LootRequest, Lootable, NpcRightClick, auto_loot, npc_right_click};
use game_engine::mail_data::MailIntentQueue;
use game_engine::quest_tracking::QuestTrackedItem;
use game_engine::targeting::CurrentTarget;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::UiState;
use shared::components::{Health as NetHealth, Npc};
use shared::protocol::{EmoteIntent, EmoteKind};

use crate::camera::Player;
use crate::game_state::GameState;
use crate::networking::RemoteEntity;
use game_engine::input_bindings::{InputAction, InputBindings};

type RemoteTargetQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Transform, Option<&'static Visibility>),
    (With<RemoteEntity>, With<Npc>, Without<Player>),
>;

#[cfg(test)]
#[path = "target_nameplate_tests.rs"]
mod nameplate_click_tests;
#[path = "target_visuals.rs"]
mod target_visuals;
#[path = "target_zone_transition.rs"]
mod target_zone_transition;

use target_visuals::{spawn_target_circle, update_target_circle};
use target_zone_transition::{
    ZoneTransitionContactState, reset_zone_transition_contact, trigger_zone_transition_on_collision,
};
#[cfg(test)]
use target_zone_transition::{player_inside_zone_transition, update_zone_transition_contact};

/// Marker on the selection circle entity.
#[derive(Component)]
pub struct TargetMarker;

#[derive(Component, Clone, Copy)]
struct TargetMarkerScaleFactor(f32);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldObjectInteraction {
    pub kind: WorldObjectInteractionKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatherNodeKind {
    CopperVein,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldObjectInteractionKind {
    Mailbox,
    Forge,
    Anvil,
    Chair,
    GatherNode(GatherNodeKind),
    ZoneTransition,
    QuestObject,
    /// A replicated server game object (`GameObjectInfo`); using it sends
    /// `UseGameObject` and the server checks the reach.
    ServerObject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteractionTarget {
    Npc(Entity),
    Object(Entity, WorldObjectInteractionKind),
}

#[derive(SystemParam)]
struct RightClickInteractionState<'w, 's> {
    parent_query: Query<'w, 's, &'static ChildOf>,
    npc_entities: Query<'w, 's, Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
    object_q: Query<'w, 's, &'static WorldObjectInteraction>,
    quest_q: Query<'w, 's, (), With<QuestTrackedItem>>,
    visibility_q: Query<'w, 's, &'static Visibility>,
    player_q: Query<'w, 's, &'static GlobalTransform, With<Player>>,
    npc_q: Query<'w, 's, &'static GlobalTransform, With<Npc>>,
    object_tf_q: Query<
        'w,
        's,
        &'static GlobalTransform,
        Or<(With<WorldObjectInteraction>, With<QuestTrackedItem>)>,
    >,
    current: ResMut<'w, CurrentTarget>,
    interactions: MessageWriter<'w, crate::networking_quests::NpcInteractionRequest>,
    mail_queue: ResMut<'w, MailIntentQueue>,
    window_manager: Option<ResMut<'w, crate::window_manager::WindowManager>>,
    emote_input: Option<ResMut<'w, crate::networking::EmoteInput>>,
    corpses: Query<'w, 's, (&'static NetHealth, Has<Lootable>)>,
    loot: MessageWriter<'w, LootRequest>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    hud: Option<Res<'w, crate::client_options::HudOptions>>,
}

/// Which visual style the target selection circle uses.
#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub enum TargetCircleStyle {
    /// Procedural yellow ring + fill (no texture).
    Procedural,
    /// BLP texture by FDID pair: (base, optional glow).
    Blp {
        name: String,
        base_fdid: u32,
        glow_fdid: Option<u32>,
        emissive: [u8; 3],
    },
}

impl Default for TargetCircleStyle {
    fn default() -> Self {
        blp_style("Fat Ring", 167207, None, [255, 220, 50])
    }
}

impl TargetCircleStyle {
    pub fn label(&self) -> &str {
        match self {
            Self::Procedural => "Procedural",
            Self::Blp { name, .. } => name,
        }
    }
}

fn blp_style(name: &str, base: u32, glow: Option<u32>, rgb: [u8; 3]) -> TargetCircleStyle {
    TargetCircleStyle::Blp {
        name: name.into(),
        base_fdid: base,
        glow_fdid: glow,
        emissive: rgb,
    }
}

/// All available circle styles for the debug picker.
pub fn available_circle_styles() -> Vec<TargetCircleStyle> {
    let mut styles = vec![TargetCircleStyle::Procedural];
    styles.extend(white_ring_styles());
    styles.extend(spell_area_styles());
    styles
}

fn white_ring_styles() -> Vec<TargetCircleStyle> {
    vec![
        blp_style("Thin Ring (Hostile)", 167208, None, [255, 40, 40]),
        blp_style("Thin Ring (Friendly)", 167208, None, [40, 255, 40]),
        blp_style("Thin Ring (Neutral)", 167208, None, [255, 220, 50]),
        blp_style("Fat Ring", 167207, None, [255, 220, 50]),
        blp_style("Ring Glow", 651522, None, [255, 220, 50]),
        blp_style("Double Ring", 623667, None, [255, 220, 50]),
        blp_style("Reticle", 166706, None, [255, 255, 255]),
    ]
}

fn spell_area_styles() -> Vec<TargetCircleStyle> {
    vec![
        blp_style("Holy", 1001694, None, [255, 240, 150]),
        blp_style("Fire", 1001600, None, [255, 120, 30]),
        blp_style("Arcane", 1001690, None, [180, 130, 255]),
        blp_style("Frost", 1001693, None, [100, 200, 255]),
        blp_style("Nature", 1001695, None, [100, 220, 80]),
        blp_style("Shadow", 1001697, None, [160, 80, 220]),
    ]
}

pub struct TargetPlugin;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentTarget>();
        app.init_resource::<TargetCircleStyle>();
        app.add_message::<crate::networking_quests::NpcInteractionRequest>();
        app.add_message::<LootRequest>();
        app.init_resource::<MailIntentQueue>();
        app.init_resource::<ZoneTransitionContactState>();
        let stage = crate::game::inworld_scene_stage::configured_inworld_scene_stage_for_app(app);
        if stage == crate::game::inworld_scene_stage::InWorldSceneStage::Empty {
            return;
        }
        app.add_systems(OnEnter(GameState::InWorld), reset_zone_transition_contact);
        app.add_systems(Update, click_to_target.run_if(click_targeting_state_active));
        app.add_systems(Update, tab_target.run_if(targeting_state_active));
        app.add_systems(Update, self_target.run_if(targeting_state_active));
        app.add_systems(Update, right_click_interact.run_if(targeting_state_active));
        app.add_systems(
            Update,
            trigger_zone_transition_on_collision.run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(Update, spawn_target_circle.run_if(targeting_state_active));
        app.add_systems(Update, update_target_circle.run_if(targeting_state_active));
    }
}

fn click_targeting_state_active(state: Res<State<GameState>>) -> bool {
    *state.get() == GameState::NameplateDebug || targeting_state_active(state)
}

fn targeting_state_active(state: Res<State<GameState>>) -> bool {
    matches!(
        *state.get(),
        GameState::InWorld | GameState::InWorldSelectionDebug
    )
}

pub(crate) fn classify_world_object_model(model: &str) -> Option<WorldObjectInteractionKind> {
    use std::path::Path;

    let normalized = model.to_ascii_lowercase();
    let stem = Path::new(&normalized)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(&normalized);

    if stem.contains("mailbox") {
        return Some(WorldObjectInteractionKind::Mailbox);
    }
    if stem.contains("copper_miningnode") {
        return Some(WorldObjectInteractionKind::GatherNode(
            GatherNodeKind::CopperVein,
        ));
    }
    if stem.contains("darkportal")
        || stem.contains("landingpad")
        || stem.contains("teleport")
        || (stem.contains("portal") && !stem.contains("antiportal"))
    {
        return Some(WorldObjectInteractionKind::ZoneTransition);
    }
    if stem.contains("anvil") && !stem.contains("anvilmar") {
        return Some(WorldObjectInteractionKind::Anvil);
    }
    if (stem.contains("forge") || stem.contains("blacksmith"))
        && !stem.contains("ironforge")
        && !stem.contains("forgerope")
        && !stem.contains("footbridge")
    {
        return Some(WorldObjectInteractionKind::Forge);
    }
    if stem.contains("chair")
        || stem.contains("bench")
        || stem.contains("stool")
        || stem.contains("seat")
    {
        return Some(WorldObjectInteractionKind::Chair);
    }
    None
}

/// Raycast from camera through mouse cursor on left-click. Target the hit RemoteEntity.
fn click_to_target(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut ray_cast: MeshRayCast,
    plate_picker: crate::rendering::nameplate_picking::NameplatePicker,
    parent_query: Query<&ChildOf>,
    remote_q: Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
    visibility_q: Query<&Visibility>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui_state: Option<Res<UiState>>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if ui_state
        .as_deref()
        .is_some_and(|ui| cursor_over_ui(ui, window))
    {
        return;
    }
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
    if !camera.is_active {
        return;
    }
    if let Some(owner) = plate_picker.pick(cursor, cam_tf.translation()) {
        current.0 = Some(owner);
        return;
    }
    let Some(ray) = camera.viewport_to_world(cam_tf, cursor).ok() else {
        return;
    };

    let hits = ray_cast.cast_ray(ray, &default());
    for &(entity, _) in hits {
        if let Some(target) =
            resolve_targetable_ancestor(entity, &parent_query, &remote_q, &visibility_q)
        {
            current.0 = Some(target);
            return;
        }
    }
}

pub(crate) fn resolve_interaction_ancestor(
    entity: Entity,
    parent_query: &Query<&ChildOf>,
    npc_q: &Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
    object_q: &Query<&WorldObjectInteraction>,
    quest_q: &Query<(), With<QuestTrackedItem>>,
    visibility_q: &Query<&Visibility>,
) -> Option<InteractionTarget> {
    let mut current = entity;
    loop {
        if is_hidden_entity(current, visibility_q) {
            return None;
        }
        if let Ok(target) = npc_q.get(current) {
            return Some(InteractionTarget::Npc(target));
        }
        if let Ok(interaction) = object_q.get(current) {
            return Some(InteractionTarget::Object(current, interaction.kind));
        }
        if quest_q.get(current).is_ok() {
            return Some(InteractionTarget::Object(
                current,
                WorldObjectInteractionKind::QuestObject,
            ));
        }
        let Ok(parent) = parent_query.get(current) else {
            return None;
        };
        current = parent.parent();
    }
}

fn is_hidden_entity(entity: Entity, visibility_q: &Query<&Visibility>) -> bool {
    visibility_q
        .get(entity)
        .is_ok_and(|visibility| *visibility == Visibility::Hidden)
}

/// On Tab, cycle through nearby RemoteEntity sorted by distance from local player.
fn tab_target(
    keybinds: crate::ui_input_mode::WorldKeybinds,
    player_q: Query<&Transform, With<Player>>,
    remote_q: RemoteTargetQuery<'_, '_>,
    mut current: ResMut<CurrentTarget>,
) {
    if !keybinds.just_pressed(InputAction::TargetNearest) {
        return;
    }
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let sorted = sorted_targets_by_distance(player_tf, &remote_q);
    current.0 = pick_next_target(&sorted, current.0);
}

/// Sort remote entities by distance from player, return entity list.
fn sorted_targets_by_distance(
    player_tf: &Transform,
    remote_q: &RemoteTargetQuery<'_, '_>,
) -> Vec<Entity> {
    let mut entities: Vec<(Entity, f32)> = remote_q
        .iter()
        .filter(|(_, _, visibility)| visibility.is_none_or(|value| *value != Visibility::Hidden))
        .map(|(entity, transform, _)| {
            (
                entity,
                transform
                    .translation
                    .distance_squared(player_tf.translation),
            )
        })
        .collect();
    entities.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    entities.into_iter().map(|(e, _)| e).collect()
}

/// Pick the next target after the current one in the sorted list, wrapping around.
fn pick_next_target(sorted: &[Entity], current: Option<Entity>) -> Option<Entity> {
    if sorted.is_empty() {
        return None;
    }
    let Some(cur) = current else {
        return Some(sorted[0]);
    };
    let idx = sorted.iter().position(|&e| e == cur);
    match idx {
        Some(i) => Some(sorted[(i + 1) % sorted.len()]),
        None => Some(sorted[0]),
    }
}

pub(crate) fn resolve_targetable_ancestor(
    entity: Entity,
    parent_query: &Query<&ChildOf>,
    remote_q: &Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
    visibility_q: &Query<&Visibility>,
) -> Option<Entity> {
    let mut current = entity;
    loop {
        if let Ok(target) = remote_q.get(current) {
            let is_hidden = visibility_q
                .get(target)
                .is_ok_and(|visibility| *visibility == Visibility::Hidden);
            if !is_hidden {
                return Some(target);
            }
            return None;
        }
        let Ok(parent) = parent_query.get(current) else {
            return None;
        };
        current = parent.parent();
    }
}

/// On F1, set the current target to the local player entity.
fn self_target(
    keybinds: crate::ui_input_mode::WorldKeybinds,
    player_q: Query<Entity, With<Player>>,
    mut current: ResMut<CurrentTarget>,
) {
    if !keybinds.just_pressed(InputAction::TargetSelf) {
        return;
    }
    let Ok(player) = player_q.single() else {
        return;
    };
    current.0 = Some(player);
}

/// Maximum distance (world units) at which NPC interaction is allowed.
const INTERACT_RANGE: f32 = 5.0;

/// On right-click, interact with the targeted NPC if within range.
fn right_click_interact(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut ray_cast: MeshRayCast,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui_state: Option<Res<UiState>>,
    mut state: RightClickInteractionState<'_, '_>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some(cursor) = right_click_cursor(&windows, ui_state.as_deref()) else {
        return;
    };
    let Ok(player_tf) = state.player_q.single() else {
        return;
    };
    let player_position = player_tf.translation();

    if let Some(interaction) = interaction_target_at_cursor(
        cursor,
        &cameras,
        &mut ray_cast,
        &state.parent_query,
        &state.npc_entities,
        &state.object_q,
        &state.quest_q,
        &state.visibility_q,
    ) && handle_clicked_interaction(interaction, player_position, &mut state)
    {
        return;
    }

    let _ = interact_with_current_npc_target(player_position, &mut state);
}

fn cursor_over_ui(ui: &UiState, window: &Window) -> bool {
    ui_cursor_position(&ui.registry, window)
        .is_some_and(|cursor| find_frame_at(&ui.registry, cursor.x, cursor.y).is_some())
}

fn right_click_cursor(
    windows: &Query<&Window, With<PrimaryWindow>>,
    ui_state: Option<&UiState>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    if ui_state.is_some_and(|ui| cursor_over_ui(ui, window)) {
        return None;
    }
    Some(cursor)
}

fn handle_clicked_interaction(
    interaction: InteractionTarget,
    player_position: Vec3,
    state: &mut RightClickInteractionState<'_, '_>,
) -> bool {
    match interaction {
        InteractionTarget::Npc(target_entity) => {
            interact_with_clicked_npc(target_entity, player_position, state)
        }
        InteractionTarget::Object(target_entity, kind) => {
            interact_with_clicked_object(target_entity, kind, player_position, state)
        }
    }
}

fn interact_with_clicked_npc(
    target_entity: Entity,
    player_position: Vec3,
    state: &mut RightClickInteractionState<'_, '_>,
) -> bool {
    state.current.0 = Some(target_entity);
    let Ok(npc_tf) = state.npc_q.get(target_entity) else {
        return true;
    };
    if player_position.distance(npc_tf.translation()) > INTERACT_RANGE {
        return true;
    }
    interact_or_loot(target_entity, state);
    true
}

/// A lootable corpse opens its loot (`autoLootDefault` inverted by Shift, the
/// `AUTOLOOTTOGGLE` default); any other corpse is only targeted; a living NPC is
/// interacted with.
fn interact_or_loot(npc: Entity, state: &mut RightClickInteractionState<'_, '_>) {
    let (dead, lootable) = state
        .corpses
        .get(npc)
        .map_or((false, false), |(health, lootable)| {
            (health.current <= 0.0, lootable)
        });
    let default = state.hud.as_ref().is_some_and(|hud| hud.auto_loot);
    let shift = state.keys.pressed(KeyCode::ShiftLeft) || state.keys.pressed(KeyCode::ShiftRight);
    match npc_right_click(dead, lootable, auto_loot(default, shift)) {
        NpcRightClick::Loot { auto } => {
            state.loot.write(LootRequest::Open { corpse: npc, auto });
        }
        NpcRightClick::Target => {}
        NpcRightClick::Interact => {
            state
                .interactions
                .write(crate::networking_quests::NpcInteractionRequest::Interact(
                    npc,
                ));
        }
    }
}

fn interact_with_clicked_object(
    target_entity: Entity,
    kind: WorldObjectInteractionKind,
    player_position: Vec3,
    state: &mut RightClickInteractionState<'_, '_>,
) -> bool {
    if kind == WorldObjectInteractionKind::ServerObject {
        state
            .interactions
            .write(crate::networking_quests::NpcInteractionRequest::UseObject(
                target_entity,
            ));
        return true;
    }
    let Ok(object_tf) = state.object_tf_q.get(target_entity) else {
        return true;
    };
    if player_position.distance(object_tf.translation()) > INTERACT_RANGE {
        return true;
    }
    let _ = interact_with_object(
        kind,
        &mut state.mail_queue,
        state.window_manager.as_deref_mut(),
        state.emote_input.as_deref_mut(),
    );
    true
}

fn interact_with_current_npc_target(
    player_position: Vec3,
    state: &mut RightClickInteractionState<'_, '_>,
) -> bool {
    let Some(target_entity) = state.current.0 else {
        return false;
    };
    let Ok(npc_tf) = state.npc_q.get(target_entity) else {
        return false;
    };
    if player_position.distance(npc_tf.translation()) > INTERACT_RANGE {
        return false;
    }
    interact_or_loot(target_entity, state);
    true
}

fn interaction_target_at_cursor(
    cursor: Vec2,
    cameras: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    ray_cast: &mut MeshRayCast,
    parent_query: &Query<&ChildOf>,
    npc_q: &Query<Entity, (With<RemoteEntity>, With<Npc>, Without<Player>)>,
    object_q: &Query<&WorldObjectInteraction>,
    quest_q: &Query<(), With<QuestTrackedItem>>,
    visibility_q: &Query<&Visibility>,
) -> Option<InteractionTarget> {
    let Ok((camera, cam_tf)) = cameras.single() else {
        return None;
    };
    let ray = camera.viewport_to_world(cam_tf, cursor).ok()?;
    let hits = ray_cast.cast_ray(ray, &default());
    for &(entity, _) in hits {
        if let Some(target) = resolve_interaction_ancestor(
            entity,
            parent_query,
            npc_q,
            object_q,
            quest_q,
            visibility_q,
        ) {
            return Some(target);
        }
    }
    None
}

fn interact_with_object(
    kind: WorldObjectInteractionKind,
    mail_queue: &mut MailIntentQueue,
    window_manager: Option<&mut crate::window_manager::WindowManager>,
    emote_input: Option<&mut crate::networking::EmoteInput>,
) -> bool {
    match kind {
        WorldObjectInteractionKind::Mailbox => {
            mail_queue.open_mailbox();
            if let Some(window_manager) = window_manager {
                window_manager.open(crate::window_manager::WindowId::Mail);
            }
            true
        }
        // Crafting spell foci (`SpellFocusObject`), not interactable in Retail.
        WorldObjectInteractionKind::Forge | WorldObjectInteractionKind::Anvil => false,
        WorldObjectInteractionKind::Chair => {
            if let Some(input) = emote_input {
                input.0 = Some(EmoteIntent {
                    emote: EmoteKind::Sit,
                });
                return true;
            }
            false
        }
        // Gathering nodes are server game objects (follow-up), not doodads.
        WorldObjectInteractionKind::GatherNode(_) => false,
        WorldObjectInteractionKind::ZoneTransition => true,
        WorldObjectInteractionKind::QuestObject | WorldObjectInteractionKind::ServerObject => false,
    }
}

/// When CurrentTarget changes, spawn or move the selection circle.
#[cfg(test)]
#[path = "../../../tests/unit/target_tests/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/target_portal_tests.rs"]
mod portal_tests;

#[cfg(test)]
#[path = "../../../tests/unit/target_visuals_write_tests.rs"]
mod target_visuals_write_tests;
