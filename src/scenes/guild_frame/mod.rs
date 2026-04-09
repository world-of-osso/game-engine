use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::guild::{GuildRuntimeState, queue_query};
use game_engine::status::GuildStatusSnapshot;
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::guild_frame_component::{
    ACTION_GUILD_TOGGLE, GuildFrameState, GuildMemberRow, GuildTab, GuildTabKind,
    guild_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;
use crate::ui_input::walk_up_for_onclick;

#[derive(Resource, Default)]
pub struct GuildFrameOpen(pub bool);

#[derive(Resource, Clone, Copy, PartialEq, Eq, Default)]
struct GuildFrameSelection(GuildTabKind);

struct GuildFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for GuildFrameRes {}
unsafe impl Sync for GuildFrameRes {}

#[derive(Resource)]
struct GuildFrameWrap(GuildFrameRes);

#[derive(Resource, Clone, PartialEq, Eq)]
struct GuildFrameModel(GuildFrameState);

pub struct GuildFramePlugin;

impl Plugin for GuildFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuildFrameOpen>();
        app.init_resource::<GuildFrameSelection>();
        app.add_systems(OnEnter(GameState::InWorld), build_guild_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_guild_frame_ui);
        app.add_systems(
            Update,
            (
                toggle_guild_frame,
                sync_guild_frame_state,
                handle_guild_frame_input,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_guild_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    snapshot: Option<Res<GuildStatusSnapshot>>,
    open: Res<GuildFrameOpen>,
    selection: Res<GuildFrameSelection>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(snapshot.as_deref(), &open, &selection);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(guild_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(GuildFrameWrap(GuildFrameRes { screen, shared }));
    commands.insert_resource(GuildFrameModel(state));
}

fn teardown_guild_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<GuildFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<GuildFrameWrap>();
    commands.remove_resource::<GuildFrameModel>();
}

fn toggle_guild_frame(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    mut open: ResMut<GuildFrameOpen>,
    mut runtime: ResMut<GuildRuntimeState>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
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
    if action == ACTION_GUILD_TOGGLE {
        open.0 = !open.0;
        if open.0 {
            queue_query(&mut runtime);
        }
    }
}

fn sync_guild_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<GuildFrameWrap>>,
    mut last_model: Option<ResMut<GuildFrameModel>>,
    snapshot: Option<Res<GuildStatusSnapshot>>,
    open: Res<GuildFrameOpen>,
    selection: Res<GuildFrameSelection>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(snapshot.as_deref(), &open, &selection);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(
    snapshot: Option<&GuildStatusSnapshot>,
    open: &GuildFrameOpen,
    selection: &GuildFrameSelection,
) -> GuildFrameState {
    let status_text = snapshot
        .and_then(|snapshot| {
            snapshot
                .last_error
                .clone()
                .or_else(|| snapshot.last_server_message.clone())
        })
        .unwrap_or_default();
    GuildFrameState {
        visible: open.0,
        guild_name: snapshot.map(|s| s.guild_name.clone()).unwrap_or_default(),
        motd: snapshot.map(|s| s.motd.clone()).unwrap_or_default(),
        info_text: snapshot.map(|s| s.info_text.clone()).unwrap_or_default(),
        status_text,
        active_tab: selection.0,
        tabs: [
            (GuildTabKind::Roster, "Roster"),
            (GuildTabKind::Info, "Info"),
        ]
        .into_iter()
        .map(|(tab, name)| GuildTab {
            name: name.into(),
            active: tab == selection.0,
            action: tab.action().into(),
        })
        .collect(),
        members: snapshot
            .map(|snapshot| {
                snapshot
                    .entries
                    .iter()
                    .map(|entry| GuildMemberRow {
                        name: entry.character_name.clone(),
                        level: entry.level,
                        class_name: entry.class_name.clone(),
                        rank_name: entry.rank_name.clone(),
                        status: if entry.online {
                            "Online".into()
                        } else {
                            entry.last_online.clone()
                        },
                        officer_note: entry.officer_note.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn handle_guild_frame_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    open: Res<GuildFrameOpen>,
    mut selection: ResMut<GuildFrameSelection>,
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
    if let Some(tab) = GuildTabKind::from_action(&action) {
        selection.0 = tab;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_state_maps_officer_notes() {
        let snapshot = GuildStatusSnapshot {
            guild_id: Some(7),
            guild_name: "Raid Team".into(),
            motd: "Bring flasks".into(),
            info_text: "Wed/Sun raids".into(),
            entries: vec![game_engine::status::GuildMemberEntry {
                character_name: "Alice".into(),
                level: 60,
                class_name: "Priest".into(),
                rank_name: "Member".into(),
                online: true,
                officer_note: "Reliable healer".into(),
                last_online: "Online".into(),
            }],
            last_server_message: Some("guild loaded".into()),
            last_error: None,
        };

        let state = build_state(
            Some(&snapshot),
            &GuildFrameOpen(true),
            &GuildFrameSelection(GuildTabKind::Roster),
        );

        assert_eq!(state.guild_name, "Raid Team");
        assert_eq!(state.members[0].officer_note, "Reliable healer");
        assert_eq!(state.status_text, "guild loaded");
    }
}
