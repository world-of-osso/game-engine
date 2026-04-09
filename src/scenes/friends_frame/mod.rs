use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::status::{FriendsStatusSnapshot, WhoStatusSnapshot};
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::friends_frame_component::{
    FriendEntry as UiFriendEntry, FriendsFrameState, FriendsFrameTabKind, FriendsTab,
    WhoEntry as UiWhoEntry, friends_frame_screen,
};
use game_engine::who::{WhoRuntimeState, queue_query};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;
use crate::ui_input::walk_up_for_onclick;

#[derive(Resource, Default)]
pub struct FriendsFrameOpen(pub bool);

#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
struct FriendsFrameSelection(FriendsFrameTabKind);

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
        app.init_resource::<FriendsFrameOpen>();
        app.init_resource::<FriendsFrameSelection>();
        app.add_systems(OnEnter(GameState::InWorld), build_friends_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_friends_frame_ui);
        app.add_systems(
            Update,
            (
                toggle_friends_frame,
                sync_friends_frame_state,
                handle_friends_frame_input,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_friends_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    snapshot: Option<Res<FriendsStatusSnapshot>>,
    who_snapshot: Option<Res<WhoStatusSnapshot>>,
    open: Res<FriendsFrameOpen>,
    selection: Res<FriendsFrameSelection>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(
        snapshot.as_deref(),
        who_snapshot.as_deref(),
        &open,
        &selection,
    );
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
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    mut open: ResMut<FriendsFrameOpen>,
    selection: Res<FriendsFrameSelection>,
    mut who_runtime: ResMut<WhoRuntimeState>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyO) {
        open.0 = !open.0;
        if open.0 && selection.0 == FriendsFrameTabKind::Who {
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
    open: Res<FriendsFrameOpen>,
    selection: Res<FriendsFrameSelection>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(
        snapshot.as_deref(),
        who_snapshot.as_deref(),
        &open,
        &selection,
    );
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
    open: &FriendsFrameOpen,
    selection: &FriendsFrameSelection,
) -> FriendsFrameState {
    FriendsFrameState {
        visible: open.0,
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
        FriendsFrameTabKind::Raid => "Raid roster is not implemented yet.".into(),
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
    open: Res<FriendsFrameOpen>,
    mut selection: ResMut<FriendsFrameSelection>,
    mut who_runtime: ResMut<WhoRuntimeState>,
) {
    if !open.0 || !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
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
    dispatch_action(&action, &mut selection, &mut who_runtime);
}

fn dispatch_action(
    action: &str,
    selection: &mut FriendsFrameSelection,
    who_runtime: &mut WhoRuntimeState,
) {
    let Some(tab) = FriendsFrameTabKind::from_action(action) else {
        return;
    };
    selection.0 = tab;
    if tab == FriendsFrameTabKind::Who {
        queue_query(who_runtime, String::new());
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
            &FriendsFrameOpen(true),
            &FriendsFrameSelection(FriendsFrameTabKind::Who),
        );

        assert_eq!(state.active_tab, FriendsFrameTabKind::Who);
        assert_eq!(state.who_query, "ali");
        assert_eq!(state.who_results.len(), 1);
        assert_eq!(state.who_results[0].name, "Alice");
    }

    #[test]
    fn dispatch_action_switches_to_who_and_queues_refresh() {
        let mut selection = FriendsFrameSelection::default();
        let mut runtime = WhoRuntimeState::default();

        dispatch_action("friends_tab:who", &mut selection, &mut runtime);

        assert_eq!(selection.0, FriendsFrameTabKind::Who);
        assert_eq!(game_engine::who::queued_query_count(&runtime), 1);
        assert_eq!(game_engine::who::first_queued_query(&runtime), Some(""));
    }
}
