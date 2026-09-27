use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::input_bindings::InputAction;
use game_engine::instance_state::{
    InstanceCatalog, InstanceCommand, InstanceState, seconds_to_time,
};
use game_engine::status::{FriendsStatusSnapshot, WhoStatusSnapshot};
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::friends_frame_component::{
    FriendEntry as UiFriendEntry, FriendsFrameState, FriendsFrameTabKind, FriendsTab,
    WhoEntry as UiWhoEntry, friends_frame_screen,
};
use game_engine::ui::screens::raid_info_frame_component::{
    ACTION_RAID_INFO_CLOSE, ACTION_RAID_INFO_EXTEND, ACTION_RAID_INFO_SELECT_PREFIX,
    ACTION_RAID_INFO_TOGGLE, RaidInfoRow, RaidInfoState,
};
use game_engine::who::{WhoRuntimeState, queue_query};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};

#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
struct FriendsFrameSelection(FriendsFrameTabKind);

/// `RaidInfoFrame` shown and its selected row (`RaidInfoFrame.selectedIndex`).
#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
struct RaidInfoSelection {
    open: bool,
    selected: Option<usize>,
}

/// What the Raid Info list is built from.
#[derive(bevy::ecs::system::SystemParam)]
struct RaidInfoSources<'w> {
    selection: Res<'w, RaidInfoSelection>,
    instances: Option<Res<'w, InstanceState>>,
    catalog: Option<Res<'w, InstanceCatalog>>,
    time: Res<'w, Time>,
}

impl RaidInfoSources<'_> {
    fn state(&self) -> RaidInfoState {
        let (Some(instances), Some(catalog)) = (self.instances.as_deref(), self.catalog.as_deref())
        else {
            return RaidInfoState::default();
        };
        build_raid_info(
            instances,
            catalog,
            *self.selection,
            self.time.elapsed_secs_f64(),
        )
    }
}

struct FriendsFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for FriendsFrameRes {}
unsafe impl Sync for FriendsFrameRes {}

#[derive(Resource)]
struct FriendsFrameWrap(FriendsFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct FriendsFrameModel(FriendsFrameState);

pub struct FriendsFramePlugin;

impl Plugin for FriendsFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FriendsFrameSelection>()
            .init_resource::<RaidInfoSelection>()
            .add_message::<InstanceCommand>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_friends_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_friends_frame_ui);
        app.add_systems(
            Update,
            (
                toggle_friends_frame,
                sync_friends_frame_state,
                handle_friends_frame_input,
            )
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn build_friends_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    snapshot: Option<Res<FriendsStatusSnapshot>>,
    who_snapshot: Option<Res<WhoStatusSnapshot>>,
    window_manager: Res<WindowManager>,
    selection: Res<FriendsFrameSelection>,
    raid_info: RaidInfoSources,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let mut state = build_state(
        snapshot.as_deref(),
        who_snapshot.as_deref(),
        window_manager.is_open(WindowId::Friends),
        &selection,
    );
    state.raid_info = raid_info.state();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(friends_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(FriendsFrameWrap(FriendsFrameRes { screen, shared }));
    commands.insert_resource(FriendsFrameModel(state));
}

fn teardown_friends_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<FriendsFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<FriendsFrameWrap>();
    commands.remove_resource::<FriendsFrameModel>();
}

fn toggle_friends_frame(
    keybinds: crate::ui_input_mode::WorldKeybinds,
    mut window_manager: ResMut<WindowManager>,
    selection: Res<FriendsFrameSelection>,
    mut who_runtime: ResMut<WhoRuntimeState>,
) {
    if keybinds.just_pressed(InputAction::ToggleSocial) {
        let opened = window_manager.toggle(WindowId::Friends);
        if opened && selection.0 == FriendsFrameTabKind::Who {
            queue_query(&mut who_runtime, String::new());
        }
    }
}

fn sync_friends_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<FriendsFrameWrap>>,
    mut last_model: Option<ResMut<FriendsFrameModel>>,
    snapshot: Option<Res<FriendsStatusSnapshot>>,
    who_snapshot: Option<Res<WhoStatusSnapshot>>,
    window_manager: Res<WindowManager>,
    selection: Res<FriendsFrameSelection>,
    raid_info: RaidInfoSources,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let mut state = build_state(
        snapshot.as_deref(),
        who_snapshot.as_deref(),
        window_manager.is_open(WindowId::Friends),
        &selection,
    );
    state.raid_info = raid_info.state();
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(
    snapshot: Option<&FriendsStatusSnapshot>,
    who_snapshot: Option<&WhoStatusSnapshot>,
    open: bool,
    selection: &FriendsFrameSelection,
) -> FriendsFrameState {
    FriendsFrameState {
        visible: open,
        active_tab: selection.0,
        tabs: build_tabs(selection.0),
        friends: snapshot
            .map(|snapshot| snapshot.entries.iter().map(map_friend_entry).collect())
            .unwrap_or_default(),
        who_query: who_snapshot
            .map(|snapshot| snapshot.query.clone())
            .unwrap_or_default(),
        who_results: who_snapshot
            .map(|snapshot| snapshot.entries.iter().map(map_who_entry).collect())
            .unwrap_or_default(),
        status_text: tab_status_text(selection.0, who_snapshot),
        raid_info: RaidInfoState::default(),
    }
}

/// `RaidInfoFrame_Update` + `RaidInfoFrame_UpdateButtons`: a row per saved instance, the
/// Extend button for the selected one (`UNEXTEND_RAID_LOCK` when extended, else
/// `EXTEND_RAID_LOCK` while locked or `REACTIVATE_RAID_LOCK`).
fn build_raid_info(
    instances: &InstanceState,
    catalog: &InstanceCatalog,
    selection: RaidInfoSelection,
    now: f64,
) -> RaidInfoState {
    let rows = instances
        .saved
        .iter()
        .map(|lock| {
            let active = lock.locked || lock.extended;
            RaidInfoRow {
                name: catalog.map_name(lock.map_id).to_owned(),
                difficulty: catalog.difficulty_name(lock.difficulty_id).to_owned(),
                reset: if active {
                    seconds_to_time(instances.time_remaining(lock, now))
                } else {
                    "Expired".into()
                },
                expired: !active,
                extended: lock.extended,
            }
        })
        .collect::<Vec<_>>();
    let selected = selection.selected.filter(|&index| index < rows.len());
    let selected_lock = selected.map(|index| &instances.saved[index]);
    let extend_text = match selected_lock {
        Some(lock) if lock.extended => "Remove Raid Lock Extension",
        Some(lock) if !lock.locked => "Reactivate Raid Lock",
        _ => "Extend Raid Lock",
    };
    RaidInfoState {
        visible: selection.open && !rows.is_empty(),
        button_enabled: !rows.is_empty(),
        extend_enabled: selected_lock.is_some_and(|lock| {
            !catalog
                .extension_disabled
                .contains(&(lock.map_id, lock.difficulty_id))
        }),
        extend_text: extend_text.into(),
        rows,
        selected,
    }
}

fn build_tabs(active: FriendsFrameTabKind) -> Vec<FriendsTab> {
    [
        (FriendsFrameTabKind::Friends, "Friends"),
        (FriendsFrameTabKind::Who, "Who"),
        (FriendsFrameTabKind::Raid, "Raid"),
        (FriendsFrameTabKind::QuickJoin, "Quick Join"),
    ]
    .into_iter()
    .map(|(tab, name)| FriendsTab {
        name: name.into(),
        active: tab == active,
        action: tab.action(),
    })
    .collect()
}

fn map_friend_entry(entry: &game_engine::status::FriendEntry) -> UiFriendEntry {
    UiFriendEntry {
        name: entry.name.clone(),
        game: format!("Lvl {} {} {}", entry.level, entry.class_name, entry.area),
        status: match entry.presence {
            game_engine::status::PresenceStateEntry::Online => "Online".into(),
            game_engine::status::PresenceStateEntry::Afk => "Away".into(),
            game_engine::status::PresenceStateEntry::Dnd => "Busy".into(),
            game_engine::status::PresenceStateEntry::Offline => "Offline".into(),
        },
        online: entry.online,
        is_bnet: false,
    }
}

fn map_who_entry(entry: &game_engine::status::WhoEntry) -> UiWhoEntry {
    UiWhoEntry {
        name: entry.name.clone(),
        details: format!("Lvl {} {} {}", entry.level, entry.class_name, entry.area),
    }
}

fn tab_status_text(
    active: FriendsFrameTabKind,
    who_snapshot: Option<&WhoStatusSnapshot>,
) -> String {
    match active {
        FriendsFrameTabKind::Who => who_snapshot
            .and_then(|snapshot| {
                snapshot
                    .last_error
                    .clone()
                    .or_else(|| snapshot.last_server_message.clone())
            })
            .unwrap_or_default(),
        FriendsFrameTabKind::Raid => String::new(),
        FriendsFrameTabKind::QuickJoin => "Quick Join is not implemented yet.".into(),
        FriendsFrameTabKind::Friends => String::new(),
    }
}

fn handle_friends_frame_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    window_manager: Res<WindowManager>,
    mut selection: ResMut<FriendsFrameSelection>,
    mut who_runtime: ResMut<WhoRuntimeState>,
    mut raid_info: ResMut<RaidInfoSelection>,
    instances: Option<Res<InstanceState>>,
    mut instance_commands: MessageWriter<InstanceCommand>,
) {
    if !window_manager.is_open(WindowId::Friends)
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
    let saved = instances
        .as_deref()
        .map_or(&[][..], |state| state.saved.as_slice());
    if let Some(command) = dispatch_raid_info_action(&action, &mut raid_info, saved) {
        instance_commands.write(command);
        return;
    }
    if dispatch_action(&action, &mut selection, &mut who_runtime) == Some(FriendsFrameTabKind::Raid)
    {
        // RaidFrame_OnShow → RequestRaidInfo.
        instance_commands.write(InstanceCommand::RequestRaidInfo);
    }
}

/// The tab `action` switched to, if any.
fn dispatch_action(
    action: &str,
    selection: &mut FriendsFrameSelection,
    who_runtime: &mut WhoRuntimeState,
) -> Option<FriendsFrameTabKind> {
    let tab = FriendsFrameTabKind::from_action(action)?;
    selection.0 = tab;
    if tab == FriendsFrameTabKind::Who {
        queue_query(who_runtime, String::new());
    }
    Some(tab)
}

/// Raid Info button, rows, Extend and Close (`RaidFrameRaidInfoButton_OnClick`,
/// `RaidInfoInstance_OnClick`, `RaidInfoExtendButton_OnClick`): the command to send.
fn dispatch_raid_info_action(
    action: &str,
    raid_info: &mut RaidInfoSelection,
    saved: &[shared::protocol::InstanceLockInfo],
) -> Option<InstanceCommand> {
    match action {
        ACTION_RAID_INFO_TOGGLE => {
            raid_info.open = !raid_info.open;
            raid_info.open.then_some(InstanceCommand::RequestRaidInfo)
        }
        ACTION_RAID_INFO_CLOSE => {
            raid_info.open = false;
            None
        }
        ACTION_RAID_INFO_EXTEND => {
            let lock = saved.get(raid_info.selected?)?;
            Some(InstanceCommand::SetExtended {
                map_id: lock.map_id,
                difficulty_id: lock.difficulty_id,
                extend: !lock.extended,
            })
        }
        _ => {
            raid_info.selected = Some(
                action
                    .strip_prefix(ACTION_RAID_INFO_SELECT_PREFIX)?
                    .parse()
                    .ok()?,
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_state_maps_who_results_for_who_tab() {
        let who_snapshot = WhoStatusSnapshot {
            query: "ali".into(),
            entries: vec![game_engine::status::WhoEntry {
                name: "Alice".into(),
                level: 42,
                class_name: "Mage".into(),
                area: "Zone 12".into(),
            }],
            last_server_message: Some("who: 1 result(s)".into()),
            last_error: None,
        };

        let state = build_state(
            None,
            Some(&who_snapshot),
            true,
            &FriendsFrameSelection(FriendsFrameTabKind::Who),
        );

        assert_eq!(state.active_tab, FriendsFrameTabKind::Who);
        assert_eq!(state.who_query, "ali");
        assert_eq!(state.who_results.len(), 1);
        assert_eq!(state.who_results[0].name, "Alice");
    }

    fn heroic_grim_batol_lock(
        time_remaining_secs: u32,
        locked: bool,
        extended: bool,
    ) -> shared::protocol::InstanceLockInfo {
        shared::protocol::InstanceLockInfo {
            map_id: 670,
            difficulty_id: 2,
            instance_id: 3,
            time_remaining_secs,
            completed_mask: 1 << 3,
            locked,
            extended,
        }
    }

    #[test]
    fn raid_info_lists_saved_instances_with_their_reset_and_extend_button() {
        let catalog = InstanceCatalog::load().unwrap();
        let instances = InstanceState {
            saved: vec![
                heroic_grim_batol_lock(3 * 3600 + 20 * 60, true, false),
                heroic_grim_batol_lock(0, false, false),
            ],
            saved_at: 10.0,
            ..Default::default()
        };
        let closed = build_raid_info(&instances, &catalog, RaidInfoSelection::default(), 70.0);
        assert!(closed.button_enabled && !closed.visible);
        let open = RaidInfoSelection {
            open: true,
            selected: Some(0),
        };
        let state = build_raid_info(&instances, &catalog, open, 70.0);
        assert!(state.visible);
        assert_eq!(
            state.rows,
            vec![
                RaidInfoRow {
                    name: "Grim Batol".into(),
                    difficulty: "Heroic".into(),
                    reset: "3 Hr 19 Min".into(),
                    expired: false,
                    extended: false,
                },
                RaidInfoRow {
                    name: "Grim Batol".into(),
                    difficulty: "Heroic".into(),
                    reset: "Expired".into(),
                    expired: true,
                    extended: false,
                },
            ]
        );
        assert_eq!(
            (state.extend_text.as_str(), state.extend_enabled),
            ("Extend Raid Lock", true)
        );
        let expired = build_raid_info(
            &instances,
            &catalog,
            RaidInfoSelection {
                open: true,
                selected: Some(1),
            },
            70.0,
        );
        assert_eq!(expired.extend_text, "Reactivate Raid Lock");
        let none = build_raid_info(&InstanceState::default(), &catalog, open, 0.0);
        assert!(!none.button_enabled && !none.visible);
    }

    #[test]
    fn raid_info_button_row_and_extend_send_their_requests() {
        let saved = [heroic_grim_batol_lock(3600, true, false)];
        let mut selection = RaidInfoSelection::default();
        assert_eq!(
            dispatch_raid_info_action(ACTION_RAID_INFO_TOGGLE, &mut selection, &saved),
            Some(InstanceCommand::RequestRaidInfo)
        );
        assert!(selection.open);
        assert_eq!(
            dispatch_raid_info_action(ACTION_RAID_INFO_EXTEND, &mut selection, &saved),
            None
        );
        dispatch_raid_info_action("raid_info_select:0", &mut selection, &saved);
        assert_eq!(selection.selected, Some(0));
        assert_eq!(
            dispatch_raid_info_action(ACTION_RAID_INFO_EXTEND, &mut selection, &saved),
            Some(InstanceCommand::SetExtended {
                map_id: 670,
                difficulty_id: 2,
                extend: true,
            })
        );
        dispatch_raid_info_action(ACTION_RAID_INFO_CLOSE, &mut selection, &saved);
        assert!(!selection.open);
    }

    #[test]
    fn dispatch_action_switches_to_who_and_queues_refresh() {
        let mut selection = FriendsFrameSelection::default();
        let mut runtime = WhoRuntimeState::default();

        assert_eq!(
            dispatch_action("friends_tab:who", &mut selection, &mut runtime),
            Some(FriendsFrameTabKind::Who)
        );

        assert_eq!(selection.0, FriendsFrameTabKind::Who);
        assert_eq!(game_engine::who::queued_query_count(&runtime), 1);
        assert_eq!(game_engine::who::first_queued_query(&runtime), Some(""));
    }
}
