use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::raid_party_data::{GroupIntentQueue, LootMethod, LootThreshold, PartyState};
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::loot_rules_frame_component::{
    ACTION_CLOSE, ACTION_METHOD_PREFIX, ACTION_THRESHOLD_PREFIX, LootRulesFrameState,
    loot_rules_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;
use crate::ui_input::walk_up_for_onclick;

#[derive(Resource, Default)]
pub struct LootRulesFrameOpen(pub bool);

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

pub struct LootRulesFramePlugin;

impl Plugin for LootRulesFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LootRulesFrameOpen>();
        app.init_resource::<PartyState>();
        app.init_resource::<GroupIntentQueue>();
        app.add_systems(OnEnter(GameState::InWorld), build_loot_rules_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_loot_rules_frame_ui);
        app.add_systems(
            Update,
            (
                toggle_loot_rules_frame,
                sync_loot_rules_frame_state,
                handle_loot_rules_input,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_loot_rules_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    open: Res<LootRulesFrameOpen>,
    party: Res<PartyState>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(&open, &party);
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
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    mut open: ResMut<LootRulesFrameOpen>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyL) {
        open.0 = !open.0;
    }
}

fn sync_loot_rules_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<LootRulesFrameWrap>>,
    mut last_model: Option<ResMut<LootRulesFrameModel>>,
    open: Res<LootRulesFrameOpen>,
    party: Res<PartyState>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(&open, &party);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(open: &LootRulesFrameOpen, party: &PartyState) -> LootRulesFrameState {
    LootRulesFrameState {
        visible: open.0,
        group_summary: build_group_summary(party),
        current_method: party.loot.method,
        current_threshold: party.loot.threshold,
    }
}

fn build_group_summary(party: &PartyState) -> String {
    let count = party.member_count() + 1;
    if party.member_count() == 0 {
        return "Solo: changes stay local until group sync exists".into();
    }
    format!(
        "Party of {count} • method: {} • threshold: {}",
        party.loot.method.label(),
        party.loot.threshold.label()
    )
}

fn handle_loot_rules_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    mut open_state: ResMut<LootRulesFrameOpen>,
    mut party: ResMut<PartyState>,
    mut queue: ResMut<GroupIntentQueue>,
) {
    if !open_state.0
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
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    dispatch_action(&action, &mut open_state, &mut party, &mut queue);
}

fn dispatch_action(
    action: &str,
    open: &mut LootRulesFrameOpen,
    party: &mut PartyState,
    queue: &mut GroupIntentQueue,
) {
    if action == ACTION_CLOSE {
        open.0 = false;
        return;
    }
    if let Some(method) = parse_loot_method_action(action) {
        party.loot.method = method;
        queue.set_loot_method(method);
        return;
    }
    if let Some(threshold) = parse_loot_threshold_action(action) {
        party.loot.threshold = threshold;
        queue.set_loot_threshold(threshold);
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

    #[test]
    fn build_state_formats_group_summary() {
        let mut party = PartyState::default();
        party.members = vec![sample_member(), sample_member()];
        party.loot.method = LootMethod::MasterLooter;
        party.loot.threshold = LootThreshold::Epic;

        let state = build_state(&LootRulesFrameOpen(true), &party);

        assert!(state.visible);
        assert_eq!(state.current_method, LootMethod::MasterLooter);
        assert_eq!(state.current_threshold, LootThreshold::Epic);
        assert_eq!(
            state.group_summary,
            "Party of 3 • method: Master Looter • threshold: Epic"
        );
    }

    #[test]
    fn dispatch_personal_loot_updates_party_and_queue() {
        let mut open = LootRulesFrameOpen(true);
        let mut party = PartyState::default();
        let mut queue = GroupIntentQueue::default();

        dispatch_action(
            &format!(
                "{ACTION_METHOD_PREFIX}{}",
                game_engine::ui::screens::loot_rules_frame_component::loot_method_token(
                    LootMethod::PersonalLoot
                )
            ),
            &mut open,
            &mut party,
            &mut queue,
        );

        assert_eq!(party.loot.method, LootMethod::PersonalLoot);
        assert_eq!(queue.pending.len(), 1);
        assert!(matches!(
            queue.pending[0],
            game_engine::raid_party_data::GroupIntent::SetLootMethod {
                method: LootMethod::PersonalLoot
            }
        ));
    }

    #[test]
    fn dispatch_threshold_updates_party_and_queue() {
        let mut open = LootRulesFrameOpen(true);
        let mut party = PartyState::default();
        let mut queue = GroupIntentQueue::default();

        dispatch_action(
            &format!(
                "{ACTION_THRESHOLD_PREFIX}{}",
                game_engine::ui::screens::loot_rules_frame_component::loot_threshold_token(
                    LootThreshold::Epic
                )
            ),
            &mut open,
            &mut party,
            &mut queue,
        );

        assert_eq!(party.loot.threshold, LootThreshold::Epic);
        assert_eq!(queue.pending.len(), 1);
        assert!(matches!(
            queue.pending[0],
            game_engine::raid_party_data::GroupIntent::SetLootThreshold {
                threshold: LootThreshold::Epic
            }
        ));
    }

    fn sample_member() -> game_engine::raid_party_data::GroupUnitState {
        game_engine::raid_party_data::GroupUnitState {
            name: "Valeera".into(),
            health_current: 100,
            health_max: 100,
            power_current: 100,
            power_max: 100,
            power_type: game_engine::raid_party_data::PowerType::Mana,
            role: game_engine::raid_party_data::GroupRole::Dps,
            debuffs: Vec::new(),
            in_range: true,
            alive: true,
            online: true,
            ready_check: game_engine::raid_party_data::ReadyCheck::None,
            incoming_heals: 0,
        }
    }
}
