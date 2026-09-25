use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::group_state::GroupState;
use game_engine::input_bindings::InputAction;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::loot_rules_frame_component::{
    ACTION_CLOSE, ACTION_METHOD_PREFIX, ACTION_THRESHOLD_PREFIX, LootMethod, LootRulesFrameState,
    LootThreshold, loot_rules_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};

struct LootRulesFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for LootRulesFrameRes {}
unsafe impl Sync for LootRulesFrameRes {}

#[derive(Resource)]
struct LootRulesFrameWrap(LootRulesFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct LootRulesFrameModel(LootRulesFrameState);

/// The window's selections. Client-side only: nothing sends them to the server.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
struct LootSettings {
    method: LootMethod,
    threshold: LootThreshold,
}

pub struct LootRulesFramePlugin;

impl Plugin for LootRulesFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LootSettings>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_loot_rules_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_loot_rules_frame_ui);
        app.add_systems(
            Update,
            (
                toggle_loot_rules_frame,
                sync_loot_rules_frame_state,
                handle_loot_rules_input,
            )
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn build_loot_rules_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    window_manager: Res<WindowManager>,
    loot: Res<LootSettings>,
    group: Res<GroupState>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(window_manager.is_open(WindowId::LootRules), &loot, &group);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(loot_rules_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(LootRulesFrameWrap(LootRulesFrameRes { screen, shared }));
    commands.insert_resource(LootRulesFrameModel(state));
}

fn teardown_loot_rules_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<LootRulesFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<LootRulesFrameWrap>();
    commands.remove_resource::<LootRulesFrameModel>();
}

fn toggle_loot_rules_frame(
    keybinds: crate::ui_input_mode::WorldKeybinds,
    mut window_manager: ResMut<WindowManager>,
) {
    if keybinds.just_pressed(InputAction::ToggleLootRules) {
        window_manager.toggle(WindowId::LootRules);
    }
}

fn sync_loot_rules_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<LootRulesFrameWrap>>,
    mut last_model: Option<ResMut<LootRulesFrameModel>>,
    window_manager: Res<WindowManager>,
    loot: Res<LootSettings>,
    group: Res<GroupState>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(window_manager.is_open(WindowId::LootRules), &loot, &group);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(open: bool, loot: &LootSettings, group: &GroupState) -> LootRulesFrameState {
    LootRulesFrameState {
        visible: open,
        group_summary: build_group_summary(loot, group),
        current_method: loot.method,
        current_threshold: loot.threshold,
    }
}

fn build_group_summary(loot: &LootSettings, group: &GroupState) -> String {
    if !group.in_group() {
        return "Solo: changes stay local until group sync exists".into();
    }
    format!(
        "Party of {} • method: {} • threshold: {}",
        group.members.len(),
        loot.method.label(),
        loot.threshold.label()
    )
}

fn handle_loot_rules_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    mut window_manager: ResMut<WindowManager>,
    mut loot: ResMut<LootSettings>,
) {
    if !window_manager.is_open(WindowId::LootRules)
        || !crate::networking::gameplay_input_allowed(reconnect)
        || modal_open.is_some()
    {
        return;
    }
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    dispatch_action(&action, &mut window_manager, &mut loot);
}

fn dispatch_action(action: &str, window_manager: &mut WindowManager, loot: &mut LootSettings) {
    if action == ACTION_CLOSE {
        window_manager.close(WindowId::LootRules);
        return;
    }
    if let Some(method) = parse_loot_method_action(action) {
        loot.method = method;
        return;
    }
    if let Some(threshold) = parse_loot_threshold_action(action) {
        loot.threshold = threshold;
    }
}

fn parse_loot_method_action(action: &str) -> Option<LootMethod> {
    let token = action.strip_prefix(ACTION_METHOD_PREFIX)?;
    match token {
        "free_for_all" => Some(LootMethod::FreeForAll),
        "round_robin" => Some(LootMethod::RoundRobin),
        "master_looter" => Some(LootMethod::MasterLooter),
        "group_loot" => Some(LootMethod::GroupLoot),
        "need_before_greed" => Some(LootMethod::NeedBeforeGreed),
        "personal_loot" => Some(LootMethod::PersonalLoot),
        _ => None,
    }
}

fn parse_loot_threshold_action(action: &str) -> Option<LootThreshold> {
    let token = action.strip_prefix(ACTION_THRESHOLD_PREFIX)?;
    match token {
        "poor" => Some(LootThreshold::Poor),
        "common" => Some(LootThreshold::Common),
        "uncommon" => Some(LootThreshold::Uncommon),
        "rare" => Some(LootThreshold::Rare),
        "epic" => Some(LootThreshold::Epic),
        "legendary" => Some(LootThreshold::Legendary),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::loot::LootMode;
    use shared::protocol::{GroupMemberSnapshot, GroupRoleSnapshot, GroupRosterSnapshot};

    fn group_of(names: &[&str]) -> GroupState {
        let mut group = GroupState::default();
        group.apply_roster(GroupRosterSnapshot {
            is_raid: false,
            ready_count: 0,
            total_count: 0,
            members: names
                .iter()
                .map(|name| GroupMemberSnapshot {
                    name: (*name).into(),
                    role: GroupRoleSnapshot::None,
                    is_leader: false,
                    online: true,
                    subgroup: 1,
                    class: 4,
                    level: 10,
                    entity: None,
                })
                .collect(),
            loot_method: LootMode::PersonalLoot,
        });
        group
    }

    #[test]
    fn build_state_formats_group_summary() {
        let loot = LootSettings {
            method: LootMethod::MasterLooter,
            threshold: LootThreshold::Epic,
        };

        let state = build_state(true, &loot, &group_of(&["Ann", "Bob", "Valeera"]));

        assert!(state.visible);
        assert_eq!(state.current_method, LootMethod::MasterLooter);
        assert_eq!(state.current_threshold, LootThreshold::Epic);
        assert_eq!(
            state.group_summary,
            "Party of 3 • method: Master Looter • threshold: Epic"
        );
        assert_eq!(
            build_state(true, &loot, &GroupState::default()).group_summary,
            "Solo: changes stay local until group sync exists"
        );
    }

    #[test]
    fn dispatch_method_and_threshold_update_settings() {
        let mut window_manager = WindowManager::default();
        let mut loot = LootSettings::default();

        dispatch_action(
            &format!(
                "{ACTION_METHOD_PREFIX}{}",
                game_engine::ui::screens::loot_rules_frame_component::loot_method_token(
                    LootMethod::PersonalLoot
                )
            ),
            &mut window_manager,
            &mut loot,
        );
        dispatch_action(
            &format!(
                "{ACTION_THRESHOLD_PREFIX}{}",
                game_engine::ui::screens::loot_rules_frame_component::loot_threshold_token(
                    LootThreshold::Epic
                )
            ),
            &mut window_manager,
            &mut loot,
        );

        assert_eq!(
            loot,
            LootSettings {
                method: LootMethod::PersonalLoot,
                threshold: LootThreshold::Epic,
            }
        );
    }
}
