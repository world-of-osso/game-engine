use std::collections::HashMap;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::inspect::InspectRuntimeState;
use game_engine::status::{GroupMemberEntry, GroupRole, GroupStatusSnapshot};
use game_engine::targeting::CurrentTarget;
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::group_frames_component::{
    ACTION_GROUP_MENU_CLOSE, ACTION_GROUP_MENU_INSPECT, ACTION_GROUP_MENU_TARGET, GROUP_MENU_W,
    GroupContextMenuState, GroupFramesState, group_frames_screen, group_menu_height,
};
use game_engine::ui::screens::party_frame_component::{
    PartyFrameState, PartyMemberState, PartyRole, ReadyCheckState,
};
use game_engine::ui::screens::raid_frame_component::{RaidFrameState, RaidGroup, RaidMember};
use shared::components::{Health as NetHealth, Player as NetPlayer};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;
use crate::networking::LocalPlayer;
use crate::ui_input::walk_up_for_onclick;

const GROUP_FRAME_RANGE: f32 = 45.0;

struct GroupFramesRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for GroupFramesRes {}
unsafe impl Sync for GroupFramesRes {}

#[derive(Resource)]
struct GroupFramesWrap(GroupFramesRes);

#[derive(Resource, Clone, PartialEq)]
struct GroupFramesModel(GroupFramesState);

#[derive(Resource, Default, Clone, PartialEq)]
struct GroupFrameMenu {
    visible: bool,
    target: Option<Entity>,
    title: String,
    x: f32,
    y: f32,
}

#[derive(Resource, Default)]
struct GroupFrameClickMap {
    party_targets: Vec<Option<Entity>>,
    raid_targets: Vec<Vec<Option<Entity>>>,
}

#[derive(Clone, Copy)]
struct ResolvedGroupUnit {
    entity: Entity,
    health_current: u32,
    health_max: u32,
    in_range: bool,
}

pub struct GroupFramesPlugin;

impl Plugin for GroupFramesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GroupFrameMenu>();
        app.init_resource::<GroupFrameClickMap>();
        app.add_systems(OnEnter(GameState::InWorld), build_group_frames_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_group_frames_ui);
        app.add_systems(
            Update,
            (sync_group_frames_state, handle_group_frame_pointer)
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_group_frames_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    roster: Option<Res<GroupStatusSnapshot>>,
    menu: Res<GroupFrameMenu>,
    players: Query<(
        Entity,
        &NetPlayer,
        Option<&NetHealth>,
        Option<&Transform>,
        Has<LocalPlayer>,
    )>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let (state, click_map) = build_group_frames_state(roster.as_deref(), &menu, &players);
    let mut shared = SharedContext::new();
    insert_screen_state(&mut shared, &state);
    let mut screen = Screen::new(group_frames_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(GroupFramesWrap(GroupFramesRes { screen, shared }));
    commands.insert_resource(GroupFramesModel(state));
    commands.insert_resource(click_map);
}

fn teardown_group_frames_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<GroupFramesWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<GroupFramesWrap>();
    commands.remove_resource::<GroupFramesModel>();
}

fn sync_group_frames_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<GroupFramesWrap>>,
    mut last_model: Option<ResMut<GroupFramesModel>>,
    mut click_map: ResMut<GroupFrameClickMap>,
    roster: Option<Res<GroupStatusSnapshot>>,
    menu: Res<GroupFrameMenu>,
    players: Query<(
        Entity,
        &NetPlayer,
        Option<&NetHealth>,
        Option<&Transform>,
        Has<LocalPlayer>,
    )>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let (state, next_click_map) = build_group_frames_state(roster.as_deref(), &menu, &players);
    *click_map = next_click_map;
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    insert_screen_state(&mut res.shared, &state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn insert_screen_state(shared: &mut SharedContext, state: &GroupFramesState) {
    shared.insert(state.clone());
    shared.insert(state.party.clone());
    shared.insert(state.raid.clone());
    shared.insert(state.menu.clone());
}

fn build_group_frames_state(
    roster: Option<&GroupStatusSnapshot>,
    menu: &GroupFrameMenu,
    players: &Query<(
        Entity,
        &NetPlayer,
        Option<&NetHealth>,
        Option<&Transform>,
        Has<LocalPlayer>,
    )>,
) -> (GroupFramesState, GroupFrameClickMap) {
    let roster = roster.cloned().unwrap_or_default();
    let (resolved_units, local_name) = resolve_group_units(players);
    let party = build_party_frame_state(&roster, local_name.as_deref(), &resolved_units);
    let raid = build_raid_frame_state(&roster, &resolved_units);
    let click_map = build_click_map(&roster, local_name.as_deref(), &resolved_units);
    let menu = build_menu_state(menu);
    (GroupFramesState { party, raid, menu }, click_map)
}

fn resolve_group_units(
    players: &Query<(
        Entity,
        &NetPlayer,
        Option<&NetHealth>,
        Option<&Transform>,
        Has<LocalPlayer>,
    )>,
) -> (HashMap<String, ResolvedGroupUnit>, Option<String>) {
    let local_position = players.iter().find_map(|(_, _, _, transform, is_local)| {
        if is_local {
            transform.map(|transform| transform.translation)
        } else {
            None
        }
    });
    let mut resolved = HashMap::new();
    let mut local_name = None;
    for (entity, player, health, transform, is_local) in players.iter() {
        if is_local {
            local_name = Some(player.name.clone());
        }
        let (health_current, health_max) = health
            .map(|health| {
                (
                    health.current.max(0.0).round() as u32,
                    health.max.max(0.0).round() as u32,
                )
            })
            .unwrap_or((100, 100));
        let in_range = match (local_position, transform) {
            (Some(local), Some(transform)) => {
                transform.translation.distance_squared(local) <= GROUP_FRAME_RANGE.powi(2)
            }
            _ => is_local,
        };
        resolved.insert(
            player.name.clone(),
            ResolvedGroupUnit {
                entity,
                health_current,
                health_max: health_max.max(1),
                in_range,
            },
        );
    }
    (resolved, local_name)
}

fn build_party_frame_state(
    roster: &GroupStatusSnapshot,
    local_name: Option<&str>,
    resolved_units: &HashMap<String, ResolvedGroupUnit>,
) -> PartyFrameState {
    if roster.is_raid {
        return PartyFrameState::default();
    }
    let members = roster
        .members
        .iter()
        .filter(|member| Some(member.name.as_str()) != local_name)
        .take(4)
        .map(|member| map_party_member(member, resolved_units.get(&member.name)))
        .collect::<Vec<_>>();
    PartyFrameState {
        visible: !members.is_empty(),
        members,
    }
}

fn build_raid_frame_state(
    roster: &GroupStatusSnapshot,
    resolved_units: &HashMap<String, ResolvedGroupUnit>,
) -> RaidFrameState {
    if !roster.is_raid {
        return RaidFrameState::default();
    }
    let mut groups = vec![RaidGroup::default(); 5];
    for member in &roster.members {
        let group_index = usize::from(member.subgroup.saturating_sub(1)).min(4);
        groups[group_index]
            .members
            .push(map_raid_member(member, resolved_units.get(&member.name)));
    }
    RaidFrameState {
        visible: !roster.members.is_empty(),
        groups,
    }
}

fn build_click_map(
    roster: &GroupStatusSnapshot,
    local_name: Option<&str>,
    resolved_units: &HashMap<String, ResolvedGroupUnit>,
) -> GroupFrameClickMap {
    if roster.is_raid {
        let mut raid_targets = vec![Vec::new(); 5];
        for member in &roster.members {
            let group_index = usize::from(member.subgroup.saturating_sub(1)).min(4);
            raid_targets[group_index]
                .push(resolved_units.get(&member.name).map(|unit| unit.entity));
        }
        GroupFrameClickMap {
            party_targets: Vec::new(),
            raid_targets,
        }
    } else {
        GroupFrameClickMap {
            party_targets: roster
                .members
                .iter()
                .filter(|member| Some(member.name.as_str()) != local_name)
                .take(4)
                .map(|member| resolved_units.get(&member.name).map(|unit| unit.entity))
                .collect(),
            raid_targets: vec![Vec::new(); 5],
        }
    }
}

fn build_menu_state(menu: &GroupFrameMenu) -> GroupContextMenuState {
    GroupContextMenuState {
        visible: menu.visible,
        title: menu.title.clone(),
        x: menu.x,
        y: menu.y,
    }
}

fn map_party_member(
    member: &GroupMemberEntry,
    resolved: Option<&ResolvedGroupUnit>,
) -> PartyMemberState {
    let health_current =
        resolved.map_or(default_current(member.online), |unit| unit.health_current);
    let health_max = resolved.map_or(100, |unit| unit.health_max);
    PartyMemberState {
        name: member.name.clone(),
        health_current,
        health_max,
        role: map_party_role(member.role.clone()),
        debuffs: Vec::new(),
        online: member.online,
        in_range: resolved.is_some_and(|unit| unit.in_range),
        ready_check: ReadyCheckState::None,
        incoming_heals: 0.0,
    }
}

fn map_raid_member(member: &GroupMemberEntry, resolved: Option<&ResolvedGroupUnit>) -> RaidMember {
    let health_current =
        resolved.map_or(default_current(member.online), |unit| unit.health_current);
    let health_max = resolved.map_or(100, |unit| unit.health_max);
    RaidMember {
        name: member.name.clone(),
        health_current,
        health_max,
        alive: health_current > 0,
        in_range: resolved.is_some_and(|unit| unit.in_range),
        ready_check: game_engine::ui::screens::raid_frame_component::RaidReadyCheck::None,
        incoming_heals: 0.0,
    }
}

fn default_current(online: bool) -> u32 {
    if online { 100 } else { 0 }
}

fn map_party_role(role: GroupRole) -> PartyRole {
    match role {
        GroupRole::Tank => PartyRole::Tank,
        GroupRole::Healer => PartyRole::Healer,
        GroupRole::Damage | GroupRole::None => PartyRole::Dps,
    }
}

fn handle_group_frame_pointer(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    ui: Res<UiState>,
    click_map: Res<GroupFrameClickMap>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    mut menu: ResMut<GroupFrameMenu>,
    mut current_target: ResMut<CurrentTarget>,
    mut inspect_runtime: Option<ResMut<InspectRuntimeState>>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Left) && !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if mouse.just_pressed(MouseButton::Left) {
        handle_group_frame_click(
            &ui.registry,
            &click_map,
            cursor,
            MouseButton::Left,
            &mut menu,
            &mut current_target,
            &mut inspect_runtime,
        );
    }
    if mouse.just_pressed(MouseButton::Right) {
        handle_group_frame_click(
            &ui.registry,
            &click_map,
            cursor,
            MouseButton::Right,
            &mut menu,
            &mut current_target,
            &mut inspect_runtime,
        );
    }
}

fn handle_group_frame_click(
    registry: &game_engine::ui::registry::FrameRegistry,
    click_map: &GroupFrameClickMap,
    cursor: Vec2,
    button: MouseButton,
    menu: &mut GroupFrameMenu,
    current_target: &mut CurrentTarget,
    inspect_runtime: &mut Option<ResMut<InspectRuntimeState>>,
) {
    let Some(frame_id) = find_frame_at(registry, cursor.x, cursor.y) else {
        menu.visible = false;
        return;
    };

    if button == MouseButton::Left
        && let Some(action) = walk_up_for_onclick(registry, frame_id)
    {
        dispatch_menu_action(&action, menu, current_target, inspect_runtime);
        return;
    }

    let Some(entity) = resolve_group_frame_target(registry, frame_id, click_map) else {
        if button == MouseButton::Left {
            menu.visible = false;
        }
        return;
    };

    current_target.0 = Some(entity);
    if button == MouseButton::Left {
        menu.visible = false;
        return;
    }

    menu.visible = true;
    menu.target = Some(entity);
    menu.title = resolve_menu_title(registry, frame_id);
    let (x, y) = clamp_menu_position(cursor, registry);
    menu.x = x;
    menu.y = y;
}

fn dispatch_menu_action(
    action: &str,
    menu: &mut GroupFrameMenu,
    current_target: &mut CurrentTarget,
    inspect_runtime: &mut Option<ResMut<InspectRuntimeState>>,
) {
    match action {
        ACTION_GROUP_MENU_TARGET => {
            current_target.0 = menu.target;
            menu.visible = false;
        }
        ACTION_GROUP_MENU_INSPECT => {
            current_target.0 = menu.target;
            if let (Some(target), Some(runtime)) = (menu.target, inspect_runtime.as_deref_mut()) {
                game_engine::inspect::request_query_for_target(runtime, Some(target));
            }
            menu.visible = false;
        }
        ACTION_GROUP_MENU_CLOSE => {
            menu.visible = false;
        }
        _ => {}
    }
}

fn resolve_group_frame_target(
    registry: &game_engine::ui::registry::FrameRegistry,
    mut frame_id: u64,
    click_map: &GroupFrameClickMap,
) -> Option<Entity> {
    loop {
        let frame = registry.get(frame_id)?;
        if let Some(name) = frame.name.as_deref() {
            if let Some(index) = parse_party_frame_name(name) {
                return click_map
                    .party_targets
                    .get(index)
                    .and_then(|entity| *entity);
            }
            if let Some((group, member)) = parse_raid_cell_name(name) {
                return click_map
                    .raid_targets
                    .get(group)
                    .and_then(|members| members.get(member))
                    .and_then(|entity| *entity);
            }
        }
        frame_id = frame.parent_id?;
    }
}

fn parse_party_frame_name(name: &str) -> Option<usize> {
    let index = name.strip_prefix("PartyMember")?;
    if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    index.parse().ok()
}

fn parse_raid_cell_name(name: &str) -> Option<(usize, usize)> {
    let coords = name.strip_prefix("RaidCell")?;
    let (group, member) = coords.split_once('_')?;
    if group.is_empty()
        || member.is_empty()
        || !group.bytes().all(|byte| byte.is_ascii_digit())
        || !member.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some((group.parse().ok()?, member.parse().ok()?))
}

fn resolve_menu_title(
    registry: &game_engine::ui::registry::FrameRegistry,
    frame_id: u64,
) -> String {
    let mut current = Some(frame_id);
    while let Some(id) = current {
        let Some(frame) = registry.get(id) else { break };
        if let Some(text) = resolve_named_unit_title(registry, frame.name.as_deref()) {
            return text;
        }
        current = frame.parent_id;
    }
    "Unit".into()
}

fn resolve_named_unit_title(
    registry: &game_engine::ui::registry::FrameRegistry,
    name: Option<&str>,
) -> Option<String> {
    let label = unit_label_frame_name(name?)?;
    frame_text(registry, &label)
}

fn unit_label_frame_name(name: &str) -> Option<String> {
    if let Some(index) = parse_party_frame_name(name) {
        return Some(format!("PartyMember{index}Name"));
    }
    if let Some((group, member)) = parse_raid_cell_name(name) {
        return Some(format!("RaidCell{group}_{member}Name"));
    }
    None
}

fn frame_text(registry: &game_engine::ui::registry::FrameRegistry, name: &str) -> Option<String> {
    let id = registry.get_by_name(name)?;
    let frame = registry.get(id)?;
    let game_engine::ui::frame::WidgetData::FontString(text) = frame.widget_data.as_ref()? else {
        return None;
    };
    Some(text.text.clone())
}

fn clamp_menu_position(
    cursor: Vec2,
    registry: &game_engine::ui::registry::FrameRegistry,
) -> (f32, f32) {
    let max_x = (registry.screen_width - GROUP_MENU_W).max(0.0);
    let max_y = (registry.screen_height - group_menu_height()).max(0.0);
    (cursor.x.clamp(0.0, max_x), cursor.y.clamp(0.0, max_y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::ui::registry::FrameRegistry;
    use ui_toolkit::layout::recompute_layouts;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn player(name: &str, subgroup: u8) -> GroupMemberEntry {
        GroupMemberEntry {
            name: name.into(),
            role: GroupRole::Damage,
            is_leader: false,
            online: true,
            subgroup,
        }
    }

    #[test]
    fn party_state_excludes_local_player() {
        let roster = GroupStatusSnapshot {
            is_raid: false,
            members: vec![player("Theron", 1), player("Valeera", 1)],
            ..Default::default()
        };
        let mut resolved = HashMap::new();
        resolved.insert(
            "Theron".into(),
            ResolvedGroupUnit {
                entity: Entity::from_bits(1),
                health_current: 100,
                health_max: 100,
                in_range: true,
            },
        );
        resolved.insert(
            "Valeera".into(),
            ResolvedGroupUnit {
                entity: Entity::from_bits(2),
                health_current: 75,
                health_max: 100,
                in_range: true,
            },
        );

        let state = build_party_frame_state(&roster, Some("Theron"), &resolved);
        let click_map = build_click_map(&roster, Some("Theron"), &resolved);

        assert_eq!(state.members.len(), 1);
        assert_eq!(state.members[0].name, "Valeera");
        assert_eq!(click_map.party_targets, vec![Some(Entity::from_bits(2))]);
    }

    #[test]
    fn raid_state_groups_members_by_subgroup() {
        let roster = GroupStatusSnapshot {
            is_raid: true,
            members: vec![player("Tank", 1), player("Healer", 1), player("Mage", 2)],
            ..Default::default()
        };
        let resolved = HashMap::new();

        let state = build_raid_frame_state(&roster, &resolved);

        assert_eq!(state.groups[0].members.len(), 2);
        assert_eq!(state.groups[1].members.len(), 1);
        assert_eq!(state.groups[0].members[0].name, "Tank");
        assert_eq!(state.groups[1].members[0].name, "Mage");
    }

    #[test]
    fn left_click_party_frame_targets_entity() {
        let roster = GroupStatusSnapshot {
            is_raid: false,
            members: vec![player("Theron", 1), player("Valeera", 1)],
            ..Default::default()
        };
        let mut resolved = HashMap::new();
        let valeera = Entity::from_bits(2);
        resolved.insert(
            "Theron".into(),
            ResolvedGroupUnit {
                entity: Entity::from_bits(1),
                health_current: 100,
                health_max: 100,
                in_range: true,
            },
        );
        resolved.insert(
            "Valeera".into(),
            ResolvedGroupUnit {
                entity: valeera,
                health_current: 90,
                health_max: 100,
                in_range: true,
            },
        );
        let state = GroupFramesState {
            party: build_party_frame_state(&roster, Some("Theron"), &resolved),
            raid: RaidFrameState::default(),
            menu: GroupContextMenuState::default(),
        };
        let click_map = build_click_map(&roster, Some("Theron"), &resolved);
        let registry = build_registry(&state);
        let mut menu = GroupFrameMenu::default();
        let mut target = CurrentTarget::default();
        let mut inspect_runtime = None;

        handle_group_frame_click(
            &registry,
            &click_map,
            Vec2::new(20.0, 225.0),
            MouseButton::Left,
            &mut menu,
            &mut target,
            &mut inspect_runtime,
        );

        assert_eq!(target.0, Some(valeera));
    }

    #[test]
    fn right_click_party_frame_opens_context_menu() {
        let roster = GroupStatusSnapshot {
            is_raid: false,
            members: vec![player("Theron", 1), player("Valeera", 1)],
            ..Default::default()
        };
        let mut resolved = HashMap::new();
        resolved.insert(
            "Theron".into(),
            ResolvedGroupUnit {
                entity: Entity::from_bits(1),
                health_current: 100,
                health_max: 100,
                in_range: true,
            },
        );
        resolved.insert(
            "Valeera".into(),
            ResolvedGroupUnit {
                entity: Entity::from_bits(2),
                health_current: 90,
                health_max: 100,
                in_range: true,
            },
        );
        let state = GroupFramesState {
            party: build_party_frame_state(&roster, Some("Theron"), &resolved),
            raid: RaidFrameState::default(),
            menu: GroupContextMenuState::default(),
        };
        let click_map = build_click_map(&roster, Some("Theron"), &resolved);
        let registry = build_registry(&state);
        let mut menu = GroupFrameMenu::default();
        let mut target = CurrentTarget::default();
        let mut inspect_runtime = None;

        handle_group_frame_click(
            &registry,
            &click_map,
            Vec2::new(20.0, 225.0),
            MouseButton::Right,
            &mut menu,
            &mut target,
            &mut inspect_runtime,
        );

        assert!(menu.visible);
        assert_eq!(menu.title, "Valeera");
    }

    fn build_registry(state: &GroupFramesState) -> FrameRegistry {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(state.clone());
        shared.insert(state.party.clone());
        shared.insert(state.raid.clone());
        shared.insert(state.menu.clone());
        Screen::new(group_frames_screen).sync(&shared, &mut registry);
        recompute_layouts(&mut registry);
        registry
    }
}
