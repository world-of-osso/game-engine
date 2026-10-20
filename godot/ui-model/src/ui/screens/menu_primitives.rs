//! Retail Blizzard_Menu templates. Positions retain the original public API:
//! dropdown y is y-up; context-menu y is y-down. Skin selection belongs to atlas resolution.
use crate::bank_art::{label, texture};
use crate::quest_art::named_atlas_texture;
use crate::ui::strata::FrameStrata;
use std::fmt;
use ui_toolkit::{rsx, widget_def::Element, widgets::font_string::GameFont};

const ROW_HEIGHT: f32 = 20.0;
const MENU_INSET: f32 = 8.0;
const MENU_BOTTOM: f32 = 15.0;
const DIVIDER_HEIGHT: f32 = 13.0;
const SPACER_HEIGHT: f32 = 10.0;
const NORMAL: &str = "1.0,1.0,1.0,1.0";
const GOLD: &str = "1.0,0.82,0.0,1.0";
const DISABLED: &str = "0.5,0.5,0.5,1.0";
const MENU_HIGHLIGHT: u32 = 136810;
const MENU_DIVIDER: u32 = 918860;
const SUBMENU_ARROW: u32 = 130940;

struct DynName(String);
impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy)]
pub struct DropdownButton<'a> {
    pub frame_name: &'a str,
    pub label_name: &'a str,
    pub arrow_name: &'a str,
    pub text: &'a str,
    pub width: f32,
    pub height: f32,
    pub x: f32,
    pub y: f32,
    /// Retained for source compatibility; Retail template art replaces flat fills.
    pub background_color: &'a str,
    pub text_color: &'a str,
    /// Retained for source compatibility; the arrow is now atlas art.
    pub arrow_color: &'a str,
    pub onclick: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DropdownStyle {
    #[default]
    WowStyle1,
    Filter,
    WowStyle2,
}

#[derive(Clone, Copy)]
pub struct DropdownState<'a> {
    pub style: DropdownStyle,
    /// Retail resizeToText; false preserves an explicit XML/Lua width.
    pub resize_to_text: bool,
    pub open: bool,
    pub enabled: bool,
    /// UIResetButtonTemplate: hidden unless filters differ from defaults.
    pub reset_action: Option<&'a str>,
}
impl Default for DropdownState<'_> {
    fn default() -> Self {
        Self {
            style: DropdownStyle::WowStyle1,
            resize_to_text: true,
            open: false,
            enabled: true,
            reset_action: None,
        }
    }
}

pub fn dropdown_button(props: DropdownButton<'_>) -> Element {
    dropdown_button_with_state(props, DropdownState::default())
}

/// Mainline/MenuTemplates.xml:3,66 and MenuTemplates.xml:4. Callers own menu visibility.
pub fn dropdown_button_with_state(props: DropdownButton<'_>, state: DropdownState<'_>) -> Element {
    let props = resize_filter_to_text(props, state);
    let mut contents = dropdown_art(props, state);
    contents.extend(dropdown_text(props, state));
    contents.extend(reset_button(props, state));
    let name = DynName(props.frame_name.into());
    let disabled = !state.enabled;
    let onclick = if state.enabled {
        props.onclick.unwrap_or("")
    } else {
        ""
    };
    rsx! { button {
        name, width: {props.width}, height: {props.height}, button_default_skin: false,
        onclick, disabled, mouse_enabled: state.enabled, pos_type: "absolute", left: {props.x}, top: {-props.y},
        {contents}
    } }
}

/// DropdownTextMixin:UpdateText and WowStyle1FilterDropdownTemplate resizeToTextPadding60.
fn resize_filter_to_text<'a>(
    props: DropdownButton<'a>,
    state: DropdownState<'_>,
) -> DropdownButton<'a> {
    if state.style != DropdownStyle::Filter || !state.resize_to_text {
        return props;
    }
    DropdownButton {
        width: filter_dropdown_width(props.text),
        ..props
    }
}

/// The authored content width, also used by right-anchored filter owners.
pub fn filter_dropdown_width(text: &str) -> f32 {
    ui_toolkit::text_measure::measure_text(text, GameFont::FrizQuadrata, 12.0)
        .expect("Filter template font loaded")
        .0
        + 60.0
}

fn dropdown_art(props: DropdownButton<'_>, state: DropdownState<'_>) -> Element {
    let name = props.frame_name;
    match state.style {
        DropdownStyle::WowStyle1 => {
            let mut out = named_atlas_texture(
                format!("{name}Background"),
                "common-dropdown-textholder",
                (-8.0, -7.0, props.width + 16.0, props.height + 16.0),
            );
            out.extend(dropdown_arrow(props, state));
            out
        }
        DropdownStyle::Filter => dropdown_surface(
            props,
            state,
            "common-dropdown-b-button",
            [-4.0, -4.0, 8.0, 8.0],
        ),
        DropdownStyle::WowStyle2 => {
            let mut out = dropdown_surface(
                props,
                state,
                "common-dropdown-c-button",
                [-7.0, -7.0, 14.0, 14.0],
            );
            out.extend(rsx! { texture {
                name: {DynName(props.arrow_name.into())}, texture_atlas: "common-dropdown-c-button-hover-arrow",
                width: 12.0, height: 5.0, left: {(props.width - 12.0) / 2.0}, top: {props.height},
                pos_type: "absolute", draw_layer: "HIGHLIGHT",
            } });
            out
        }
    }
}

fn dropdown_surface(
    props: DropdownButton<'_>,
    state: DropdownState<'_>,
    atlas: &str,
    [x, y, w, h]: [f32; 4],
) -> Element {
    let suffix = if !state.enabled {
        "-disabled"
    } else if state.open {
        "-open"
    } else {
        ""
    };
    let up = format!("{atlas}{suffix}");
    let pressed_suffix = if state.style == DropdownStyle::WowStyle2 {
        "-pressed-1"
    } else {
        "-pressed"
    };
    let hover_suffix = if state.style == DropdownStyle::WowStyle2 {
        "-hover-1"
    } else {
        "-hover"
    };
    let pressed = format!("{atlas}{pressed_suffix}");
    let hover = format!("{atlas}{hover_suffix}");
    let disabled = format!("{atlas}-disabled");
    rsx! { button {
        name: {DynName(format!("{}Background", props.frame_name))},
        width: {props.width + w}, height: {props.height + h}, left: x, top: y, pos_type: "absolute",
        mouse_enabled: false, button_default_skin: false,
        button_atlas_up: up.as_str(), button_atlas_pressed: pressed.as_str(),
        button_atlas_highlight: hover.as_str(), button_highlight_alpha: 1.0,
        button_atlas_disabled: disabled.as_str(),
    } }
}

fn dropdown_arrow(props: DropdownButton<'_>, state: DropdownState<'_>) -> Element {
    let up = if state.open {
        "common-dropdown-a-button-open"
    } else {
        "common-dropdown-a-button"
    };
    let up = if state.enabled {
        up
    } else {
        "common-dropdown-a-button-disabled"
    };
    // Arrow RIGHT (+1,-3), atlas 27x27; input belongs to the whole dropdown.
    rsx! { button {
        name: {DynName(props.arrow_name.into())}, width: 27.0, height: 27.0,
        pos_type: "absolute", left: {props.width - 26.0}, top: {(props.height - 27.0) / 2.0 + 3.0},
        button_default_skin: false, mouse_enabled: false,
        button_atlas_up: up, button_atlas_pressed: "common-dropdown-a-button-pressed",
        button_atlas_highlight: "common-dropdown-a-button-hover", button_highlight_alpha: 1.0,
        button_atlas_disabled: "common-dropdown-a-button-disabled",
    } }
}

fn dropdown_text(props: DropdownButton<'_>, state: DropdownState<'_>) -> Element {
    let color = if !state.enabled {
        DISABLED
    } else if state.style == DropdownStyle::WowStyle1 {
        NORMAL
    } else {
        GOLD
    };
    let rect = calculate_dropdown_text_rect(props, state.style);
    let justify = if state.style == DropdownStyle::WowStyle1 {
        "LEFT"
    } else {
        "CENTER"
    };
    rsx! { fontstring {
        name: {DynName(props.label_name.into())},
        width: {rect.2}, height: {rect.3}, text: {props.text},
        font: GameFont::FrizQuadrata, font_size: 12.0, font_color: color,
        shadow_color: "0.0,0.0,0.0,1.0", shadow_offset: "1,-1",
        justify_h: justify, word_wrap: false, max_lines: 1,
        pos_type: "absolute", left: {rect.0}, top: {rect.1},
    } }
}

fn calculate_dropdown_text_rect(
    props: DropdownButton<'_>,
    style: DropdownStyle,
) -> (f32, f32, f32, f32) {
    match style {
        // Retail's 10px FontString becomes a 15px-minimum native Label. Center
        // that label in the button, retaining the source horizontal text/arrow anchors.
        DropdownStyle::WowStyle1 => (8.0, 0.0, props.width - 34.0, props.height),
        DropdownStyle::Filter => (0.0, 0.0, props.width, 20.0),
        DropdownStyle::WowStyle2 => (13.0, (props.height - 20.0) / 2.0, props.width - 26.0, 20.0),
    }
}

fn reset_button(props: DropdownButton<'_>, state: DropdownState<'_>) -> Element {
    if state.style == DropdownStyle::WowStyle1 {
        return Vec::new();
    }
    let hidden = state.reset_action.is_none();
    let onclick = state.reset_action.unwrap_or("");
    let y = if state.style == DropdownStyle::WowStyle2 {
        5.0
    } else {
        0.0
    };
    rsx! { button {
        name: {DynName(format!("{}ResetButton", props.frame_name))}, width: 23.0, height: 23.0,
        pos_type: "absolute", left: {props.width - 3.0 - 11.5}, top: {y - 11.5}, hidden, onclick,
        button_default_skin: false, button_atlas_up: "auctionhouse-ui-filter-redx",
        button_atlas_highlight: "auctionhouse-ui-filter-redx", button_highlight_alpha: "0.4",
    } }
}

#[derive(Clone, Copy)]
pub struct ContextMenuItem<'a> {
    pub name: &'a str,
    pub label: &'a str,
    pub action: &'a str,
}
#[derive(Clone, Copy)]
pub struct ContextMenu<'a> {
    pub frame_name: &'a str,
    pub title_name: &'a str,
    pub divider_name: &'a str,
    pub hidden: bool,
    pub title: &'a str,
    pub width: f32,
    pub x: f32,
    pub y: f32,
    pub items: &'a [ContextMenuItem<'a>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuElementKind {
    Button,
    Checkbox { checked: bool },
    Radio { checked: bool },
    Title,
    Divider,
    Spacer,
    Submenu,
}

/// MenuVariants.lua and Mainline/MenuVariants.lua: real menu elements, not push-button skins.
pub fn menu_element(
    item: ContextMenuItem<'_>,
    kind: MenuElementKind,
    enabled: bool,
    rect: (f32, f32, f32, f32),
) -> Element {
    let (x, y, width, height) = rect;
    if kind == MenuElementKind::Spacer {
        return rsx! { r#frame { name:{DynName(item.name.into())},left:x,top:y,width,height,pos_type:"absolute" } };
    }
    if kind == MenuElementKind::Divider {
        return texture(item.name.into(), MENU_DIVIDER, rect, NORMAL);
    }
    let mut contents = menu_element_contents(item, kind, enabled, width);
    if kind != MenuElementKind::Title {
        contents.extend(rsx! { texture {
            name: {DynName(format!("{}Highlight", item.name))}, width, height,
            texture_fdid: MENU_HIGHLIGHT, draw_layer: "HIGHLIGHT", blend_mode: "ADD",
            pos_type: "absolute", left: 0.0, top: 0.0,
        } });
    }
    if kind == MenuElementKind::Title {
        return place_title(item.name, contents, rect);
    }
    let name = DynName(item.name.into());
    let onclick = if enabled { item.action } else { "" };
    let disabled = !enabled;
    rsx! { button {
        name, width, height, pos_type: "absolute", left: x, top: y, onclick,
        disabled, mouse_enabled: enabled, button_default_skin: false,
        button_highlight_alpha: 1.0, {contents}
    } }
}

fn place_title(
    name: &str,
    contents: Element,
    (x, y, width, height): (f32, f32, f32, f32),
) -> Element {
    rsx! { r#frame { name: {DynName(name.into())}, width, height, left: x, top: y, pos_type: "absolute", {contents} } }
}

fn menu_element_contents(
    item: ContextMenuItem<'_>,
    kind: MenuElementKind,
    enabled: bool,
    width: f32,
) -> Element {
    let (inset, mut contents) = menu_tick(item.name, kind);
    let color = if !enabled {
        DISABLED
    } else if kind == MenuElementKind::Title {
        GOLD
    } else {
        NORMAL
    };
    contents.extend(label(
        format!("{}Text", item.name),
        item.label,
        (inset, 0.0, width - inset, ROW_HEIGHT),
        (12.0, color, "LEFT"),
    ));
    if kind == MenuElementKind::Submenu {
        contents.extend(texture(
            format!("{}SubmenuArrow", item.name),
            SUBMENU_ARROW,
            (width - 16.0, 2.0, 16.0, 16.0),
            NORMAL,
        ));
    }
    contents
}

fn menu_tick(name: &str, kind: MenuElementKind) -> (f32, Element) {
    let (box_art, tick_art, checked, box_rect, tick_rect, inset) = match kind {
        MenuElementKind::Checkbox { checked } => (
            "common-dropdown-ticksquare",
            "common-dropdown-icon-checkmark-yellow",
            checked,
            (0.0, 4.0, 12.0, 12.0),
            (0.5, 2.0, 15.0, 14.0),
            19.0,
        ),
        MenuElementKind::Radio { checked } => (
            "common-dropdown-tickradial",
            "common-dropdown-icon-radialtick-yellow",
            checked,
            (-3.0, 1.0, 18.0, 18.0),
            (-3.0, 1.0, 18.0, 18.0),
            16.0,
        ),
        _ => return (0.0, Vec::new()),
    };
    let mut contents = named_atlas_texture(format!("{name}Box"), box_art, box_rect);
    if checked {
        contents.extend(named_atlas_texture(
            format!("{name}Tick"),
            tick_art,
            tick_rect,
        ));
    }
    (inset, contents)
}

/// Menu description elements: item, template kind, enabled state.
pub type DescribedMenuElement<'a> = (ContextMenuItem<'a>, MenuElementKind, bool);

fn description_row_height(kind: MenuElementKind) -> f32 {
    match kind {
        MenuElementKind::Spacer => SPACER_HEIGHT,
        MenuElementKind::Divider => DIVIDER_HEIGHT,
        _ => ROW_HEIGHT,
    }
}

/// DropdownButton.lua minimum owner width; Menu.lua content-fit compositor;
/// MenuTemplates.CreateSpacer defaults to10. No synthetic title/divider is inserted.
pub fn context_menu_description(
    name: &str,
    (x, y): (f32, f32),
    minimum_width: f32,
    entries: &[DescribedMenuElement<'_>],
) -> Element {
    use ui_toolkit::{text_measure::measure_text, widgets::font_string::GameFont};
    let content_width = entries
        .iter()
        .filter(|(_, kind, _)| *kind != MenuElementKind::Spacer)
        .map(|(item, kind, _)| {
            let label_width = measure_text(item.label, GameFont::FrizQuadrata, 12.)
                .expect("menu font")
                .0;
            let tick_width = match kind {
                MenuElementKind::Checkbox { .. } => 19.,
                MenuElementKind::Radio { .. } => 16.,
                _ => 0.,
            };
            let arrow_width = if *kind == MenuElementKind::Submenu {
                16.
            } else {
                0.
            };
            label_width + tick_width + arrow_width
        })
        .fold(0., f32::max);
    let width = minimum_width.max(content_width.ceil() + MENU_INSET * 2.);
    let height = MENU_INSET
        + MENU_BOTTOM
        + entries
            .iter()
            .map(|(_, kind, _)| description_row_height(*kind))
            .sum::<f32>();
    let mut contents =
        crate::selection_list::menu_background(format!("{name}Background"), (width, height));
    let mut top = MENU_INSET;
    for (item, kind, enabled) in entries {
        let row_height = description_row_height(*kind);
        contents.extend(menu_element(
            *item,
            *kind,
            *enabled,
            (MENU_INSET, top, width - 2. * MENU_INSET, row_height),
        ));
        top += row_height;
    }
    // Menu.lua:2080–2094: ordinary menu proxies use FULLSCREEN_DIALOG.
    rsx! { r#frame { name:{DynName(name.into())},width,height,left:x,top:y,pos_type:"absolute",strata:FrameStrata::FullscreenDialog,frame_level:60.0,{contents} } }
}

pub fn menu_height_for_items(item_count: usize) -> f32 {
    menu_height(item_count, true)
}
fn heading_height(has_heading: bool) -> f32 {
    if has_heading {
        ROW_HEIGHT + DIVIDER_HEIGHT
    } else {
        0.0
    }
}
fn menu_height(item_count: usize, has_heading: bool) -> f32 {
    MENU_INSET + heading_height(has_heading) + item_count as f32 * ROW_HEIGHT + MENU_BOTTOM
}

/// MenuTemplates/MenuVariants title, divider and20px entry layout.
pub fn menu_row_rect(width: f32, index: usize) -> (f32, f32, f32, f32) {
    menu_row_rect_with_heading(width, index, true)
}

/// LFDFrame.lua creates root radios without CreateTitle/CreateDivider.
pub fn menu_row_rect_with_heading(
    width: f32,
    index: usize,
    has_heading: bool,
) -> (f32, f32, f32, f32) {
    (
        MENU_INSET,
        MENU_INSET + heading_height(has_heading) + index as f32 * ROW_HEIGHT,
        width - 2.0 * MENU_INSET,
        ROW_HEIGHT,
    )
}

pub fn context_menu(props: ContextMenu<'_>) -> Element {
    let has_heading = !props.title.is_empty();
    let rows: Element = props
        .items
        .iter()
        .enumerate()
        .flat_map(|(index, item)| {
            menu_element(
                *item,
                MenuElementKind::Button,
                true,
                menu_row_rect_with_heading(props.width, index, has_heading),
            )
        })
        .collect();
    context_menu_with_rows(props, rows)
}

/// Compose menu_element rows without changing the shared frame/backdrop/layout.
/// Empty title omits the heading/divider rather than creating a blank header.
pub fn context_menu_with_rows(props: ContextMenu<'_>, rows: Element) -> Element {
    let height = menu_height(props.items.len(), !props.title.is_empty());
    let mut contents = crate::selection_list::menu_background(
        format!("{}Background", props.frame_name),
        (props.width, height),
    );
    contents.extend(context_menu_heading(props));
    contents.extend(rows);
    rsx! { r#frame {
        name: {DynName(props.frame_name.into())}, width: {props.width}, height, hidden: props.hidden,
        // Mainline Menu.lua:2080–2094 / UIDropDownMenu.xml:3.
        strata: FrameStrata::FullscreenDialog, frame_level: 60.0, pos_type: "absolute", left: {props.x}, top: {props.y}, {contents}
    } }
}

fn context_menu_heading(props: ContextMenu<'_>) -> Element {
    if props.title.is_empty() {
        return Vec::new();
    }
    let mut heading = label(
        props.title_name.into(),
        props.title,
        (MENU_INSET, MENU_INSET, props.width - 16.0, ROW_HEIGHT),
        (12.0, GOLD, "LEFT"),
    );
    heading.extend(texture(
        props.divider_name.into(),
        MENU_DIVIDER,
        (
            MENU_INSET,
            MENU_INSET + ROW_HEIGHT,
            props.width - 16.0,
            DIVIDER_HEIGHT,
        ),
        NORMAL,
    ));
    heading
}
