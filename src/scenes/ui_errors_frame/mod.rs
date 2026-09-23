use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::network_runtime::messages::MessageReceivers;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::ui_errors_frame_component::ui_errors_frame_screen;
use game_engine::ui::ui_errors::{UiErrors, cast_failed_text};
use shared::protocol::CastFailed;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;

struct UiErrorsFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for UiErrorsFrameRes {}
unsafe impl Sync for UiErrorsFrameRes {}

#[derive(Resource)]
struct UiErrorsFrameWrap(UiErrorsFrameRes);

#[derive(Resource, PartialEq)]
struct UiErrorsFrameModel(UiErrors);

pub struct UiErrorsFramePlugin;

impl Plugin for UiErrorsFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiErrors>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_ui_errors_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_ui_errors_ui);
        app.add_systems(
            Update,
            (tick_ui_errors, sync_ui_errors_ui)
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

/// The spell's power type is not known client-side yet, so resource errors use the
/// generic wording unless the server supplies `detail`.
pub(crate) fn receive_cast_failed(
    mut receivers: MessageReceivers<CastFailed>,
    mut errors: ResMut<UiErrors>,
) {
    for receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            errors.add(cast_failed_text(msg.reason, msg.detail.as_deref(), None));
        }
    }
}

fn build_ui_errors_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    errors: Res<UiErrors>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let mut shared = SharedContext::new();
    shared.insert(errors.clone());
    let mut screen = Screen::new(ui_errors_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(UiErrorsFrameWrap(UiErrorsFrameRes { screen, shared }));
    commands.insert_resource(UiErrorsFrameModel(errors.clone()));
}

fn teardown_ui_errors_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<UiErrorsFrameWrap>>,
    mut errors: ResMut<UiErrors>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    *errors = UiErrors::default();
    commands.remove_resource::<UiErrorsFrameWrap>();
    commands.remove_resource::<UiErrorsFrameModel>();
}

fn tick_ui_errors(time: Res<Time>, mut errors: ResMut<UiErrors>) {
    if !errors.lines.is_empty() {
        errors.tick(time.delta_secs());
    }
}

fn sync_ui_errors_ui(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<UiErrorsFrameWrap>>,
    mut last_model: Option<ResMut<UiErrorsFrameModel>>,
    errors: Res<UiErrors>,
) {
    let (Some(wrap), Some(last_model)) = (wrap.as_mut(), last_model.as_mut()) else {
        return;
    };
    if last_model.0 == *errors {
        return;
    }
    last_model.0 = errors.clone();
    let res = &mut wrap.0;
    res.shared.insert(errors.clone());
    res.screen.sync(&res.shared, &mut ui.registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use game_engine::network_runtime::messages::Inbox;
    use game_engine::ui::screens::ui_errors_frame_component::ui_error_line_name;
    use shared::spell_data::CastFailReason;
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::registry::FrameRegistry;

    fn failed(reason: CastFailReason) -> CastFailed {
        CastFailed {
            spell_id: 133,
            reason,
            detail: None,
        }
    }

    fn errors_app() -> App {
        let mut app = App::new();
        app.insert_resource(UiState {
            registry: FrameRegistry::new(1920.0, 1080.0),
            event_bus: Default::default(),
            focused_frame: None,
        });
        app.insert_resource(Time::<()>::default());
        app.init_resource::<UiErrors>();
        app.world_mut().run_system_once(build_ui_errors_ui).unwrap();
        app.add_systems(
            Update,
            (receive_cast_failed, tick_ui_errors, sync_ui_errors_ui).chain(),
        );
        app
    }

    fn deliver(app: &mut App, messages: Vec<CastFailed>) {
        app.insert_resource(Inbox::new(messages));
        app.update();
    }

    fn shown_lines(app: &App) -> Vec<String> {
        let reg = &app.world().resource::<UiState>().registry;
        (0..3)
            .filter_map(|index| {
                let frame = reg.get(reg.get_by_name(&ui_error_line_name(index))?)?;
                if frame.hidden {
                    return None;
                }
                match frame.widget_data.as_ref() {
                    Some(WidgetData::FontString(fs)) => Some(fs.text.clone()),
                    _ => None,
                }
            })
            .collect()
    }

    #[test]
    fn repeated_out_of_range_refreshes_one_line() {
        let mut app = errors_app();
        deliver(
            &mut app,
            vec![
                failed(CastFailReason::OutOfRange),
                failed(CastFailReason::OutOfRange),
            ],
        );
        assert_eq!(shown_lines(&app), ["Out of range."]);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(2));
        app.update();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::ZERO);
        deliver(&mut app, vec![failed(CastFailReason::OutOfRange)]);
        let errors = app.world().resource::<UiErrors>();
        assert_eq!(errors.lines.len(), 1);
        assert!(errors.lines[0].age < 2.0, "refresh resets the fade timer");
    }

    #[test]
    fn four_distinct_errors_show_three_newest() {
        let mut app = errors_app();
        deliver(
            &mut app,
            vec![
                failed(CastFailReason::OutOfRange),
                failed(CastFailReason::InvalidTarget),
                failed(CastFailReason::OnCooldown),
                failed(CastFailReason::NoTarget),
            ],
        );
        assert_eq!(
            shown_lines(&app),
            [
                "You have no target.",
                "Spell is not ready yet.",
                "Invalid target"
            ]
        );
    }
}
