//! Blizzard_FrameXML/Mainline/NavigationBar.xml and NavigationBar.lua.
//! Caller supplies measured text widths (not glyph counts) and owns overflow visibility.
use crate::bank_art::label;
use crate::menu_primitives::{ContextMenuItem, MenuElementKind, menu_element};
use crate::quest_art::DynName;
use ui_toolkit::{rsx, widget_def::Element};

pub const HEIGHT: f32 = 34.0;
const BUTTON_HEIGHT: f32 = 30.0;
const WIDTH_BUFFER: f32 = 20.0;
const HOME_OFFSET: f32 = -15.0;
const OVERFLOW_OFFSET: f32 = -18.0;
const OVERFLOW_WIDTH: f32 = 44.0;
const SHEET: u32 = 516763;
const TILE: u32 = 516764;
const GOLD: &str = "1.0,0.82,0.0,1.0";

#[derive(Clone, Copy)]
pub struct NavButton<'a> {
    pub name: &'a str,
    pub text: &'a str,
    pub action: &'a str,
    pub text_width: f32,
}

pub struct NavBar<'a> {
    pub name: &'a str,
    pub rect: (f32, f32, f32, f32),
    /// First element is the home button. Last element is the current, disabled breadcrumb.
    pub buttons: &'a [NavButton<'a>],
    pub overflow_action: &'a str,
    pub overflow_open: bool,
}

#[derive(Debug, PartialEq)]
pub struct NavLayout {
    pub widths: Vec<f32>,
    pub first_shown: usize,
    pub positions: Vec<f32>,
}

/// NavBar_CheckLength counts full widths right-to-left before applying authored overlap.
pub fn layout(width: f32, text_widths: &[f32]) -> NavLayout {
    let widths: Vec<_> = text_widths
        .iter()
        .enumerate()
        .map(|(i, w)| {
            if i == 0 {
                (w + 50.0).min(128.0)
            } else {
                w + 30.0
            }
        })
        .collect();
    let first_shown = first_visible_breadcrumb(width, &widths);
    let positions = breadcrumb_positions(&widths, first_shown);
    NavLayout {
        widths,
        first_shown,
        positions,
    }
}

fn first_visible_breadcrumb(width: f32, widths: &[f32]) -> usize {
    let max_width = width - WIDTH_BUFFER;
    let mut total = 0.0;
    let mut first_shown = widths.len();
    for (index, w) in widths.iter().enumerate().rev() {
        total += w;
        if total <= max_width {
            first_shown = index;
        }
    }
    if first_shown > 0 {
        let visible: f32 = widths[first_shown..].iter().sum();
        if visible + OVERFLOW_WIDTH > max_width {
            first_shown = (first_shown + 1).min(widths.len());
        }
    }
    first_shown
}

fn breadcrumb_positions(widths: &[f32], first_shown: usize) -> Vec<f32> {
    let start = if first_shown > 0 {
        OVERFLOW_WIDTH + OVERFLOW_OFFSET
    } else {
        0.0
    };
    let mut x = start;
    let positions = widths
        .iter()
        .enumerate()
        .map(|(index, w)| {
            let left = x;
            if index >= first_shown {
                x += w + if index == 0 { HOME_OFFSET } else { 0.0 };
            }
            left
        })
        .collect();
    positions
}

pub fn nav_bar(props: NavBar<'_>) -> Element {
    let (x, y, width, height) = props.rect;
    let text_widths: Vec<_> = props
        .buttons
        .iter()
        .map(|button| button.text_width)
        .collect();
    let layout = layout(width, &text_widths);
    let mut contents = crop(
        format!("{}Background", props.name),
        TILE,
        [0.0, 1.0, 0.1875, 0.25390625],
        (0.0, 0.0, width, height),
    );
    contents.extend(visible_breadcrumbs(&props, &layout, height));
    if layout.first_shown > 0 {
        contents.extend(overflow_button(&props, height));
    }
    // NavBarTemplate overlay is above all button regions.
    contents.extend(crop(
        format!("{}Overlay", props.name),
        TILE,
        [0.0, 1.0, 0.2578125, 0.32421875],
        (0.0, 0.0, width, height),
    ));
    if props.overflow_open {
        contents.extend(overflow_menu(&props, &layout));
    }
    rsx! { r#frame { name: {DynName(props.name.into())}, left:x, top:y, width, height, pos_type:"absolute", {contents} } }
}

fn visible_breadcrumbs(props: &NavBar<'_>, layout: &NavLayout, height: f32) -> Element {
    props
        .buttons
        .iter()
        .enumerate()
        .filter(|(i, _)| *i >= layout.first_shown)
        .flat_map(|(i, button)| {
            nav_button(
                *button,
                i == 0,
                i + 1 == props.buttons.len(),
                (
                    layout.positions[i],
                    (height - BUTTON_HEIGHT) / 2.0,
                    layout.widths[i],
                    BUTTON_HEIGHT,
                ),
            )
        })
        .collect()
}

fn nav_button(
    button: NavButton<'_>,
    home: bool,
    current: bool,
    rect: (f32, f32, f32, f32),
) -> Element {
    let (x, y, width, height) = rect;
    let inset = if home { 10.0 } else { 20.0 };
    let mut contents = nav_button_art(button.name, home, current, width, height);
    contents.extend(label(
        format!("{}Text", button.name),
        button.text,
        (inset, 9.0, width - inset - 10.0, 12.0),
        (12.0, GOLD, "LEFT"),
    ));
    let onclick = if current { "" } else { button.action };
    let mouse_enabled = !current;
    rsx! { button { name: {DynName(button.name.into())}, left:x, top:y, width, height,
    pos_type:"absolute", button_default_skin:false, disabled: current, onclick, mouse_enabled, {contents} } }
}

fn nav_button_art(name: &str, home: bool, current: bool, width: f32, height: f32) -> Element {
    let (fdid, normal, pushed, hover) = if home {
        let left = 0.703125 - (width / 128.0) * 0.25;
        (
            SHEET,
            [left, 0.703125, 0.0078125, 0.2421875],
            [left, 0.703125, 0.2578125, 0.4921875],
            [left, 0.713125, 0.5078125, 0.7421875],
        )
    } else {
        (
            TILE,
            [0.0, 1.0, 0.0625, 0.12109375],
            [0.0, 1.0, 0.125, 0.18359375],
            [0.00195313, 0.25195313, 0.65625, 0.921875],
        )
    };
    let mut contents = state_crops(name, fdid, normal, pushed, (0.0, 0.0, width, height));
    contents.extend(highlight_crop(
        name,
        SHEET,
        hover,
        (0.0, 0.0, width, height),
    ));
    if home {
        contents.extend(crate::bank_art::texture(
            format!("{name}Shadow"),
            423811,
            (0.0, 0.0, 30.0, 30.0),
            "1,1,1,1",
        ));
    } else {
        contents.extend(breadcrumb_arrow(name, width, height, current));
    }
    contents
}

fn breadcrumb_arrow(name: &str, width: f32, height: f32, current: bool) -> Element {
    let mut out = state_crops(
        &format!("{name}Arrow"),
        SHEET,
        [0.88867188, 0.9296875, 0.296875, 0.53125],
        [0.6328125, 0.67382813, 0.7578125, 0.9921875],
        (width, 0.0, 21.0, height),
    );
    if current {
        out.extend(crop(
            format!("{name}Selected"),
            SHEET,
            [0.00195313, 0.25195313, 0.375, 0.640625],
            (0.0, 0.0, width, height),
        ));
    }
    out
}

fn overflow_button(props: &NavBar<'_>, height: f32) -> Element {
    let name = format!("{}OverflowButton", props.name);
    let mut contents = state_crops(
        &name,
        SHEET,
        [0.54296875, 0.62890625, 0.7578125, 0.9921875],
        [0.453125, 0.5390625, 0.7578125, 0.9921875],
        (0.0, 0.0, OVERFLOW_WIDTH, BUTTON_HEIGHT),
    );
    contents.extend(highlight_crop(
        &name,
        SHEET,
        [0.54296875, 0.62890625, 0.7578125, 0.9921875],
        (0.0, 0.0, OVERFLOW_WIDTH, BUTTON_HEIGHT),
    ));
    rsx! { button { name:{DynName(name)}, left:0.0, top:{(height-BUTTON_HEIGHT)/2.0}, width:OVERFLOW_WIDTH,
    height:BUTTON_HEIGHT, pos_type:"absolute", button_default_skin:false, onclick:props.overflow_action, {contents} } }
}

fn overflow_menu(props: &NavBar<'_>, layout: &NavLayout) -> Element {
    let width = props.buttons[..layout.first_shown]
        .iter()
        .map(|button| button.text_width)
        .fold(0.0_f32, f32::max)
        + 36.0;
    let height = 23.0 + 20.0 * layout.first_shown as f32;
    let mut contents = crate::selection_list::menu_background(
        format!("{}OverflowMenuBackground", props.name),
        (width, height),
    );
    contents.extend(
        props.buttons[..layout.first_shown]
            .iter()
            .enumerate()
            .flat_map(|(i, button)| {
                menu_element(
                    ContextMenuItem {
                        name: button.name,
                        label: button.text,
                        action: button.action,
                    },
                    MenuElementKind::Button,
                    true,
                    (8.0, 8.0 + 20.0 * i as f32, width - 16.0, 20.0),
                )
            }),
    );
    // DropdownButton.xml: TOPLEFT on the centered overflow button's BOTTOMLEFT.
    let top = (props.rect.3 + BUTTON_HEIGHT) / 2.0;
    rsx! { r#frame { name:{DynName(format!("{}OverflowMenu",props.name))}, width,height,left:0.0,top,
    pos_type:"absolute",strata:"DIALOG",frame_level:60.0,{contents} } }
}

fn state_crops(
    name: &str,
    fdid: u32,
    normal: [f32; 4],
    pushed: [f32; 4],
    rect: (f32, f32, f32, f32),
) -> Element {
    let mut out = crop(format!("{name}NormalArt"), fdid, normal, rect);
    let (x, y, width, height) = rect;
    let coords = format!("{},{},{},{}", pushed[0], pushed[1], pushed[2], pushed[3]);
    out.extend(
        rsx! {texture {name:{DynName(format!("{name}PushedArt"))},width,height,left:x,top:y,
        pos_type:"absolute",texture_fdid:fdid,tex_coords:coords.as_str(),hidden:true}},
    );
    out
}

fn highlight_crop(name: &str, fdid: u32, coords: [f32; 4], rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, width, height) = rect;
    let coords = format!("{},{},{},{}", coords[0], coords[1], coords[2], coords[3]);
    rsx! {texture {name:{DynName(format!("{name}Highlight"))},width,height,left:x,top:y,pos_type:"absolute",
    texture_fdid:fdid,tex_coords:coords.as_str(),draw_layer:"HIGHLIGHT",blend_mode:"ADD"}}
}

fn crop(name: String, fdid: u32, coords: [f32; 4], rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, width, height) = rect;
    let coords = format!("{},{},{},{}", coords[0], coords[1], coords[2], coords[3]);
    rsx! {texture {name:{DynName(name)},width,height,left:x,top:y,pos_type:"absolute",texture_fdid:fdid,tex_coords:coords.as_str()}}
}
