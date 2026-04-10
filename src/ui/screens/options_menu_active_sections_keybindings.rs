use super::*;

pub(super) fn keybindings_body(bindings: &KeybindingsView) -> Element {
    content_stack(
        [
            keybinding_section_tabs(bindings.section),
            spacer("KeybindingsSpacer", 6.0),
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
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                y: {visuals.text_y},
            }
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
            {keybinding_value(row)}
            {keybinding_clear_button(row)}
            {keybinding_rebind_button(row)}
        }
    }
}

fn keybinding_value(row: &KeybindingRowView) -> Element {
    let text = if row.capturing {
        "Press a key or mouse button...".to_string()
    } else {
        row.binding_text.clone()
    };
    rsx! {
        fontstring {
            name: {DynName(format!("KeybindingValue{}", row.action.key()))},
            width: {BINDING_VALUE_W},
            height: 20.0,
            text: {&text},
            font_size: 14.0,
            color: "0.95,0.90,0.74,1.0",
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x: "-176",
            }
        }
    }
}

fn keybinding_clear_button(row: &KeybindingRowView) -> Element {
    let clear_name = DynName(format!("KeybindingClear{}", row.action.key()));
    if row.can_clear {
        let action = keybinding_clear_action(row.action);
        rsx! {
            button {
                name: {clear_name},
                width: 72.0,
                height: 28.0,
                text: "Clear",
                font_size: 13.0,
                onclick: {&action},
                button_atlas_up: "defaultbutton-nineslice-up",
                button_atlas_pressed: "defaultbutton-nineslice-pressed",
                button_atlas_highlight: "defaultbutton-nineslice-highlight",
                button_atlas_disabled: "defaultbutton-nineslice-disabled",
                anchor { point: AnchorPoint::Right, relative_point: AnchorPoint::Right, x: "-84" }
            }
        }
    } else {
        rsx! {
            button {
                name: {clear_name},
                width: 72.0,
                height: 28.0,
                text: "Clear",
                font_size: 13.0,
                disabled: true,
                button_atlas_up: "defaultbutton-nineslice-up",
                button_atlas_pressed: "defaultbutton-nineslice-pressed",
                button_atlas_highlight: "defaultbutton-nineslice-highlight",
                button_atlas_disabled: "defaultbutton-nineslice-disabled",
                anchor { point: AnchorPoint::Right, relative_point: AnchorPoint::Right, x: "-84" }
            }
        }
    }
}

fn keybinding_rebind_button(row: &KeybindingRowView) -> Element {
    let action = keybinding_rebind_action(row.action);
    rsx! {
        button {
            name: {DynName(format!("KeybindingRebind{}", row.action.key()))},
            width: 72.0,
            height: 28.0,
            text: "Rebind",
            font_size: 13.0,
            onclick: {&action},
            button_atlas_up: "defaultbutton-nineslice-up",
            button_atlas_pressed: "defaultbutton-nineslice-pressed",
            button_atlas_highlight: "defaultbutton-nineslice-highlight",
            button_atlas_disabled: "defaultbutton-nineslice-disabled",
            anchor { point: AnchorPoint::Right, relative_point: AnchorPoint::Right }
        }
    }
}
