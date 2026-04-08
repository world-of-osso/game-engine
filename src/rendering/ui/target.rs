use bevy::ecs::system::SystemParam;
use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::gossip_data::GossipIntentQueue;
use game_engine::mail_data::MailIntentQueue;
use game_engine::quest_tracking::QuestTrackedItem;
use game_engine::targeting::CurrentTarget;
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::UiState;
use shared::components::Npc;
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

#[path = "target_visuals.rs"]
mod target_visuals;

use target_visuals::{spawn_target_circle, update_target_circle};

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

impl GatherNodeKind {
    pub const fn node_id(self) -> u32 {
        match self {
            Self::CopperVein => 1,
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::CopperVein => "Copper Vein",
        }
    }

    pub const fn cast_duration_secs(self) -> f32 {
        match self {
            Self::CopperVein => 1.5,
        }
    }
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
    gossip_queue: ResMut<'w, GossipIntentQueue>,
    mail_queue: ResMut<'w, MailIntentQueue>,
    mail_frame_open: Option<ResMut<'w, crate::scenes::mail_frame::MailFrameOpen>>,
    professions_open: Option<ResMut<'w, crate::scenes::professions_frame::ProfessionsFrameOpen>>,
    emote_input: Option<ResMut<'w, crate::networking::EmoteInput>>,
    profession_runtime: Option<ResMut<'w, game_engine::profession::ProfessionRuntimeState>>,
    casting_state: Option<ResMut<'w, game_engine::casting_data::CastingState>>,
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
        app.init_resource::<GossipIntentQueue>();
        app.init_resource::<MailIntentQueue>();
        app.add_systems(Update, click_to_target.run_if(targeting_state_active));
        app.add_systems(Update, tab_target.run_if(targeting_state_active));
        app.add_systems(Update, self_target.run_if(targeting_state_active));
        app.add_systems(Update, clear_target.run_if(targeting_state_active));
        app.add_systems(Update, right_click_interact.run_if(targeting_state_active));
        app.add_systems(Update, spawn_target_circle.run_if(targeting_state_active));
        app.add_systems(Update, update_target_circle.run_if(targeting_state_active));
    }
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
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut ray_cast: MeshRayCast,
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
        .is_some_and(|ui| find_frame_at(&ui.registry, cursor.x, cursor.y).is_some())
    {
        return;
    }
    let Ok((camera, cam_tf)) = cameras.single() else {
        return;
    };
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
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    player_q: Query<&Transform, With<Player>>,
    remote_q: RemoteTargetQuery<'_, '_>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    bindings: Res<InputBindings>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if !bindings.is_just_pressed(InputAction::TargetNearest, &keys, &mouse_buttons) {
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
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    player_q: Query<Entity, With<Player>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    bindings: Res<InputBindings>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if !bindings.is_just_pressed(InputAction::TargetSelf, &keys, &mouse_buttons) {
        return;
    }
    let Ok(player) = player_q.single() else {
        return;
    };
    current.0 = Some(player);
}

/// On Escape, clear the current target.
fn clear_target(
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    mut current: ResMut<CurrentTarget>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        current.0 = None;
    }
}

/// Maximum distance (world units) at which NPC interaction is allowed.
const INTERACT_RANGE: f32 = 5.0;

/// On right-click, interact with the targeted NPC if within range.
fn right_click_interact(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
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

fn right_click_cursor(
    windows: &Query<&Window, With<PrimaryWindow>>,
    ui_state: Option<&UiState>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    if ui_state.is_some_and(|ui| find_frame_at(&ui.registry, cursor.x, cursor.y).is_some()) {
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
    state.gossip_queue.interact(target_entity.to_bits());
    true
}

fn interact_with_clicked_object(
    target_entity: Entity,
    kind: WorldObjectInteractionKind,
    player_position: Vec3,
    state: &mut RightClickInteractionState<'_, '_>,
) -> bool {
    let Ok(object_tf) = state.object_tf_q.get(target_entity) else {
        return true;
    };
    if player_position.distance(object_tf.translation()) > INTERACT_RANGE {
        return true;
    }
    let _ = interact_with_object(
        kind,
        &mut state.mail_queue,
        state.mail_frame_open.as_deref_mut(),
        state.professions_open.as_deref_mut(),
        state.emote_input.as_deref_mut(),
        state.profession_runtime.as_deref_mut(),
        state.casting_state.as_deref_mut(),
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
    state.gossip_queue.interact(target_entity.to_bits());
    true
}

fn interaction_target_at_cursor(
    cursor: Vec2,
    cameras: &Query<(&Camera, &GlobalTransform)>,
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
    mail_frame_open: Option<&mut crate::scenes::mail_frame::MailFrameOpen>,
    professions_open: Option<&mut crate::scenes::professions_frame::ProfessionsFrameOpen>,
    emote_input: Option<&mut crate::networking::EmoteInput>,
    profession_runtime: Option<&mut game_engine::profession::ProfessionRuntimeState>,
    casting_state: Option<&mut game_engine::casting_data::CastingState>,
) -> bool {
    match kind {
        WorldObjectInteractionKind::Mailbox => {
            mail_queue.open_mailbox();
            if let Some(open) = mail_frame_open {
                open.0 = true;
            }
            true
        }
        WorldObjectInteractionKind::Forge | WorldObjectInteractionKind::Anvil => {
            if let Some(open) = professions_open {
                open.0 = true;
                return true;
            }
            false
        }
        WorldObjectInteractionKind::Chair => {
            if let Some(input) = emote_input {
                input.0 = Some(EmoteIntent {
                    emote: EmoteKind::Sit,
                });
                return true;
            }
            false
        }
        WorldObjectInteractionKind::GatherNode(node) => {
            start_gather_cast(node, profession_runtime, casting_state)
        }
        WorldObjectInteractionKind::ZoneTransition => true,
        WorldObjectInteractionKind::QuestObject => false,
    }
}

fn start_gather_cast(
    node: GatherNodeKind,
    profession_runtime: Option<&mut game_engine::profession::ProfessionRuntimeState>,
    casting_state: Option<&mut game_engine::casting_data::CastingState>,
) -> bool {
    let (Some(profession_runtime), Some(casting_state)) = (profession_runtime, casting_state)
    else {
        return false;
    };
    if casting_state.active.is_some() {
        return false;
    }
    game_engine::profession::queue_gather_action(profession_runtime, node.node_id());
    casting_state.start(game_engine::casting_data::ActiveCast {
        spell_name: format!("Mining {}", node.display_name()),
        spell_id: 0,
        icon_fdid: 0,
        cast_type: game_engine::casting_data::CastType::Cast,
        interruptible: true,
        duration: node.cast_duration_secs(),
        elapsed: 0.0,
    });
    true
}

/// When CurrentTarget changes, spawn or move the selection circle.
#[cfg(test)]
#[path = "../../../tests/unit/target_tests.rs"]
mod tests;
