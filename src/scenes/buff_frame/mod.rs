//! Mounts the player BuffFrame/DebuffFrame from `AuraState`; expiring auras flash and
//! right-click cancels a buff.

#[cfg(test)]
#[path = "../../ui/screens/menu_character_layout_test_support.rs"]
mod native_layout_support;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::buff_data::AuraState;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::buff_frame_component::{
    BuffFrameState, aura_button_name, aura_warning_alpha, buff_button_at, buff_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::client_options::GraphicsOptions;
use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking_auras::CancelAuraRequest;

struct BuffFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for BuffFrameRes {}
unsafe impl Sync for BuffFrameRes {}

#[derive(Resource)]
struct BuffFrameWrap(BuffFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct BuffFrameModel(BuffFrameState);

pub struct BuffFramePlugin;

impl Plugin for BuffFramePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_buff_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_buff_frame_ui);
        app.add_systems(
            Update,
            (
                (sync_buff_frame_ui, flash_expiring_auras).chain(),
                cancel_buff_on_right_click,
            )
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn frame_state(auras: Option<&AuraState>, graphics: Option<&GraphicsOptions>) -> BuffFrameState {
    let colorblind = graphics.is_some_and(|graphics| graphics.colorblind_mode);
    auras.map_or_else(BuffFrameState::default, |auras| {
        BuffFrameState::from_auras(&auras.auras, colorblind)
    })
}

fn build_buff_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    auras: Option<Res<AuraState>>,
    graphics: Option<Res<GraphicsOptions>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = frame_state(auras.as_deref(), graphics.as_deref());
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(buff_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(BuffFrameWrap(BuffFrameRes { screen, shared }));
    commands.insert_resource(BuffFrameModel(state));
}

fn teardown_buff_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<BuffFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<BuffFrameWrap>();
    commands.remove_resource::<BuffFrameModel>();
}

fn sync_buff_frame_ui(
    mut ui: ResMut<UiState>,
    wrap: Option<ResMut<BuffFrameWrap>>,
    last_model: Option<ResMut<BuffFrameModel>>,
    auras: Option<Res<AuraState>>,
    graphics: Option<Res<GraphicsOptions>>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap, last_model) else {
        return;
    };
    let state = frame_state(auras.as_deref(), graphics.as_deref());
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

/// Buttons of timed auras under `BUFF_WARNING_TIME` pulse on the shared fader clock.
fn flash_expiring_auras(
    time: Res<Time>,
    mut ui: ResMut<UiState>,
    wrap: Option<Res<BuffFrameWrap>>,
    auras: Option<Res<AuraState>>,
) {
    let (Some(_), Some(auras)) = (wrap, auras) else {
        return;
    };
    let clock = time.elapsed_secs();
    let buttons = (auras.buffs().enumerate().map(|(i, aura)| (false, i, aura)))
        .chain(auras.debuffs().enumerate().map(|(i, aura)| (true, i, aura)));
    for (is_debuff, index, aura) in buttons {
        let Some(id) = ui.registry.get_by_name(&aura_button_name(is_debuff, index)) else {
            continue;
        };
        let time_left = (!aura.is_permanent()).then_some(aura.remaining);
        let alpha = aura_warning_alpha(clock, time_left);
        if ui
            .registry
            .get(id)
            .is_some_and(|frame| frame.alpha != alpha)
        {
            ui.registry.set_alpha(id, alpha);
        }
    }
}

fn cancel_buff_on_right_click(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    ui: Res<UiState>,
    auras: Option<Res<AuraState>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    mut requests: MessageWriter<CancelAuraRequest>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) {
        return;
    }
    let (Some(mouse), Some(auras)) = (mouse, auras) else {
        return;
    };
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    if let Some(spell_id) = buff_to_cancel(&ui.registry, &auras, cursor) {
        requests.write(CancelAuraRequest { spell_id });
    }
}

/// Only buffs are cancelable; debuffs ignore right-clicks.
fn buff_to_cancel(registry: &FrameRegistry, auras: &AuraState, cursor: Vec2) -> Option<u32> {
    let frame_id = find_frame_at(registry, cursor.x, cursor.y)?;
    let (false, index) = buff_button_at(registry, frame_id)? else {
        return None;
    };
    auras.buffs().nth(index).map(|aura| aura.spell_id)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
