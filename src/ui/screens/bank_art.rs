//! Retail art and widgets shared by the bank frame and the guild bank frame: item
//! slots, textures cropped from atlas members, labels and the money entry prompt
//! (`MoneyInputFrameTemplate` in a `StaticPopup`).

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::screens::merchant_frame_component::{MoneyAlign, money};
use crate::ui::screens::quest_art::{DynName, NORMAL_FONT_COLOR, atlas_texture, panel_button};
use crate::ui::screens::static_popup_component::STATIC_POPUP_PANEL_STYLE;
use crate::ui::strata::FrameStrata;

pub const WHITE: &str = "1.0,1.0,1.0,1.0";
pub const HIGHLIGHT_FONT_COLOR: &str = "1.0,1.0,1.0,1.0";
pub const RED_FONT_COLOR: &str = "1.0,0.1,0.1,1.0";
/// Retail `ITEM_BUTTON` size (37×37).
pub const ITEM_BUTTON: f32 = 37.0;
/// `Interface\Buttons\UI-Quickslot2`.
pub const QUICKSLOT: u32 = 130_841;
/// `Interface\Icons\INV_Misc_QuestionMark`.
pub const UNKNOWN_ICON: u32 = 134_400;
/// Selected-tab marker: UI-Quickslot2 tinted gold. Retail draws
/// `Interface\Buttons\CheckButtonHilight` with ADD blending, which the ui-toolkit
/// texture renderer does not do (alpha-blended it is a black square).
pub fn selected_marker(name: String, (x, y, size): (f32, f32, f32)) -> Element {
    let quick = size * 64.0 / 37.0;
    texture(
        name,
        QUICKSLOT,
        (
            x + (size - quick) / 2.0,
            y + (size - quick) / 2.0,
            quick,
            quick,
        ),
        "1.0,0.82,0.0,1.0",
    )
}
/// `Interface\Buttons\UI-CheckBox-Up` / `-Check`.
pub const CHECKBOX_UP: u32 = 130_755;
pub const CHECKBOX_CHECK: u32 = 130_751;
/// `Interface\Common\Common-Input-Border` (`InputBoxTemplate`).
const INPUT_BORDER: u32 = 130_975;

/// Item shown in a slot.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SlotItem {
    pub icon_fdid: u32,
    pub count: u32,
    /// Retail `ITEM_QUALITY_COLORS` border colour.
    pub quality_border: String,
}

/// Absolutely positioned whole texture.
pub fn texture(name: String, fdid: u32, rect: (f32, f32, f32, f32), color: &str) -> Element {
    let (x, y, width, height) = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: fdid,
            vertex_color: color,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// Absolutely positioned texture cut by `tex_coords` ("left,right,top,bottom").
pub fn cropped(name: String, fdid: u32, coords: &str, rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, width, height) = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: fdid,
            tex_coords: coords,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

pub fn atlas(name: String, art: &AtlasArt, rect: (f32, f32, f32, f32)) -> Element {
    atlas_texture(name, art, rect)
}

/// A shadowed label; `size` is the font size.
pub fn label(
    name: String,
    text: &str,
    rect: (f32, f32, f32, f32),
    (size, color, justify): (f32, &str, &str),
) -> Element {
    let (x, y, width, height) = rect;
    rsx! {
        fontstring {
            name: {DynName(name)},
            width,
            height,
            text,
            font: GameFont::FrizQuadrata,
            font_size: size,
            font_color: color,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: justify,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// One clickable `ItemButton` at `(x, y)` over its slot background; `action` runs on
/// any click of the slot.
pub fn item_slot(
    prefix: &str,
    (x, y): (f32, f32),
    background: Element,
    item: Option<&SlotItem>,
    action: &str,
) -> Element {
    let mut children = background;
    if let Some(item) = item {
        children.extend(texture(
            format!("{prefix}Icon"),
            item.icon_fdid,
            (0.0, 0.0, ITEM_BUTTON, ITEM_BUTTON),
            WHITE,
        ));
        if item.quality_border != WHITE {
            children.extend(texture(
                format!("{prefix}IconBorder"),
                QUICKSLOT,
                (-13.0, -13.0, 64.0, 64.0),
                &item.quality_border,
            ));
        }
        if item.count > 1 {
            children.extend(label(
                format!("{prefix}Count"),
                &item.count.to_string(),
                (0.0, ITEM_BUTTON - 16.0, ITEM_BUTTON - 5.0, 14.0),
                (14.0, WHITE, "RIGHT"),
            ));
        }
    }
    rsx! {
        r#frame {
            name: {DynName(prefix.into())},
            width: ITEM_BUTTON,
            height: ITEM_BUTTON,
            onclick: action,
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

/// Gold / silver / copper edit box names of a money entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoneyBoxNames {
    pub gold: &'static str,
    pub silver: &'static str,
    pub copper: &'static str,
}

impl MoneyBoxNames {
    pub fn all(self) -> [&'static str; 3] {
        [self.gold, self.silver, self.copper]
    }
}

/// A text edit box on `InputBoxTemplate` art; the registry keeps what was typed.
pub fn edit_box(name: &'static str, rect: (f32, f32, f32, f32)) -> Element {
    let (x, y, width, height) = rect;
    let mut children = cropped(
        format!("{name}Left"),
        INPUT_BORDER,
        "0.0,0.0625,0.0,0.625",
        (x - 5.0, y, 8.0, height),
    );
    children.extend(cropped(
        format!("{name}Middle"),
        INPUT_BORDER,
        "0.0625,0.9375,0.0,0.625",
        (x + 3.0, y, width - 6.0, height),
    ));
    children.extend(cropped(
        format!("{name}Right"),
        INPUT_BORDER,
        "0.9375,1.0,0.0,0.625",
        (x + width - 3.0, y, 8.0, height),
    ));
    children.extend(rsx! {
        editbox {
            name: {DynName(name.to_string())},
            width,
            height,
            font: GameFont::ArialNarrow,
            font_size: 14.0,
            font_color: HIGHLIGHT_FONT_COLOR,
            text_insets: "2,0,0,0",
            pos_type: "absolute",
            left: x,
            top: y,
        }
    });
    children
}

/// `MoneyInputFrameTemplate` 176×18 (Blizzard_MoneyFrame/Mainline/MoneyInputFrame.xml:72):
/// gold 70 wide, silver and copper 28, each followed by its coin label.
pub fn money_input(boxes: MoneyBoxNames, pos: (f32, f32)) -> Element {
    money_input_sized(boxes, pos, (70.0, 28.0), 22.0)
}

/// `MoneyInputFrame_SetCompact`: a 62 wide gold box and narrower silver / copper
/// boxes packed into a 160 wide row (the trade frame's money inset).
pub fn money_input_compact(boxes: MoneyBoxNames, pos: (f32, f32)) -> Element {
    money_input_sized(boxes, pos, (62.0, 22.0), 14.0)
}

/// Gold / silver / copper boxes of `(gold_w, small_w)`, each box and its coin label
/// taking `label_w` more before the next box.
fn money_input_sized(
    boxes: MoneyBoxNames,
    (x, y): (f32, f32),
    (gold_w, small_w): (f32, f32),
    label_w: f32,
) -> Element {
    let silver_x = x + gold_w + label_w;
    let copper_x = silver_x + small_w + label_w;
    let parts = [
        (boxes.gold, x, gold_w, "g"),
        (boxes.silver, silver_x, small_w, "s"),
        (boxes.copper, copper_x, small_w, "c"),
    ];
    parts
        .into_iter()
        .flat_map(|(name, bx, width, unit)| {
            let mut out = edit_box(name, (bx, y, width, 18.0));
            out.extend(label(
                format!("{name}Unit"),
                unit,
                (bx + width + 4.0, y + 2.0, 14.0, 14.0),
                (12.0, NORMAL_FONT_COLOR, "LEFT"),
            ));
            out
        })
        .collect()
}

pub struct MoneyPrompt<'a> {
    pub name: &'a str,
    /// `BANK_MONEY_DEPOSIT_PROMPT` "Amount to deposit:" etc.
    pub text: &'a str,
    pub boxes: MoneyBoxNames,
    pub accept_action: &'a str,
    pub cancel_action: &'a str,
}

/// `StaticPopup` 320×116 with a money input and Accept / Cancel (GameDialogDefs.lua:559-610).
pub fn money_prompt(prompt: &MoneyPrompt, (x, y): (f32, f32)) -> Element {
    let name = prompt.name;
    let mut children = label(
        format!("{name}Text"),
        prompt.text,
        (15.0, 16.0, 290.0, 16.0),
        (14.0, HIGHLIGHT_FONT_COLOR, "CENTER"),
    );
    children.extend(money_input(prompt.boxes, (72.0, 44.0)));
    children.extend(panel_button(
        format!("{name}Accept"),
        "Accept",
        prompt.accept_action,
        true,
        (27.0, 78.0, 128.0, 21.0),
    ));
    children.extend(panel_button(
        format!("{name}Cancel"),
        "Cancel",
        prompt.cancel_action,
        true,
        (165.0, 78.0, 128.0, 21.0),
    ));
    rsx! {
        r#frame {
            name: {DynName(name.to_string())},
            width: 320.0,
            height: 116.0,
            style: STATIC_POPUP_PANEL_STYLE,
            strata: FrameStrata::Dialog,
            frame_level: 200.0,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

/// `UICheckButtonTemplate` 24×24 with its label to the right.
pub fn checkbox(
    name: &str,
    text: &str,
    checked: bool,
    action: &str,
    (x, y): (f32, f32),
) -> Element {
    let mut children = texture(
        format!("{name}Up"),
        CHECKBOX_UP,
        (0.0, 0.0, 24.0, 24.0),
        WHITE,
    );
    if checked {
        children.extend(texture(
            format!("{name}Check"),
            CHECKBOX_CHECK,
            (0.0, 0.0, 24.0, 24.0),
            WHITE,
        ));
    }
    children.extend(label(
        format!("{name}Text"),
        text,
        (28.0, 5.0, 180.0, 14.0),
        (12.0, NORMAL_FONT_COLOR, "LEFT"),
    ));
    rsx! {
        r#frame {
            name: {DynName(name.to_string())},
            width: 24.0,
            height: 24.0,
            onclick: action,
            mouse_enabled: true,
            pos_type: "absolute",
            left: x,
            top: y,
            {children}
        }
    }
}

/// `SmallMoneyFrameTemplate` right-aligned at `right`, bottom at `bottom`.
pub fn money_display(
    prefix: &str,
    copper: u64,
    (right, bottom): (f32, f32),
    gray: bool,
) -> Element {
    money(prefix, copper, (right, bottom), MoneyAlign::Right, gray)
}
