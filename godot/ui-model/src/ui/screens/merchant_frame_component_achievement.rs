//! AchievementFrameTabButtonTemplate (Mainline AchievementUI.xml246–342).
//! Distinct Header-sheet art; the shared builder's Panel/Minimal variants stay unchanged.
use super::{DynName, Element, GameFont, HIGHLIGHT_FONT_COLOR, NORMAL_FONT_COLOR, Text, text};
use ui_toolkit::rsx;

pub(super) fn tab(
    name: &str,
    label: &str,
    (x, y, width): (f32, f32, f32),
    selected: bool,
    action: &str,
) -> Element {
    let mut children = strips(name, width, selected);
    // AchievementFrame_UpdateTabs: CENTER y−5 selected /−3 inactive.
    let text_y = if selected { 16.0 } else { 14.0 };
    children.extend(text(Text {
        name: format!("{name}Text"),
        text: label,
        rect: (25.0, text_y, width - 50.0, 10.0),
        font: GameFont::FrizQuadrata,
        size: 10.0,
        color: if selected {
            HIGHLIGHT_FONT_COLOR
        } else {
            NORMAL_FONT_COLOR
        },
        justify: "CENTER",
    }));
    let action = if selected { "" } else { action };
    let mouse_enabled = !selected;
    rsx! {button {name:{DynName(name.into())},width,height:32.0,
    left:x,top:y,pos_type:"absolute",onclick:action,disabled:selected,
    mouse_enabled,button_default_skin:false,{children}}}
}

fn strips(name: &str, width: f32, selected: bool) -> Element {
    let height = if selected { 59.0 } else { 49.0 };
    let color = if selected { "1,1,1,1" } else { "0.6,0.6,0.6,1" };
    [
        ("Left", "0.47265625,0.513671875,0.76953125,1", -4.0, 21.0),
        (
            "Middle",
            "0.513671875,0.685546875,0.76953125,1",
            17.0,
            width - 31.0,
        ),
        (
            "Right",
            "0.685546875,0.720703125,0.76953125,1",
            width - 14.0,
            18.0,
        ),
    ]
    .into_iter()
    .flat_map(|(part, uv, left, part_width)| {
        rsx! {texture {name:{DynName(format!("{name}{part}"))},texture_fdid:235402,
        tex_coords:uv,vertex_color:color,left,top:0.0,width:part_width,height,pos_type:"absolute"}}
    })
    .collect()
}
