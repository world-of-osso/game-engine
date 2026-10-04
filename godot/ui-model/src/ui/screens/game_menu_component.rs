use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::input_bindings::BindingSection;
use crate::ui::screens::options_menu_component::{
    OPTIONS_CONTENT_SCROLL, OptionsCategory, OptionsViewModel, options_view,
};
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

impl GameMenuViewModel {
    /// What the Options content area shows: the view, the category and, on Keybindings, the
    /// section.
    pub fn options_page(&self) -> (GameMenuView, OptionsCategory, BindingSection) {
        let options = &self.options;
        (self.view, options.category, options.bindings.section)
    }
}

pub fn game_menu_screen(shared: &SharedContext) -> Element {
    let Some(model) = shared.get::<GameMenuViewModel>() else {
        return Vec::new();
    };
    match model.view {
        GameMenuView::MainMenu => main_menu_view(model.logged_in),
        GameMenuView::Options => options_menu_overlay(
            &model.options,
            shared.scroll_first_row(OPTIONS_CONTENT_SCROLL),
        ),
    }
}

fn options_menu_overlay(options: &OptionsViewModel, first_item: usize) -> Element {
    let options = options_view(options, first_item);
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
