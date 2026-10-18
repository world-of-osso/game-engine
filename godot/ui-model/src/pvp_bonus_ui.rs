//! Retail PVPUI.xml1167–1232/645–695; Lua620–645/691–710/1130–1156.
//! Queue exposure only: no invented rewards, availability or level requirement.
use super::PvpView;
use crate::bank_art::label;
use crate::menu_primitives::{
    ContextMenu, ContextMenuItem, DropdownButton, DropdownState, MenuElementKind,
    context_menu_with_rows, dropdown_button_with_state, menu_element,
};
use crate::quest_art::named_atlas_texture;
use ui_toolkit::{
    rsx,
    widget_def::{Attr, Element, WidgetChild},
};

// HonorFrame(220,22), Inset(4,85)..(-5,-22), dropdown BOTTOMRIGHT(-6,+4), width180.
const TYPE_RECT: [f32; 4] = [431.0, 78.0, 180.0, 25.0];
const BONUS_RECT: [f32; 4] = [224.0, 107.0, 393.0, 296.0];
// First source slot TOP-9; Lua retains five-slot height (296-26)/5 even in this slice.
const RANDOM_RECT: [f32; 4] = [14.0, 9.0, 365.0, 54.0];
const TYPE_ITEMS: &[ContextMenuItem<'_>] = &[
    ContextMenuItem {
        name: "HonorFrameTypeMenuBonus",
        label: "Bonus Battlegrounds",
        action: "pvp:type:bonus",
    },
    ContextMenuItem {
        name: "HonorFrameTypeMenuSpecific",
        label: "Specific Battlegrounds",
        action: "pvp:type:specific",
    },
];

pub(super) fn type_selector(view: &PvpView) -> Element {
    let mut elements = render_type_button(view);
    if view.honor_type_menu_open {
        elements.extend(type_menu(view));
    }
    elements
}

fn render_type_button(view: &PvpView) -> Element {
    let text = if view.honor_bonus {
        "Bonus Battlegrounds"
    } else {
        "Specific Battlegrounds"
    };
    dropdown_button_with_state(
        build_type_button_props(text),
        DropdownState {
            resize_to_text: false,
            open: view.honor_type_menu_open,
            enabled: !view.pending && view.battleground().is_none(),
            ..Default::default()
        },
    )
}

fn build_type_button_props(text: &str) -> DropdownButton<'_> {
    let [left, top, width, height] = TYPE_RECT;
    DropdownButton {
        frame_name: "HonorFrameTypeDropdown",
        label_name: "HonorFrameTypeDropdownText",
        arrow_name: "HonorFrameTypeDropdownArrow",
        text,
        width,
        height,
        x: left,
        y: -top,
        background_color: "",
        text_color: "1,1,1,1",
        arrow_color: "",
        onclick: Some("pvp:type:toggle"),
    }
}

fn type_menu(view: &PvpView) -> Element {
    let selected = if view.honor_bonus {
        "pvp:type:bonus"
    } else {
        "pvp:type:specific"
    };
    context_menu_with_rows(
        ContextMenu {
            frame_name: "HonorFrameTypeMenu",
            title_name: "HonorFrameTypeMenuTitle",
            divider_name: "HonorFrameTypeMenuDivider",
            hidden: false,
            title: "",
            width: TYPE_RECT[2],
            x: TYPE_RECT[0],
            y: TYPE_RECT[1] + TYPE_RECT[3],
            items: TYPE_ITEMS,
        },
        render_type_rows(selected),
    )
}

fn render_type_rows(selected: &str) -> Element {
    TYPE_ITEMS
        .iter()
        .enumerate()
        .flat_map(|(index, item)| {
            menu_element(
                *item,
                MenuElementKind::Radio {
                    checked: item.action == selected,
                },
                true,
                (8.0, 8.0 + index as f32 * 20.0, TYPE_RECT[2] - 16.0, 20.0),
            )
        })
        .collect()
}

pub(super) fn bonus_panel(view: &PvpView) -> Element {
    let mut children = Vec::new();
    if let Some(faction) = view.honor_faction {
        let atlas = match faction {
            crate::char_create_data::Faction::Alliance => "pvpqueue-background-casual-alliance",
            crate::char_create_data::Faction::Horde => "pvpqueue-background-casual-horde",
        };
        let region = ui_toolkit::atlas::resolve_region(atlas, ui_toolkit::atlas::thread_skin())
            .expect("Retail casual faction background atlas");
        children.extend(named_atlas_texture(
            "HonorFrameBonusFrameWorldBattlesTexture".into(),
            atlas,
            (
                (BONUS_RECT[2] - region.width) / 2.0,
                3.0,
                region.width,
                region.height,
            ),
        ));
    }
    children.extend(random_button(view));
    let [left, top, width, height] = BONUS_RECT;
    rsx! { r#frame { name: "HonorFrameBonusFrame", left, top, width, height,
    pos_type: "absolute", mouse_enabled: true, {children} } }
}

fn random_button(view: &PvpView) -> Element {
    let title = shared::battleground::queue_name(shared::battleground::RANDOM_BATTLEGROUND_ID)
        .expect("Random queue identity is supported");
    let contents = label(
        "HonorFrameBonusFrameRandomBGButtonTitle".into(),
        &title,
        (20.0, 20.0, 325.0, 16.0),
        (16.0, "1,1,1,1", "LEFT"),
    );
    let [left, top, width, height] = RANDOM_RECT;
    let mut row = crate::selection_list::row(
        "HonorFrameBonusFrameRandomBGButton".into(),
        "pvp:bonus:random",
        !view.pending && view.battleground().is_none(),
        (left, top, width, height),
        (&crate::selection_list::PVP_CASUAL_ACTIVITY, true),
        contents,
    );
    apply_random_button_art(&mut row);
    row
}

fn apply_random_button_art(row: &mut Element) {
    let WidgetChild::Widget(button) = &mut row[0] else {
        unreachable!("Random activity is a button")
    };
    for (key, atlas) in [
        ("button_atlas_up", "pvpqueue-button-casual-up"),
        ("button_atlas_pressed", "pvpqueue-button-casual-down"),
        ("button_atlas_disabled", "pvpqueue-button-casual-up"),
    ] {
        button.attrs.push(Attr::new_dynamic(key, atlas.into()));
    }
}
