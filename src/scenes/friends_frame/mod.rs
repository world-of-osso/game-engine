use bevy::prelude::*;
use game_engine::status::FriendsStatusSnapshot;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::friends_frame_component::{
    FriendEntry as UiFriendEntry, FriendsFrameState, friends_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;

#[derive(Resource, Default)]
pub struct FriendsFrameOpen(pub bool);

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
        app.add_systems(OnEnter(GameState::InWorld), build_friends_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_friends_frame_ui);
        app.add_systems(
            Update,
            (toggle_friends_frame, sync_friends_frame_state).run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_friends_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    snapshot: Option<Res<FriendsStatusSnapshot>>,
    open: Res<FriendsFrameOpen>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(snapshot.as_deref(), &open);
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
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyO) {
        open.0 = !open.0;
    }
}

fn sync_friends_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<FriendsFrameWrap>>,
    mut last_model: Option<ResMut<FriendsFrameModel>>,
    snapshot: Option<Res<FriendsStatusSnapshot>>,
    open: Res<FriendsFrameOpen>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(snapshot.as_deref(), &open);
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
    open: &FriendsFrameOpen,
) -> FriendsFrameState {
    FriendsFrameState {
        visible: open.0,
        tabs: FriendsFrameState::default().tabs,
        friends: snapshot
            .map(|snapshot| snapshot.entries.iter().map(map_friend_entry).collect())
            .unwrap_or_default(),
    }
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
