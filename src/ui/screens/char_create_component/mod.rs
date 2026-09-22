mod appearance_widgets;
mod char_create_widgets;
pub mod navigation_art;
mod reference_layout;
mod view_model;

pub use view_model::*;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

use appearance_widgets::{customization_row, dropdown_panel};

/// Restore authored MenuStyle2 atlas slicing after the declarative screen sync.
pub fn apply_character_create_styles(
    registry: &mut crate::ui::registry::FrameRegistry,
    open_dropdown: Option<u32>,
) {
    appearance_widgets::apply_dropdown_background_style(registry, open_dropdown);
}
use char_create_widgets::{
    body_type_buttons, bottom_buttons, camera_controls, category_button, class_button,
    faction_column, name_input_field, small_button,
};
use reference_layout::*;

pub const CHAR_CREATE_ROOT: FrameName = FrameName("CharCreateRoot");
pub const CREATE_NAME_INPUT: FrameName = FrameName("CharCreateNameInput");
pub const CREATE_BUTTON: FrameName = FrameName("CharCreateButton");
pub const BACK_BUTTON: FrameName = FrameName("CharCreateBack");
pub const NEXT_BUTTON: FrameName = FrameName("CharCreateNext");
pub const RANDOMIZE_BUTTON: FrameName = FrameName("CharCreateRandomize");
pub const ERROR_TEXT: FrameName = FrameName("CharCreateError");

pub(super) const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
pub(super) const COLOR_WHITE: FontColor = FontColor::new(1.0, 1.0, 1.0, 1.0);
pub(super) const COLOR_DISABLED: FontColor = FontColor::new(0.4, 0.4, 0.4, 1.0);
pub(super) struct DynName(pub String);

fn race_grid(state: &CharCreateUiState) -> Element {
    use crate::char_create_data::Faction;
    rsx! {
        r#frame { name: "RaceGrid", width: "fill", height: "fill",
            pos_type: "absolute", left: 0.0, top: 0.0,
            {faction_column(Faction::Alliance, state)}
            {faction_column(Faction::Horde, state)}
        }
    }
}

fn class_grid(state: &CharCreateUiState) -> Element {
    let layout = class_layout(state.viewport_width, state.class_availability.len());
    let buttons: Element = state
        .class_availability
        .iter()
        .enumerate()
        .flat_map(|(index, &(id, name, icon, available))| {
            let x = (index % layout.columns) as f32 * (layout.size + layout.gap);
            let y = (index / layout.columns) as f32 * (layout.size + 47.0 * layout.size / 67.0);
            class_button(
                id,
                name,
                icon,
                id == state.selected_class,
                available,
                [x, y, layout.size],
            )
        })
        .collect();
    rsx! {
        r#frame { name: "ClassGrid", width: layout.width, height: layout.height,
            pos_type: "absolute", left: "50%", bottom: 62.0, translate_x: "-50%",
            {buttons}
        }
    }
}

fn category_tabs(state: &CharCreateUiState) -> Element {
    let spacing = fit_spacing(
        state.viewport_width as f32 - 120.0,
        state.categories.len(),
        CATEGORY_WIDTH,
        -21.0,
    );
    let width = state.categories.len() as f32 * CATEGORY_WIDTH
        + state.categories.len().saturating_sub(1) as f32 * spacing;
    let buttons: Element = state
        .categories
        .iter()
        .enumerate()
        .flat_map(|(index, category)| {
            category_button(
                category,
                state.selected_category == category.id,
                index as f32 * (CATEGORY_WIDTH + spacing),
            )
        })
        .collect();
    let randomize_x = -(48.0 + 9.0);
    rsx! {
        r#frame { name: "CustomizationCategories", width, height: CATEGORY_HEIGHT,
            pos_type: "absolute", right: 20.0, top: CATEGORY_TOP,
            {buttons}
            {small_button(RANDOMIZE_BUTTON.0, "charactercreate-icon-dice", CharCreateAction::Randomize, randomize_x, (CATEGORY_HEIGHT - 48.0) / 2.0)}
        }
    }
}

fn customize_panel(state: &CharCreateUiState) -> Element {
    let layout = options_layout(state.viewport_height, state.options.len());
    let rows: Element = state
        .options
        .iter()
        .enumerate()
        .flat_map(|(index, option)| {
            customization_row(
                option,
                state.open_dropdown == Some(option.id),
                index as f32 * layout.step,
            )
        })
        .collect();
    let empty = if state.options.is_empty() {
        rsx! {
            fontstring { name: "CustomizationEmpty", width: OPTION_WIDTH, height: 38.0,
                text: "No customization options", font: GameFont::FrizQuadrata,
                font_size: 14.0, font_color: COLOR_WHITE,
            }
        }
    } else {
        Element::default()
    };
    let dropdown = state
        .open_dropdown
        .and_then(|id| {
            state
                .options
                .iter()
                .enumerate()
                .find(|(_, option)| option.id == id)
        })
        .map(|(index, option)| {
            dropdown_panel(
                option,
                [state.viewport_width, state.viewport_height],
                layout.top + index as f32 * layout.step + OPTION_HEIGHT,
            )
        })
        .unwrap_or_default();
    rsx! {
        {category_tabs(state)}
        r#frame { name: "CustomizePanel", width: OPTION_WIDTH, height: layout.height.max(OPTION_HEIGHT),
            pos_type: "absolute", right: OPTION_RIGHT, top: layout.top,
            {rows}
            {empty}
        }
        r#frame { name: "CustomizationPopupLayer", width: "fill", height: "fill",
            strata: FrameStrata::Dialog, pos_type: "absolute", left: 0.0, top: 0.0,
            {dropdown}
        }
    }
}

fn support_notice(state: &CharCreateUiState) -> Element {
    let hidden = state.support_notice.is_none();
    let text = state.support_notice.as_deref().unwrap_or("");
    rsx! {
        fontstring { name: "CustomizationSupportNotice", width: 560.0, height: 40.0,
            pos_type: "absolute", left: "50%", translate_x: "-50%", bottom: 28.0,
            text, font: GameFont::FrizQuadrata, font_size: 12.0, font_color: COLOR_GOLD, hidden,
        }
    }
}

pub fn char_create_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<CharCreateUiState>()
        .expect("CharCreateUiState must be in SharedContext");
    let content = match state.mode {
        CharCreateMode::RaceClass => rsx! { {race_grid(state)} {class_grid(state)} },
        CharCreateMode::Customize => {
            rsx! { {camera_controls()} {customize_panel(state)} {name_input_field(state)} {support_notice(state)} }
        }
    };
    rsx! {
        r#frame { name: CHAR_CREATE_ROOT, width: "fill", height: "fill", strata: FrameStrata::Background,
            {content}
            {body_type_buttons(state)}
            {bottom_buttons(state.mode)}
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
