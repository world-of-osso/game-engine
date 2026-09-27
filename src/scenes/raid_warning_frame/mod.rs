//! `RaidWarningFrame` in world: boss emotes (`ChatType::RaidBossEmote`) center screen.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::raid_warning::RaidWarnings;
use game_engine::ui::screens::raid_warning_frame_component::raid_warning_frame_screen;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;

struct RaidWarningFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for RaidWarningFrameRes {}
unsafe impl Sync for RaidWarningFrameRes {}

#[derive(Resource)]
struct RaidWarningFrameWrap(RaidWarningFrameRes);

#[derive(Resource, PartialEq)]
struct RaidWarningFrameModel(RaidWarnings);

pub struct RaidWarningFramePlugin;

impl Plugin for RaidWarningFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RaidWarnings>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_raid_warning_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_raid_warning_ui);
        app.add_systems(
            Update,
            (tick_raid_warnings, sync_raid_warning_ui)
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn build_raid_warning_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    warnings: Res<RaidWarnings>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let mut shared = SharedContext::new();
    shared.insert(warnings.clone());
    let mut screen = Screen::new(raid_warning_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(RaidWarningFrameWrap(RaidWarningFrameRes { screen, shared }));
    commands.insert_resource(RaidWarningFrameModel(warnings.clone()));
}

fn teardown_raid_warning_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<RaidWarningFrameWrap>>,
    mut warnings: ResMut<RaidWarnings>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    *warnings = RaidWarnings::default();
    commands.remove_resource::<RaidWarningFrameWrap>();
    commands.remove_resource::<RaidWarningFrameModel>();
}

fn tick_raid_warnings(time: Res<Time>, mut warnings: ResMut<RaidWarnings>) {
    if !warnings.lines.is_empty() {
        warnings.tick(time.delta_secs());
    }
}

fn sync_raid_warning_ui(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<RaidWarningFrameWrap>>,
    mut last_model: Option<ResMut<RaidWarningFrameModel>>,
    warnings: Res<RaidWarnings>,
) {
    let (Some(wrap), Some(last_model)) = (wrap.as_mut(), last_model.as_mut()) else {
        return;
    };
    if last_model.0 == *warnings {
        return;
    }
    last_model.0 = warnings.clone();
    let res = &mut wrap.0;
    res.shared.insert(warnings.clone());
    res.screen.sync(&res.shared, &mut ui.registry);
}
