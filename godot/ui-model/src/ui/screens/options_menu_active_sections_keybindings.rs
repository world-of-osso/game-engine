use super::*;

pub(super) fn keybindings_body(bindings: &KeybindingsView) -> OptionsPage {
    content_stack(
        [
            keybinding_section_tabs(bindings.section),
            keybinding_output(bindings),
            keybinding_rows(bindings),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

fn keybinding_section_tabs(active: BindingSection) -> Element {
    let buttons: Element = BindingSection::ALL
        .iter()
        .flat_map(|section| keybinding_section_button(*section, *section == active))
        .collect();
    rsx! {
        r#frame {
            name: "KeybindingSectionTabs",
            width: {OPTIONS_ROW_W},
            height: KEYBINDING_TAB_H,
            layout: "flex-row",
            justify: "start",
            align: "center",
            gap: KEYBINDING_TAB_GAP,
            r#frame {
                name: "KeybindingSectionTabsLeadSpacer",
                width: KEYBINDING_TAB_LEAD_X,
                height: KEYBINDING_TAB_H,
            }
            {buttons}
        }
    }
}

fn keybinding_section_button(section: BindingSection, active: bool) -> Element {
    let label = section.title().to_string();
    let action = keybinding_section_action(section);
    let names = keybinding_section_tab_names(section);
    let visuals = keybinding_section_tab_visuals(active);
    let width = keybinding_section_tab_width(&label);
    let frame_name = names.frame.clone();
    rsx! {
        r#frame {
            name: {frame_name},
            width: {width},
            height: KEYBINDING_TAB_H,
            {keybinding_section_tab_button(&names, &visuals, &action)}
            {keybinding_section_tab_label(&names, &visuals, &label, width)}
        }
    }
}

struct KeybindingSectionTabNames {
    frame: DynName,
    button: DynName,
    label: DynName,
}

fn keybinding_section_tab_names(section: BindingSection) -> KeybindingSectionTabNames {
    KeybindingSectionTabNames {
        frame: DynName(format!("KeybindingSection{}", section.key())),
        button: DynName(format!("KeybindingSection{}Button", section.key())),
        label: DynName(format!("KeybindingSection{}Label", section.key())),
    }
}

struct KeybindingSectionTabVisuals {
    text_y: f32,
    atlas_up: &'static str,
    atlas_pressed: &'static str,
    atlas_highlight: &'static str,
    text_color: &'static str,
}

fn keybinding_section_tab_visuals(active: bool) -> KeybindingSectionTabVisuals {
    KeybindingSectionTabVisuals {
        text_y: if active {
            KEYBINDING_TAB_LABEL_Y_ACTIVE
        } else {
            KEYBINDING_TAB_LABEL_Y_IDLE
        },
        atlas_up: if active {
            "defaultbutton-nineslice-pressed"
        } else {
            "defaultbutton-nineslice-up"
        },
        atlas_pressed: "defaultbutton-nineslice-pressed",
        atlas_highlight: if active {
            "defaultbutton-nineslice-pressed"
        } else {
            "defaultbutton-nineslice-highlight"
        },
        text_color: if active {
            KEYBINDING_TAB_TEXT_ACTIVE
        } else {
            KEYBINDING_TAB_TEXT_IDLE
        },
    }
}

fn keybinding_section_tab_width(label: &str) -> f32 {
    let (text_width, _) =
        measure_text(label, GameFont::FrizQuadrata, KEYBINDING_TAB_FONT_SIZE).unwrap_or((0.0, 0.0));
    (text_width + KEYBINDING_TAB_SIDE_PADDING).ceil().max(10.0)
}

fn keybinding_section_tab_button(
    names: &KeybindingSectionTabNames,
    visuals: &KeybindingSectionTabVisuals,
    action: &str,
) -> Element {
    let button_name = names.button.clone();
    rsx! {
        button {
            name: {button_name},
            stretch: true,
            text: "",
            font_size: KEYBINDING_TAB_FONT_SIZE,
            onclick: {action},
            button_atlas_up: visuals.atlas_up,
            button_atlas_pressed: visuals.atlas_pressed,
            button_atlas_highlight: visuals.atlas_highlight,
            button_atlas_disabled: "defaultbutton-nineslice-disabled",
        }
    }
}

fn keybinding_section_tab_label(
    names: &KeybindingSectionTabNames,
    visuals: &KeybindingSectionTabVisuals,
    label: &str,
    width: f32,
) -> Element {
    let label_name = names.label.clone();
    rsx! {
        fontstring {
            name: {label_name},
            width: {width},
            height: KEYBINDING_TAB_H,
            text: {label},
            font: "FrizQuadrata",
            font_size: KEYBINDING_TAB_FONT_SIZE,
            font_color: visuals.text_color,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: "50%",
            margin_top: {-(visuals.text_y)},
            translate_y: "-50%",
        }
    }
}

fn keybinding_output(bindings: &KeybindingsView) -> Element {
    let (text, color) = match &bindings.output {
        Some(output) if output.error => (output.text.as_str(), BINDING_OUTPUT_ERROR_COLOR),
        Some(output) => (output.text.as_str(), BINDING_OUTPUT_COLOR),
        None => ("", BINDING_OUTPUT_COLOR),
    };
    rsx! {
        fontstring {
            name: "KeybindingOutput",
            width: {OPTIONS_ROW_W},
            height: BINDING_OUTPUT_H,
            text: {text},
            font: "FrizQuadrata",
            font_size: 13.0,
            font_color: color,
            justify_h: "CENTER",
        }
    }
}

fn keybinding_rows(bindings: &KeybindingsView) -> Element {
    bindings.rows.iter().flat_map(keybinding_row).collect()
}

fn keybinding_row(row: &KeybindingRowView) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("KeybindingRow{}", row.action.key()))},
            width: {OPTIONS_ROW_W},
            height: 34.0,
            {row_label(&format!("KeybindingLabel{}", row.action.key()), &row.label)}
            {keybinding_button(row)}
        }
    }
}

/// The one binding button: its key, gray `NOT_BOUND` "Not Bound" at 0.8 alpha when unbound
/// (`BindingButtonTemplate_SetupBindingButton`, `Blizzard_SharedXML/BindingUtil.lua:217-232`),
/// pressed while listening. One key per action is the user's design choice; Retail shows a
/// primary and a secondary button (`Blizzard_Keybindings.xml:65-82`). Retail's button sits
/// 80 left of the row centre, 160 wide (:69-72).
fn keybinding_button(row: &KeybindingRowView) -> Element {
    let button_name = DynName(keybinding_button_name(row.action));
    let text_name = DynName(format!("KeybindingButtonText{}", row.action.key()));
    let action = keybinding_rebind_action(row.action);
    let (text, color) = match (&row.binding_text, row.capturing) {
        (_, true) => ("Press a key\u{2026}", BINDING_TEXT_COLOR),
        (Some(text), false) => (text.as_str(), BINDING_TEXT_COLOR),
        (None, false) => ("Not Bound", BINDING_NOT_BOUND_COLOR),
    };
    let atlas_up = if row.capturing {
        "defaultbutton-nineslice-pressed"
    } else {
        "defaultbutton-nineslice-up"
    };
    rsx! {
        r#frame {
            name: {DynName(format!("KeybindingButtonFrame{}", row.action.key()))},
            width: BINDING_BUTTON_W,
            height: BINDING_BUTTON_H,
            pos_type: "absolute",
            left: {OPTIONS_ROW_W / 2.0 - 80.0},
            top: "50%",
            translate_y: "-50%",
            button {
                name: {button_name},
                stretch: true,
                text: "",
                font_size: 13.0,
                onclick: {&action},
                button_atlas_up: atlas_up,
                button_atlas_pressed: "defaultbutton-nineslice-pressed",
                button_atlas_highlight: "defaultbutton-nineslice-highlight",
                button_atlas_disabled: "defaultbutton-nineslice-disabled",
            }
            fontstring {
                name: {text_name},
                width: {BINDING_BUTTON_W - 10.0},
                height: BINDING_BUTTON_H,
                text: {text},
                font: "FrizQuadrata",
                font_size: 13.0,
                font_color: color,
                justify_h: "CENTER",
                pos_type: "absolute",
                left: "50%",
                translate_x: "-50%",
                top: "50%",
                translate_y: "-50%",
            }
        }
    }
}
