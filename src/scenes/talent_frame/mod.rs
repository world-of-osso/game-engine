use bevy::prelude::*;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::talent_frame_component::{
    TALENT_COUNT, TalentFrameState, TalentNodeState, TalentSpecTab, talent_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;

/// Tracks whether the Talent panel is open.
#[derive(Resource, Default)]
pub struct TalentFrameOpen(pub bool);

struct TalentFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for TalentFrameRes {}
unsafe impl Sync for TalentFrameRes {}

#[derive(Resource)]
struct TalentFrameWrap(TalentFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct TalentFrameModel(TalentFrameState);

pub struct TalentFramePlugin;

impl Plugin for TalentFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TalentFrameOpen>();
        app.add_systems(OnEnter(GameState::InWorld), build_talent_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_talent_frame_ui);
        app.add_systems(
            Update,
            (toggle_talent_frame, sync_talent_frame_state).run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_talent_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    open: Res<TalentFrameOpen>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(&open);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(talent_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(TalentFrameWrap(TalentFrameRes { screen, shared }));
    commands.insert_resource(TalentFrameModel(state));
}

fn teardown_talent_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<TalentFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<TalentFrameWrap>();
    commands.remove_resource::<TalentFrameModel>();
}

fn toggle_talent_frame(
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    mut open: ResMut<TalentFrameOpen>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyN) {
        open.0 = !open.0;
    }
}

fn sync_talent_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<TalentFrameWrap>>,
    mut last_model: Option<ResMut<TalentFrameModel>>,
    open: Res<TalentFrameOpen>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(&open);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(open: &TalentFrameOpen) -> TalentFrameState {
    TalentFrameState {
        visible: open.0,
        spec_tabs: placeholder_spec_tabs(),
        talents: placeholder_talents(),
        points_remaining: 51,
    }
}

fn placeholder_spec_tabs() -> Vec<TalentSpecTab> {
    vec![
        TalentSpecTab {
            name: "Protection".to_string(),
            active: true,
        },
        TalentSpecTab {
            name: "Holy".to_string(),
            active: false,
        },
        TalentSpecTab {
            name: "Retribution".to_string(),
            active: false,
        },
    ]
}

fn placeholder_talents() -> Vec<TalentNodeState> {
    const NAMES: [&str; TALENT_COUNT] = [
        // Row 1
        "Divine Strength",
        "Divine Intellect",
        "Stoicism",
        "Guardian's Favor",
        // Row 2
        "Anticipation",
        "Conviction",
        "Toughness",
        "Improved Devotion Aura",
        // Row 3
        "Improved Righteous Fury",
        "Seal of the Crusader",
        "Deflection",
        "Precision",
        // Row 4
        "Redoubt",
        "Combat Expertise",
        "Spell Warding",
        "Blessing of Kings",
        // Row 5
        "Ardent Defender",
        "Reckoning",
        "Shield Specialization",
        "Holy Shield",
        // Row 6
        "One-Handed Weapon Spec",
        "Weapon Expertise",
        "Improved Holy Shield",
        "Sacred Duty",
        // Row 7
        "Holy Shield Mastery",
        "Avenger's Shield",
        "Hammer of the Righteous",
        "Touched by the Light",
    ];
    NAMES
        .iter()
        .map(|name| TalentNodeState {
            name: name.to_string(),
            points: "0/1".to_string(),
            active: false,
        })
        .collect()
}
