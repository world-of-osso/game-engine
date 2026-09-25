//! Party/raid frames, their right-click menu, the `PARTY_INVITE` popup and the ready check
//! frame, all driven by [`GroupState`]. Clicks target members, menu entries and the ready
//! check buttons write [`GroupCommand`]s.

#[cfg(test)]
#[path = "../../ui/screens/menu_character_layout_test_support.rs"]
mod native_layout_support;

use std::collections::HashMap;
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::buff_data::DebuffType;
use game_engine::group_state::{GroupCommand, GroupMenuEntry, GroupState, group_menu_entries};
use game_engine::inspect::InspectRuntimeState;
use game_engine::nameplate_data::ClassColor;
use game_engine::spell_catalog::SpellCatalog;
use game_engine::targeting::CurrentTarget;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::compact_unit_frame_component::{
    CompactDebuffView, CompactUnitView, UnitStatus,
};
use game_engine::ui::screens::group_frames_component::{
    ACTION_GROUP_MENU_CLOSE, ACTION_GROUP_MENU_INSPECT, ACTION_GROUP_MENU_TARGET, GROUP_MENU_W,
    GroupContextMenuState, GroupFramesState, GroupMenuItem, RAID_GROUPS, group_frames_screen,
    group_menu_height, party_member_frame_name, raid_member_frame_name,
};
use game_engine::ui::screens::ready_check_frame_component::{
    ACTION_READY_CHECK_NO, ACTION_READY_CHECK_YES, ReadyCheckFrameState,
};
use shared::components::{Player as NetPlayer, PowerType};
use shared::death::DeathState;
use shared::protocol::{GROUP_INVITE_TIMEOUT_SECS, GroupMemberSnapshot};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;
use crate::ui_input::walk_up_for_onclick;

/// `UnitInRange` distance for party/raid frame fading.
const GROUP_FRAME_RANGE: f32 = 40.0;
/// Longest member menu: Target, Inspect, Promote, Uninvite, four roles and Close.
const MENU_MAX_ITEMS: usize = 9;
pub const PARTY_INVITE_POPUP: &str = "PARTY_INVITE";

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

/// The open member menu: whose it is and where.
#[derive(Resource, Default, Clone, PartialEq)]
struct GroupFrameMenu {
    unit: Option<String>,
    x: f32,
    y: f32,
}

/// Member names in frame order, to resolve clicks on `CompactPartyFrameMember{n}` and
/// `CompactRaidGroup{g}Member{m}`.
#[derive(Resource, Default, Clone, Debug, PartialEq)]
struct GroupFrameClickMap {
    party: Vec<String>,
    raid: Vec<Vec<String>>,
}

pub struct GroupFramesPlugin;

impl Plugin for GroupFramesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GroupFrameMenu>()
            .init_resource::<GroupFrameClickMap>()
            .init_resource::<GroupState>()
            .init_resource::<PopupStack>()
            .add_message::<GroupCommand>()
            .add_message::<PopupResult>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_group_frames_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_group_frames_ui);
        app.add_systems(
            Update,
            (
                sync_invite_popup,
                answer_invite_popup,
                handle_group_frame_pointer,
                sync_group_frames_state,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

#[derive(SystemParam)]
struct FrameSources<'w, 's> {
    group: Res<'w, GroupState>,
    menu: Res<'w, GroupFrameMenu>,
    target: Res<'w, CurrentTarget>,
    catalog: Option<Res<'w, SpellCatalog>>,
    players: Query<'w, 's, (Entity, &'static NetPlayer, Has<LocalPlayer>)>,
}

impl FrameSources<'_, '_> {
    fn local_name(&self) -> Option<String> {
        self.players
            .iter()
            .find(|(_, _, local)| *local)
            .map(|(_, player, _)| player.name.clone())
    }

    fn entities(&self) -> HashMap<String, Entity> {
        self.players
            .iter()
            .map(|(entity, player, _)| (player.name.clone(), entity))
            .collect()
    }

    fn build(&self) -> (GroupFramesState, GroupFrameClickMap) {
        let local = self.local_name();
        let entities = self.entities();
        let target = self.target.0.and_then(|target| {
            entities
                .iter()
                .find(|(_, entity)| **entity == target)
                .map(|(name, _)| name.clone())
        });
        let icon = |spell_id: u32| {
            self.catalog
                .as_ref()
                .and_then(|catalog| catalog.get(spell_id))
                .map_or(0, |spell| spell.icon_fdid)
        };
        let context = ViewContext {
            group: &self.group,
            local_name: local.as_deref(),
            target_name: target.as_deref(),
            icon_fdid: &icon,
        };
        let (mut state, click_map) = build_frames(&context);
        state.menu = build_menu(&self.menu, &self.group, local.as_deref(), &entities);
        state.ready_check = build_ready_check(&self.group, local.as_deref());
        (state, click_map)
    }
}

fn build_group_frames_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    sources: FrameSources,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let (state, click_map) = sources.build();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
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
    mut menu: ResMut<GroupFrameMenu>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    *menu = GroupFrameMenu::default();
    commands.remove_resource::<GroupFramesWrap>();
    commands.remove_resource::<GroupFramesModel>();
}

fn sync_group_frames_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<GroupFramesWrap>>,
    mut last_model: Option<ResMut<GroupFramesModel>>,
    mut click_map: ResMut<GroupFrameClickMap>,
    sources: FrameSources,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let (state, next_click_map) = sources.build();
    *click_map = next_click_map;
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

// --- Frame views ---

struct ViewContext<'a> {
    group: &'a GroupState,
    local_name: Option<&'a str>,
    target_name: Option<&'a str>,
    icon_fdid: &'a dyn Fn(u32) -> u32,
}

/// Party mode lists the player first, then the others in roster order (`CRFSort_Group`);
/// raid mode fills each subgroup column in roster order.
fn build_frames(context: &ViewContext) -> (GroupFramesState, GroupFrameClickMap) {
    let group = context.group;
    let mut state = GroupFramesState {
        raid: vec![Vec::new(); RAID_GROUPS],
        ..Default::default()
    };
    let mut click_map = GroupFrameClickMap {
        party: Vec::new(),
        raid: vec![Vec::new(); RAID_GROUPS],
    };
    if group.is_raid {
        for member in &group.members {
            let index = usize::from(member.subgroup.clamp(1, RAID_GROUPS as u8) - 1);
            state.raid[index].push(member_view(context, member));
            click_map.raid[index].push(member.name.clone());
        }
        return (state, click_map);
    }
    let mut ordered: Vec<&GroupMemberSnapshot> = group.members.iter().collect();
    ordered.sort_by_key(|member| Some(member.name.as_str()) != context.local_name);
    for member in ordered {
        state.party.push(member_view(context, member));
        click_map.party.push(member.name.clone());
    }
    (state, click_map)
}

fn member_view(context: &ViewContext, member: &GroupMemberSnapshot) -> CompactUnitView {
    let group = context.group;
    let live = group.live.get(&member.name).filter(|_| member.online);
    let status = match (member.online, live.map(|l| l.death)) {
        (false, _) => UnitStatus::Offline,
        (true, Some(DeathState::Dead | DeathState::Ghost)) => UnitStatus::Dead,
        _ => UnitStatus::Online,
    };
    CompactUnitView {
        name: member.name.clone(),
        class_rgb: ClassColor::from_class_id(member.class).map_or([1.0; 3], ClassColor::rgb),
        health_fraction: live.map(|l| fraction(l.health as i64, l.max_health as i64)),
        power: live.and_then(|l| l.power.as_ref()).map(|power| {
            (
                fraction(i64::from(power.current), i64::from(power.max)),
                power_bar_rgb(power.power),
            )
        }),
        role: member.role,
        status,
        in_range: in_range(context, member),
        selected: context.target_name == Some(member.name.as_str()),
        ready: group.ready_mark(&member.name),
        debuffs: live
            .map(|l| {
                l.debuffs
                    .iter()
                    .map(|aura| CompactDebuffView {
                        icon_fdid: (context.icon_fdid)(aura.spell_id),
                        dispel: DebuffType::from_dispel_id(aura.dispel_type),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Offline members and members without a known position are out of range; the player is
/// always in range.
fn in_range(context: &ViewContext, member: &GroupMemberSnapshot) -> bool {
    if context.local_name == Some(member.name.as_str()) {
        return true;
    }
    let group = context.group;
    let (Some(local), Some(other)) = (
        context.local_name.and_then(|name| group.live.get(name)),
        group.live.get(&member.name).filter(|_| member.online),
    ) else {
        return false;
    };
    let (a, b) = (local.position, other.position);
    let d2 = (a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2);
    d2 <= GROUP_FRAME_RANGE.powi(2)
}

fn fraction(current: i64, max: i64) -> f32 {
    if max <= 0 {
        return 0.0;
    }
    (current as f32 / max as f32).clamp(0.0, 1.0)
}

/// `PowerBarColor` (PowerBarColorUtil.lua:18-33); others take mana's colour as Retail
/// does for tokens without an entry (CompactUnitFrame.lua:780-786).
fn power_bar_rgb(power: PowerType) -> [f32; 3] {
    match power {
        PowerType::Rage => [1.0, 0.0, 0.0],
        PowerType::Focus => [1.0, 0.5, 0.25],
        PowerType::Energy => [1.0, 1.0, 0.0],
        PowerType::RunicPower => [0.0, 0.82, 1.0],
        PowerType::LunarPower => [0.30, 0.52, 0.90],
        PowerType::Maelstrom => [0.0, 0.5, 1.0],
        PowerType::Insanity => [0.40, 0.0, 0.80],
        PowerType::Fury => [0.788, 0.259, 0.992],
        PowerType::Pain => [1.0, 156.0 / 255.0, 0.0],
        _ => [0.0, 0.0, 1.0],
    }
}

fn build_ready_check(group: &GroupState, local_name: Option<&str>) -> ReadyCheckFrameState {
    let Some(view) = &group.ready_check else {
        return ReadyCheckFrameState::default();
    };
    let visible = local_name
        .is_some_and(|name| group.awaits_ready_answer(name) && view.update.initiator_name != name);
    ReadyCheckFrameState {
        visible,
        initiator: view.update.initiator_name.clone(),
    }
}

// --- Menu ---

fn build_menu(
    menu: &GroupFrameMenu,
    group: &GroupState,
    local_name: Option<&str>,
    entities: &HashMap<String, Entity>,
) -> GroupContextMenuState {
    let Some(unit) = &menu.unit else {
        return GroupContextMenuState::default();
    };
    let is_self = local_name == Some(unit.as_str());
    let mut items = Vec::new();
    if !is_self && entities.contains_key(unit) {
        items.push(menu_item("Target", "Target", ACTION_GROUP_MENU_TARGET));
        items.push(menu_item("Inspect", "Inspect", ACTION_GROUP_MENU_INSPECT));
    }
    if let Some(local) = local_name {
        items.extend(
            group_menu_entries(group, local, unit)
                .into_iter()
                .map(|entry| menu_item(entry.frame_key(), entry.label(), entry.action())),
        );
    }
    items.push(menu_item("Close", "Close", ACTION_GROUP_MENU_CLOSE));
    GroupContextMenuState {
        visible: true,
        title: unit.clone(),
        x: menu.x,
        y: menu.y,
        items,
    }
}

fn menu_item(key: &str, label: &str, action: &str) -> GroupMenuItem {
    GroupMenuItem {
        name: format!("GroupContextMenu{key}"),
        label: label.into(),
        action: action.into(),
    }
}

// --- Invite popup ---

fn invite_popup_spec(inviter: &str) -> PopupSpec {
    PopupSpec {
        key: PARTY_INVITE_POPUP.into(),
        // Retail INVITATION.
        text: format!("{inviter} invites you to a group."),
        accept_label: "Accept".into(),
        cancel_label: Some("Decline".into()),
        timeout: Some(Duration::from_secs_f32(GROUP_INVITE_TIMEOUT_SECS)),
    }
}

fn sync_invite_popup(group: Res<GroupState>, mut stack: ResMut<PopupStack>) {
    match &group.pending_invite {
        Some(inviter) if !stack.contains(PARTY_INVITE_POPUP) => {
            stack.push(invite_popup_spec(inviter));
        }
        None if stack.contains(PARTY_INVITE_POPUP) => {
            stack.hide(PARTY_INVITE_POPUP);
        }
        _ => {}
    }
}

/// Accept sends `AcceptGroup`; Decline, Escape and the 60 s timeout decline, as Retail's
/// `PARTY_INVITE` `OnHide` does.
fn answer_invite_popup(
    mut results: MessageReader<PopupResult>,
    mut group: ResMut<GroupState>,
    mut commands: MessageWriter<GroupCommand>,
) {
    for result in results.read() {
        if result.key != PARTY_INVITE_POPUP {
            continue;
        }
        group.pending_invite = None;
        commands.write(GroupCommand::RespondInvite(
            result.outcome == PopupOutcome::Accepted,
        ));
    }
}

// --- Pointer ---

#[derive(SystemParam)]
struct PointerGate<'w> {
    reconnect: Option<Res<'w, crate::networking::ReconnectState>>,
    modal_open: Option<Res<'w, crate::scenes::game_menu::UiModalOpen>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
}

fn handle_group_frame_pointer(
    windows: Query<&Window, With<PrimaryWindow>>,
    gate: PointerGate,
    ui: Res<UiState>,
    click_map: Res<GroupFrameClickMap>,
    players: Query<(Entity, &NetPlayer)>,
    mut menu: ResMut<GroupFrameMenu>,
    mut current_target: ResMut<CurrentTarget>,
    mut inspect_runtime: Option<ResMut<InspectRuntimeState>>,
    mut commands: MessageWriter<GroupCommand>,
) {
    if !crate::networking::gameplay_input_allowed(gate.reconnect) || gate.modal_open.is_some() {
        return;
    }
    let Some(mouse) = gate.mouse else { return };
    let button = if mouse.just_pressed(MouseButton::Left) {
        MouseButton::Left
    } else if mouse.just_pressed(MouseButton::Right) {
        MouseButton::Right
    } else {
        return;
    };
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    let entity_of = |name: &str| {
        players
            .iter()
            .find(|(_, player)| player.name == name)
            .map(|(entity, _)| entity)
    };
    let click = GroupFrameClick {
        registry: &ui.registry,
        click_map: &click_map,
        entity_of: &entity_of,
    };
    for outcome in click.handle(cursor, button, &mut menu) {
        match outcome {
            ClickOutcome::Target(entity) => current_target.0 = Some(entity),
            ClickOutcome::Inspect(entity) => {
                current_target.0 = Some(entity);
                if let Some(runtime) = inspect_runtime.as_deref_mut() {
                    game_engine::inspect::request_query_for_target(runtime, Some(entity));
                }
            }
            ClickOutcome::Command(command) => {
                commands.write(command);
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum ClickOutcome {
    Target(Entity),
    Inspect(Entity),
    Command(GroupCommand),
}

struct GroupFrameClick<'a> {
    registry: &'a FrameRegistry,
    click_map: &'a GroupFrameClickMap,
    entity_of: &'a dyn Fn(&str) -> Option<Entity>,
}

impl GroupFrameClick<'_> {
    /// Left-click on a member targets it; right-click opens its menu; a left-click on a
    /// menu entry or ready check button runs it. Any other click closes the menu.
    fn handle(
        &self,
        cursor: Vec2,
        button: MouseButton,
        menu: &mut GroupFrameMenu,
    ) -> Vec<ClickOutcome> {
        let Some(frame) = find_frame_at(self.registry, cursor.x, cursor.y) else {
            menu.unit = None;
            return Vec::new();
        };
        if button == MouseButton::Left
            && let Some(action) = walk_up_for_onclick(self.registry, frame)
        {
            return self.run_action(&action, menu);
        }
        let member = self.member_at(frame);
        if button == MouseButton::Right {
            menu.unit = member.clone();
            if member.is_some() {
                let max_x = (self.registry.screen_width - GROUP_MENU_W).max(0.0);
                let max_y =
                    (self.registry.screen_height - group_menu_height(MENU_MAX_ITEMS)).max(0.0);
                menu.x = cursor.x.clamp(0.0, max_x);
                menu.y = cursor.y.clamp(0.0, max_y);
            }
            return Vec::new();
        }
        menu.unit = None;
        member
            .and_then(|name| (self.entity_of)(&name))
            .map(ClickOutcome::Target)
            .into_iter()
            .collect()
    }

    fn run_action(&self, action: &str, menu: &mut GroupFrameMenu) -> Vec<ClickOutcome> {
        let outcome = match action {
            ACTION_READY_CHECK_YES => {
                Some(ClickOutcome::Command(GroupCommand::RespondReadyCheck(true)))
            }
            ACTION_READY_CHECK_NO => Some(ClickOutcome::Command(GroupCommand::RespondReadyCheck(
                false,
            ))),
            _ => self.run_menu_action(action, menu.unit.as_deref()),
        };
        if action.starts_with("group_menu") {
            menu.unit = None;
        }
        outcome.into_iter().collect()
    }

    fn run_menu_action(&self, action: &str, unit: Option<&str>) -> Option<ClickOutcome> {
        let unit = unit?;
        match action {
            ACTION_GROUP_MENU_TARGET => (self.entity_of)(unit).map(ClickOutcome::Target),
            ACTION_GROUP_MENU_INSPECT => (self.entity_of)(unit).map(ClickOutcome::Inspect),
            _ => GroupMenuEntry::from_action(action)
                .map(|entry| ClickOutcome::Command(entry.command(unit))),
        }
    }

    fn member_at(&self, mut frame: u64) -> Option<String> {
        loop {
            let data = self.registry.get(frame)?;
            if let Some(name) = data.name.as_deref()
                && let Some(member) = self.member_named(name)
            {
                return Some(member);
            }
            frame = data.parent_id?;
        }
    }

    fn member_named(&self, frame_name: &str) -> Option<String> {
        let party = (0..self.click_map.party.len())
            .find(|&i| party_member_frame_name(i) == frame_name)
            .map(|i| self.click_map.party[i].clone());
        party.or_else(|| {
            self.click_map
                .raid
                .iter()
                .enumerate()
                .find_map(|(group, members)| {
                    (0..members.len())
                        .find(|&m| raid_member_frame_name(group, m) == frame_name)
                        .map(|m| members[m].clone())
                })
        })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
