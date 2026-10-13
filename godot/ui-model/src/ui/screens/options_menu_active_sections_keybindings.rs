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
    let buttons: Element = BindingSection::ALL.iter().flat_map(|section| {
        let hide = *section != active;
        rsx! { button { name: DynName(format!("KeybindingSection{}Button", section.key())), width: 175.0, height: 20.0,
            onclick: keybinding_section_action(*section), button_default_skin: false,
            texture { name: DynName(format!("KeybindingSection{}Selected",section.key())), texture_atlas: "Options_List_Active", width: 175.0, height: 20.0, hidden: hide, }
            fontstring { name: DynName(format!("KeybindingSection{}Label",section.key())), text: section.title(), width: 139.0, height: 20.0,
                font_size: 12.0, pos_type: "absolute", left: 36.0, top: -1.0, frame_level: 100.0, }
        } }
    }).collect();
    rsx! { r#frame { name: "KeybindingSectionTabs", width: 175.0, height: {BindingSection::ALL.len() as f32 * 20.0},
    layout: "flex-column", gap: 0.0, {buttons} } }
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
            font_size: 10.0,
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
    let hide_capture = !row.capturing;
    let (text, color) = match (&row.binding_text, row.capturing) {
        (_, true) => ("Press a key\u{2026}", BINDING_TEXT_COLOR),
        (Some(text), false) => (text.as_str(), BINDING_TEXT_COLOR),
        (None, false) => ("Not Bound", BINDING_NOT_BOUND_COLOR),
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
                font_size: 10.0,
                onclick: {&action},
                button_default_skin: false,
                {silver_button_art(row.action.key(), 383587, false)}
                {silver_button_art(&format!("{}Pressed",row.action.key()), 383585, true)}
                texture { name: DynName(format!("KeybindingCapture{}",row.action.key())), texture_fdid: 921208,
                    width: 160.0, height: 20.0, hidden: hide_capture, pos_type: "absolute", left: 0.0, top: 4.0, }
                texture { name: DynName(format!("KeybindingHover{}",row.action.key())), texture_fdid: 383586,
                    width: 160.0, height: 22.0, hidden: true, tex_coords: "0,1,0.03,0.7175", pos_type: "absolute", left: 0.0, top: 0.0, }
            }
            fontstring {
                name: {text_name},
                width: {BINDING_BUTTON_W - 10.0},
                height: BINDING_BUTTON_H,
                text: {text},
                font: "FrizQuadrata",
                font_size: 10.0,
                font_color: color,
                frame_level: 100.0,
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

fn silver_button_art(name: &str, fdid: u32, hidden: bool) -> Element {
    // UIMenuButtonStretchTemplate XML:12x6 corners; source cropped80x26 in128x32.
    let sx = [0.0, 0.09375, 0.53125, 0.625];
    let sy = [0.0, 0.1875, 0.625, 0.8125];
    let dx = [0.0, 12.0, BINDING_BUTTON_W - 12.0, BINDING_BUTTON_W];
    let dy = [0.0, 6.0, BINDING_BUTTON_H - 6.0, BINDING_BUTTON_H];
    let art: Element=(0..3).flat_map(|y| (0..3).flat_map(move |x| {
        rsx! { texture { name: DynName(format!("KeybindingSilver{name}{y}{x}")), texture_fdid: fdid,
            width: {dx[x+1]-dx[x]}, height: {dy[y+1]-dy[y]}, pos_type: "absolute", left: dx[x], top: dy[y],
            tex_coords: format!("{},{},{},{}",sx[x],sx[x+1],sy[y],sy[y+1]), } }
    })).collect();
    rsx! { r#frame { name: DynName(format!("KeybindingSilver{name}")), stretch: true, hidden, {art} } }
}
