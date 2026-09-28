use bevy::prelude::*;

use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::campsite_popup_component::{
    CAMPSITE_POPUP_ROOT, campsite_popup_screen,
};
use game_engine::ui::screens::char_select_component::CampsiteState;
use game_engine::ui_resource;
use ui_toolkit::screen::Screen;

use crate::game_state::GameState;

ui_resource! {
    pub(crate) CampsitePopupUi {
        root: CAMPSITE_POPUP_ROOT,
    }
}

struct CampsitePopupScreenRes {
    screen: Screen,
}

unsafe impl Send for CampsitePopupScreenRes {}
unsafe impl Sync for CampsitePopupScreenRes {}

#[derive(Resource)]
struct CampsitePopupScreenWrap(CampsitePopupScreenRes);

pub struct CampsitePopupScreenPlugin;

impl Plugin for CampsitePopupScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::CampsitePopup), build_campsite_popup_ui);
        app.add_systems(OnExit(GameState::CampsitePopup), teardown_campsite_popup_ui);
    }
}

fn build_campsite_popup_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);

    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(build_campsite_popup_state());
    let mut screen = Screen::new(campsite_popup_screen);
    screen.sync(&shared, &mut ui.registry);

    let popup = CampsitePopupUi::resolve(&ui.registry);
    commands.insert_resource(CampsitePopupScreenWrap(CampsitePopupScreenRes { screen }));
    commands.insert_resource(popup);
}

fn build_campsite_popup_state() -> CampsiteState {
    let warband = crate::scenes::char_select::warband::WarbandScenes::load();
    let selected_id = warband.scenes.first().map(|scene| scene.id);
    CampsiteState {
        scenes: warband.campsite_entries(),
        panel_visible: true,
        selected_id,
        page: 0,
    }
}

fn teardown_campsite_popup_ui(
    mut ui: ResMut<UiState>,
    mut screen: Option<ResMut<CampsitePopupScreenWrap>>,
    mut commands: Commands,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<CampsitePopupScreenWrap>();
    commands.remove_resource::<CampsitePopupUi>();
    ui.focused_frame = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_state_loads_preview_images_for_every_listed_scene() {
        let state = build_campsite_popup_state();
        assert!(!state.scenes.is_empty());
        for scene in &state.scenes {
            assert!(
                scene.preview_image.is_some(),
                "{} has no card art",
                scene.name
            );
        }
        assert!(state.panel_visible);
        assert_eq!(
            state.selected_id,
            state.scenes.first().map(|scene| scene.id)
        );
    }
}
