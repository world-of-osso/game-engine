use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::screens::options_menu_component::{OptionsViewModel, options_view};
use crate::ui::strata::FrameStrata;

#[path = "game_menu_main.rs"]
mod main;
use main::main_menu_view;
pub use main::{
    ACTION_ADDONS, ACTION_EXIT, ACTION_LOGOUT, ACTION_OPTIONS, ACTION_RESUME, ACTION_SUPPORT,
    GAME_MENU_ROOT,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMenuView {
    MainMenu,
    Options,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameMenuViewModel {
    pub logged_in: bool,
    pub view: GameMenuView,
    pub options: OptionsViewModel,
}

pub fn game_menu_screen(shared: &SharedContext) -> Element {
    let Some(model) = shared.get::<GameMenuViewModel>() else {
        return Vec::new();
    };
    match model.view {
        GameMenuView::MainMenu => main_menu_view(model.logged_in),
        GameMenuView::Options => options_menu_overlay(&model.options),
    }
}

fn options_menu_overlay(options: &OptionsViewModel) -> Element {
    let options = options_view(options);
    rsx! {
        r#frame {
            name: GAME_MENU_ROOT,
            stretch: true,
            background_color: "0.01,0.01,0.02,0.75",
            strata: FrameStrata::Dialog,
            mouse_enabled: true,
            {options}
        }
    }
}

// Native layout tests require Bevy and the renderer-backed UI toolkit.
