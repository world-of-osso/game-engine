//! Retail RecipeList.xml34–45 and Professions.lua1256–1263 learned-state filters.
use super::ProfessionView;
use game_engine_ui_model::menu_primitives::{
    ContextMenuItem, DropdownButton, DropdownState, DropdownStyle, MenuElementKind,
    context_menu_description, dropdown_button_with_state,
};
use ui_toolkit::widget_def::Element;

pub(super) fn dropdown(view: &ProfessionView, (left, top, width): (f32, f32, f32)) -> Element {
    let mut out = dropdown_button_with_state(
        DropdownButton {
            frame_name: "ProfessionsRecipeFilterDropdown",
            label_name: "ProfessionsRecipeFilterText",
            arrow_name: "ProfessionsRecipeFilterArrow",
            text: "Filter",
            width,
            height: 18.0,
            x: left,
            y: -top,
            background_color: "",
            text_color: game_engine_ui_model::quest_art::HIGHLIGHT_FONT_COLOR,
            arrow_color: "",
            onclick: Some("profession:filter:menu"),
        },
        DropdownState {
            style: DropdownStyle::Filter,
            open: view.filter_open,
            reset_action: (view.book.hide_learned || view.book.hide_unlearned)
                .then_some("profession:filter:reset"),
            ..Default::default()
        },
    );
    if view.filter_open {
        // MenuTemplates.xml66–75: BOTTOMLEFT, menuPointX6, menuPointY2.
        out.extend(context_menu_description(
            "ProfessionsRecipeFilterMenu",
            (left + 6.0, top + 18.0 - 2.0),
            0.0,
            &[
                (
                    ContextMenuItem {
                        name: "ProfessionsFilterLearned",
                        label: "Show Learned",
                        action: "profession:filter:learned",
                    },
                    MenuElementKind::Checkbox {
                        checked: !view.book.hide_learned,
                    },
                    true,
                ),
                (
                    ContextMenuItem {
                        name: "ProfessionsFilterUnlearned",
                        label: "Show Unlearned",
                        action: "profession:filter:unlearned",
                    },
                    MenuElementKind::Checkbox {
                        checked: !view.book.hide_unlearned,
                    },
                    true,
                ),
            ],
        ));
    }
    out
}
