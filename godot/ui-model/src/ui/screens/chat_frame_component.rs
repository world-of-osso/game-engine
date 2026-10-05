//! Tabbed chat frame (`ChatFrame1`) with the Chattynator addon's default look: its Dark
//! skin (Skins/Dark.lua) and default window (Core/Config.lua). Lua references are to the
//! Chattynator source; positions are from the frame's top-left, y down.

use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;

use crate::chat_data::ChatState;
use crate::flare_panel::{
    FLARE_ACTIVE_TEXT, FLARE_CHANNEL_ART, FLARE_FONT_SIZE, FLARE_HEADER_BUTTON_SIZE,
    FLARE_HEADER_HEIGHT, FLARE_HEADER_ICON_INSET, FLARE_HEADER_ICON_SIZE, FLARE_INACTIVE_TEXT,
    FLARE_MENU_ART, FLARE_SOCIAL_ART, FLARE_VOLUME_ART, flare_header, flare_icon, flare_panel,
    flare_text,
};
use crate::hud_layout::hud_layout;
use crate::ui::anchor::FrameName;
use crate::ui::chat_frame::{
    ChatFrameState, ChatRow, ChatRun, ChatTab, CombatLogChat, local_timestamp, messages_that_fit,
    tab_entries, wrap_chat_line,
};
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

pub const CHAT_FRAME: FrameName = FrameName("ChatFrame1");
pub const CHAT_EDITBOX: FrameName = FrameName("ChatFrame1EditBox");
pub const CHAT_BACKGROUND: FrameName = FrameName("ChatFrame1Background");
const CHAT_EDITBOX_BACKGROUND: FrameName = FrameName("ChatFrame1EditBoxBackground");
pub const CHAT_TABS: &str = "ChatFrame1Tabs";
pub const CHAT_MESSAGES: &str = "ChatFrame1Messages";
pub const CHAT_COPY_BUTTON: &str = "ChatFrame1CopyButton";
pub const CHAT_SCROLL_TO_BOTTOM_BUTTON: &str = "ChatFrame1ScrollToBottomButton";
pub const COPY_CHAT_ACTION: &str = "chat:copy";
pub const SCROLL_TO_BOTTOM_ACTION: &str = "chat:scroll_to_bottom";

/// Tab bar at TOPLEFT (32, 0), 22 high (Display/Main.lua:50-52).
const TABS_LEFT: f32 = 32.0;
const TAB_H: f32 = 22.0;
/// Constants.lua:14-16.
const MIN_TAB_TEXT_W: f32 = 20.0;
const TAB_PADDING: f32 = 30.0;
const TAB_SPACING: f32 = 10.0;
/// Tab end caps (Skins/Dark.lua:201-212) and the larger flash caps (Dark.lua:219-234).
const TAB_CAP_W: f32 = 6.0;
const FLASH_CAP_W: f32 = 8.0;
const FLASH_H: f32 = 24.0;
/// Unselected tabs are drawn at half alpha (Skins/Dark.lua:298-302).
const UNSELECTED_TAB_ALPHA: f32 = 0.5;
/// Tab labels use GameFontNormalSmall (Skins/Dark.lua:243), 5 below the tab top (Dark.lua:272).
const TAB_FONT: GameFont = GameFont::FrizQuadrata;
const TAB_FONT_SIZE: f32 = 10.0;
const TAB_TEXT_TOP: f32 = 5.0;
const TAB_TEXT_COLOR: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
/// Message wrapper at TOPLEFT (34, -27) to BOTTOMRIGHT (0, 38), the messages 5 short of its
/// right edge (Display/Main.lua:26,30-31; 38 = 6 + the 32 high edit box, Main.lua:205).
const MESSAGES_LEFT: f32 = 34.0;
const MESSAGES_TOP: f32 = 27.0;
/// The message area of a chat frame of `size` (the active layout's `chat_size`; Modern's
/// default window is 500x280 at BOTTOMLEFT (0, 40), Core/Config.lua:28-29).
fn messages_size((width, height): (f32, f32)) -> (f32, f32) {
    (width - MESSAGES_LEFT - 5.0, height - MESSAGES_TOP - 38.0)
}
/// Background spans from the frame's left edge to 5 beyond the wrapper above and below
/// (Skins/Dark.lua:155-157), at alpha 1 - `chat_transparency` 0.2 (Dark.lua:152,474).
const BACKGROUND_TOP: f32 = MESSAGES_TOP - 5.0;
const BACKGROUND_ALPHA: f32 = 0.8;
/// `Assets/ChatBackground.tga`, stored flipped vertically because the skin draws it with
/// `SetTexCoord(0, 1, 1, 0)` (Skins/Dark.lua:168-169).
const BACKGROUND_TEXTURE: &str = "data/textures/ui/chattynator/ChatBackground.png";
/// Forever skin: FlareUI's `FlareUI_Skin` replaces the background, `textPadding` 10 around
/// the messages and `headerHeight` 24 above them (Chat.lua:1515-1520, Core.lua:143,146).
pub const CHAT_FLARE_SKIN: &str = "ChatFrame1FlareSkin";
const FLARE_PADDING: f32 = 10.0;
const FLARE_HEADER_H: f32 = 24.0;
const FLARE_SKIN_ORIGIN: (f32, f32) = (
    MESSAGES_LEFT - FLARE_PADDING,
    MESSAGES_TOP - FLARE_PADDING - FLARE_HEADER_H,
);

/// The skin around a `messages`-sized message area: 450x214 around Forever's 430x170.
fn flare_skin_rect((width, height): (f32, f32)) -> (f32, f32, f32, f32) {
    (
        FLARE_SKIN_ORIGIN.0,
        FLARE_SKIN_ORIGIN.1,
        width + 2.0 * FLARE_PADDING,
        height + 2.0 * FLARE_PADDING + FLARE_HEADER_H,
    )
}
const TAB_LEFT_TEXTURE: &str = "data/textures/ui/chattynator/ChatTabLeft.png";
const TAB_MIDDLE_TEXTURE: &str = "data/textures/ui/chattynator/ChatTabMiddle.png";
const TAB_RIGHT_TEXTURE: &str = "data/textures/ui/chattynator/ChatTabRight.png";
const BUTTON_TEXTURE: &str = "data/textures/ui/chattynator/ChatButton.png";
const COPY_ICON: &str = "data/textures/ui/chattynator/Copy.png";
const SCROLL_TO_BOTTOM_ICON: &str = "data/textures/ui/chattynator/ScrollToBottom.png";
const SEPARATOR_TEXTURE: &str = "data/textures/ui/chattynator/Fade.png";
/// Chat buttons are 26x28 on ChatButton.png tinted 0.15, with a 15x15 icon tinted 0.8
/// (Skins/Dark.lua:6,29-31,146-148).
const BUTTON_W: f32 = 26.0;
const BUTTON_H: f32 = 28.0;
const BUTTON_ICON: f32 = 15.0;
const BUTTON_COLOR: &str = "0.15,0.15,0.15,1";
const BUTTON_ICON_COLOR: &str = "0.8,0.8,0.8,1";
/// `outside_left` buttons stack down from TOPRIGHT at the messages' TOPLEFT (-5, 20)
/// (Core/Config.lua:116, Display/Buttons.lua:299,310).
const BUTTONS_TOP: f32 = MESSAGES_TOP - 20.0;
const BUTTONS_LEFT: f32 = MESSAGES_LEFT - 5.0 - BUTTON_W;
/// Scroll to bottom sits at the messages' BOTTOMRIGHT (-2, 5) (Display/Buttons.lua:307).
/// Blizzard's ChatFrame1EditBox (32 high) across the frame bottom, its art replaced by a
/// 0.1 grey fill at alpha 0.8 (Display/Main.lua:203-207, Skins/Dark.lua:185-194).
const INPUT_H: f32 = 32.0;
const INPUT_BACKGROUND: &str = "0.1,0.1,0.1,0.8";
/// Retail edit box: ChatFontNormal text in the chat type's colour after a `Say: ` header
/// (`CHAT_SAY_SEND`) at LEFT (15, 0), text inset 15 + header width on the left and 13 on
/// the right (ChatFrameEditBox.xml:48-53, 111; ChatFrameEditBox.lua:674-692).
const INPUT_HEADER: &str = "Say: ";
const INPUT_HEADER_LEFT: f32 = 15.0;
const INPUT_RIGHT_INSET: f32 = 13.0;
const CHAT_EDITBOX_HEADER: FrameName = FrameName("ChatFrame1EditBoxHeader");
/// Messages use ChatFontNormal (Core/Fonts.lua:7) at `message_font_size` 14
/// (Core/Config.lua:102) with `line_spacing` 0 (Config.lua:95).
pub const CHAT_FONT: GameFont = GameFont::ArialNarrow;
pub const CHAT_FONT_SIZE: f32 = 14.0;
pub const CHAT_LINE_H: f32 = 14.0;
/// `message_spacing` 5 between messages (Core/Config.lua:96).
const MESSAGE_SPACING: f32 = 5.0;
/// The newest message sits 2 above the bottom (Display/ScrollingMessages.lua:228).
const MESSAGES_BOTTOM_PAD: f32 = 2.0;
/// Timestamps are grey (Display/ScrollingMessages.lua:247).
const TIMESTAMP_COLOR: FontColor = FontColor::new(0.6, 0.6, 0.6, 1.0);
/// ChatTypeInfo SAY, the edit box's default chat type.
const SAY_COLOR: FontColor = FontColor::new(1.0, 1.0, 1.0, 1.0);
/// Blizzard ChatFrame2 stops 15 short of the combat log holder's right (API/CustomTab.lua:30).
const COMBAT_LOG_RIGHT_INSET: f32 = 15.0;
const LINK_PREFIX: &str = "ChatFrame1Link";

struct DynName(String);

/// One shown message: its timestamp (none in the combat log) and wrapped rows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChatMessageView {
    pub timestamp: Option<String>,
    pub rows: Vec<ChatRow>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChatFrameView {
    pub tab: ChatTab,
    /// Shown messages, oldest first; the last one sits at the bottom.
    pub messages: Vec<ChatMessageView>,
    pub input_open: bool,
    pub flashing: Vec<ChatTab>,
    /// The Scroll to bottom button shows while scrolled up (Display/Buttons.lua:14-18).
    pub scrolled_up: bool,
}

/// Where a tab's message text goes and how it is spaced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChatTextArea {
    /// Text left edge in the message area.
    pub left: f32,
    /// Width lines wrap to.
    pub width: f32,
    pub spacing: f32,
}

/// Text starts `inset + 3` in and ends 1 short of the right (Display/ScrollingMessages.lua:
/// 226-227); the inset is the width of `00:00:00` plus 8 (Core/Messages.lua:367-384).
/// `messages_width` is the message area's width.
pub fn chat_text_area(tab: ChatTab, messages_width: f32) -> ChatTextArea {
    if tab.is_combat_log() {
        return ChatTextArea {
            left: 0.0,
            width: messages_width - COMBAT_LOG_RIGHT_INSET,
            spacing: 0.0,
        };
    }
    let left = timestamp_inset() + 3.0;
    ChatTextArea {
        left,
        width: messages_width - left - 1.0,
        spacing: MESSAGE_SPACING,
    }
}

fn timestamp_inset() -> f32 {
    text_width("00:00:00", CHAT_FONT, CHAT_FONT_SIZE) + 8.0
}

fn text_width(value: &str, font: GameFont, size: f32) -> f32 {
    measure_text(value, font, size).map_or(0.0, |(width, _)| width)
}

/// The messages that fit a chat frame of `chat_size`, wrapped to its width, counting up
/// from the newest past the scrolled-over ones.
pub fn chat_frame_view(
    state: &ChatFrameState,
    chat: &ChatState,
    combat: &CombatLogChat,
    spell_name: impl Fn(u32) -> String,
    chat_size: (f32, f32),
) -> ChatFrameView {
    let (messages_width, messages_height) = messages_size(chat_size);
    let available_height = messages_height - MESSAGES_BOTTOM_PAD;
    let area = chat_text_area(state.tab, messages_width);
    let entries = tab_entries(state.tab, chat, combat);
    let mut messages = Vec::new();
    let mut heights = Vec::new();
    for entry in entries.iter().rev().skip(state.scroll()) {
        let rows = wrap_chat_line(&entry.line, &spell_name, area.width, measure_chat_text);
        heights.push(rows.len() as f32 * CHAT_LINE_H);
        if messages_that_fit(&heights, available_height, area.spacing) < heights.len() {
            break;
        }
        messages.push(ChatMessageView {
            timestamp: (!state.tab.is_combat_log()).then(|| local_timestamp(entry.timestamp)),
            rows,
        });
    }
    messages.reverse();
    ChatFrameView {
        tab: state.tab,
        messages,
        input_open: state.input_open,
        flashing: state.flashing.clone(),
        scrolled_up: state.scroll() > 0,
    }
}

pub fn measure_chat_text(value: &str) -> f32 {
    text_width(value, CHAT_FONT, CHAT_FONT_SIZE)
}

pub fn tab_name(index: usize) -> String {
    format!("{CHAT_TABS}Tab{index}")
}

pub fn tab_flash_name(index: usize) -> String {
    format!("{}Flash", tab_name(index))
}

/// Tab width: the label, at least 20, plus 30 padding (Skins/Dark.lua:253, Constants.lua),
/// rounded up to whole units so the caps and middle meet without a seam after layout
/// rounding.
fn tab_width(tab: ChatTab) -> f32 {
    (text_width(tab.label(), TAB_FONT, TAB_FONT_SIZE).max(MIN_TAB_TEXT_W) + TAB_PADDING).ceil()
}

fn chattynator_background(tab: ChatTab, width: f32, messages_height: f32) -> Element {
    let [r, g, b] = tab.background();
    rsx! {
        texture {
            name: CHAT_BACKGROUND,
            width,
            height: {messages_height + 10.0},
            texture_file: BACKGROUND_TEXTURE,
            vertex_color: {format!("{r},{g},{b},{BACKGROUND_ALPHA}")},
            pos_type: "absolute",
            left: 0.0,
            top: BACKGROUND_TOP,
        }
    }
}

pub fn chat_frame_screen(ctx: &SharedContext) -> Element {
    let view = ctx
        .get::<ChatFrameView>()
        .expect("ChatFrameView must be in SharedContext");
    let hide_input = !view.input_open;
    let hide_scroll_button = !view.scrolled_up;
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let layout = hud_layout(ctx);
    let (width, height) = layout.chat_size;
    let (messages_width, messages_height) = messages_size((width, height));
    let background = match skin {
        ActiveSkin::Modern => chattynator_background(view.tab, width, messages_height),
        ActiveSkin::Forever => {
            forever_background(flare_skin_rect((messages_width, messages_height)))
        }
    };
    let tab_parts = match skin {
        ActiveSkin::Modern => tabs(view),
        ActiveSkin::Forever => forever_tabs(view),
    };
    let copy_button = match skin {
        ActiveSkin::Modern => chat_button(
            CHAT_COPY_BUTTON,
            COPY_CHAT_ACTION,
            COPY_ICON,
            BUTTONS_LEFT,
            BUTTONS_TOP,
            false,
        ),
        ActiveSkin::Forever => Element::new(), // Copy action lives on the header's menu icon.
    };
    let input_header_w = text_width(INPUT_HEADER, CHAT_FONT, CHAT_FONT_SIZE).ceil();
    let input_insets = format!(
        "{},{INPUT_RIGHT_INSET},0,0",
        INPUT_HEADER_LEFT + input_header_w
    );
    rsx! {
        r#frame {
            name: CHAT_FRAME,
            width,
            height,
            pos_type: "absolute",
            left: {layout.chat.x},
            bottom: {layout.chat.y},
            {background}
            {tab_parts}
            r#frame {
                name: {DynName(CHAT_MESSAGES.to_string())},
                width: messages_width,
                height: messages_height,
                pos_type: "absolute",
                left: MESSAGES_LEFT,
                top: MESSAGES_TOP,
                {messages(view, (messages_width, messages_height))}
            }
            {copy_button}
            {chat_button(
                CHAT_SCROLL_TO_BOTTOM_BUTTON,
                SCROLL_TO_BOTTOM_ACTION,
                SCROLL_TO_BOTTOM_ICON,
                MESSAGES_LEFT + messages_width - 2.0 - BUTTON_W,
                MESSAGES_TOP + messages_height - 5.0 - BUTTON_H,
                hide_scroll_button,
            )}
            r#frame {
                name: CHAT_EDITBOX_BACKGROUND,
                width,
                height: INPUT_H,
                background_color: INPUT_BACKGROUND,
                hidden: hide_input,
                pos_type: "absolute",
                left: 0.0,
                top: {height - INPUT_H},
            }
            fontstring {
                name: CHAT_EDITBOX_HEADER,
                width: {input_header_w},
                height: CHAT_LINE_H,
                text: INPUT_HEADER,
                font: CHAT_FONT,
                font_size: CHAT_FONT_SIZE,
                font_color: SAY_COLOR,
                justify_h: "LEFT",
                hidden: hide_input,
                pos_type: "absolute",
                left: INPUT_HEADER_LEFT,
                top: {height - (INPUT_H + CHAT_LINE_H) / 2.0},
            }
            editbox {
                name: CHAT_EDITBOX,
                width,
                height: INPUT_H,
                font: CHAT_FONT,
                font_size: CHAT_FONT_SIZE,
                font_color: SAY_COLOR,
                text_insets: {input_insets.as_str()},
                hidden: hide_input,
                pos_type: "absolute",
                left: 0.0,
                top: {height - INPUT_H},
            }
        }
    }
}

// Sourced palette/layout and measured adaptations:
// docs/specs/forever-chat-meter-chrome.md. No message/input/scroll changes.
const FOREVER_TAB_PADDING: f32 = 14.0;
const FOREVER_TAB_GAP: f32 = 4.0;
/// FlareUI scales each 22-unit header button by 0.6 (`Core.lua:161,166,171,176`), so the
/// `SetPoint` offsets `HEADER_FIRST_X` -25 and `HEADER_STEP` -35 (`Chat.lua:472,481-487`)
/// are in the button's scaled space: on the skin they are 15 and 21 units.
const FLARE_HEADER_BUTTON_SCALE: f32 = 0.6;
/// The rightmost glyph's right edge, inside the skin's right edge.
const FOREVER_HEADER_RIGHT: f32 = 25.0 * FLARE_HEADER_BUTTON_SCALE;
const FOREVER_HEADER_STEP: f32 = 35.0 * FLARE_HEADER_BUTTON_SCALE;
const FOREVER_BUTTON_TOP: f32 =
    FLARE_SKIN_ORIGIN.1 + (FLARE_HEADER_HEIGHT - FLARE_HEADER_BUTTON_SIZE) / 2.0;

/// Header glyphs left to right: FlareUI packs `HEADER_ORDER` volume, social, menu,
/// channel from the right edge (Chat.lua:471-487).
const FOREVER_HEADER_GLYPHS: [(&str, (u32, [f32; 4]), Option<&str>); 4] = [
    ("Channel", FLARE_CHANNEL_ART, None),
    ("Menu", FLARE_MENU_ART, Some(COPY_CHAT_ACTION)),
    ("Social", FLARE_SOCIAL_ART, None),
    ("Volume", FLARE_VOLUME_ART, None),
];

/// Textures the host must copy from local CASC before drawing the Forever chat header.
pub const FOREVER_CHAT_HEADER_FDIDS: [u32; 4] = [
    FLARE_CHANNEL_ART.0,
    FLARE_MENU_ART.0,
    FLARE_SOCIAL_ART.0,
    FLARE_VOLUME_ART.0,
];

/// The header glyph textures. FlareUI tints white icons with `SetVertexColor` (Chat.lua:547,
/// colours Core.lua:162-177), so the host draws each Blizzard crop as a white mask and the
/// vertex colour is the glyph's colour.
pub const FOREVER_CHAT_HEADER_ICONS: [&str; 4] = [
    "ChatFrame1FlareChannelIcon",
    "ChatFrame1FlareMenuIcon",
    "ChatFrame1FlareSocialIcon",
    "ChatFrame1FlareVolumeIcon",
];

/// Hit box of header glyph `index` (left to right) on a skin at `skin_x` `skin_width` wide.
fn forever_header_button(index: usize, skin_x: f32, skin_width: f32) -> [f32; 4] {
    let from_right = (FOREVER_HEADER_GLYPHS.len() - 1 - index) as f32;
    let glyph_left = skin_x + skin_width
        - FOREVER_HEADER_RIGHT
        - FLARE_HEADER_ICON_SIZE
        - FOREVER_HEADER_STEP * from_right;
    [
        glyph_left - FLARE_HEADER_ICON_INSET,
        FOREVER_BUTTON_TOP,
        FLARE_HEADER_BUTTON_SIZE,
        FLARE_HEADER_BUTTON_SIZE,
    ]
}

fn forever_background(skin_rect: (f32, f32, f32, f32)) -> Element {
    let (skin_x, _, skin_width, _) = skin_rect;
    let mut parts = flare_panel(CHAT_FLARE_SKIN, skin_rect);
    parts.extend(flare_header("ChatFrame1Flare", skin_rect));
    for (index, (suffix, art, action)) in FOREVER_HEADER_GLYPHS.into_iter().enumerate() {
        parts.extend(flare_icon(
            &format!("ChatFrame1Flare{suffix}"),
            art,
            forever_header_button(index, skin_x, skin_width),
            action,
        ));
    }
    parts
}

fn forever_tabs(view: &ChatFrameView) -> Element {
    let mut x = FLARE_SKIN_ORIGIN.0 + FLARE_PADDING;
    let mut parts = Element::new();
    for (index, tab) in ChatTab::ALL.into_iter().enumerate() {
        let width = text_width(tab.label(), TAB_FONT, FLARE_FONT_SIZE).ceil() + FOREVER_TAB_PADDING;
        parts.extend(forever_tab_button(index, tab, x, width, tab == view.tab));
        parts.extend(forever_tab_flash(
            index,
            tab,
            x,
            width,
            view.flashing.contains(&tab),
        ));
        x += width + FOREVER_TAB_GAP;
    }
    parts
}

fn forever_tab_button(index: usize, tab: ChatTab, x: f32, width: f32, selected: bool) -> Element {
    let name = tab_name(index);
    let color = if selected {
        FLARE_ACTIVE_TEXT
    } else {
        FLARE_INACTIVE_TEXT
    };
    let label = flare_text(
        &format!("{name}Text"),
        tab.label(),
        [0.0, 6.0, width, FLARE_FONT_SIZE],
        color,
        JustifyH::Center,
    );
    rsx! {
        r#frame {
            name: {DynName(name)},
            width,
            height: FLARE_HEADER_HEIGHT,
            mouse_enabled: true,
            onclick: {tab.action()},
            pos_type: "absolute",
            left: x,
            top: {FLARE_SKIN_ORIGIN.1},
            {label}
        }
    }
}

/// Keep the existing host's named flash pulse, but flash text rather than coloured caps.
fn forever_tab_flash(index: usize, tab: ChatTab, x: f32, width: f32, flashing: bool) -> Element {
    let name = tab_flash_name(index);
    let hidden = !flashing;
    let label = flare_text(
        &format!("{name}Text"),
        tab.label(),
        [0.0, 6.0, width, FLARE_FONT_SIZE],
        FLARE_ACTIVE_TEXT,
        JustifyH::Center,
    );
    rsx! {
        r#frame {
            name: {DynName(name)},
            width,
            height: FLARE_HEADER_HEIGHT,
            hidden,
            pos_type: "absolute",
            left: x,
            top: {FLARE_SKIN_ORIGIN.1},
            {label}
        }
    }
}

/// Tabs left to right, 10 apart (Display/Tabs.lua:47-55), each flash drawn over its tab.
fn tabs(view: &ChatFrameView) -> Element {
    let mut x = TABS_LEFT;
    let mut tabs = Element::new();
    let mut flashes = Element::new();
    for (index, tab) in ChatTab::ALL.into_iter().enumerate() {
        let width = tab_width(tab);
        tabs.extend(tab_button(index, tab, x, width, tab == view.tab));
        flashes.extend(tab_flash(
            index,
            tab,
            x,
            width,
            view.flashing.contains(&tab),
        ));
        x += width + TAB_SPACING;
    }
    tabs.extend(flashes);
    tabs
}

fn tab_button(index: usize, tab: ChatTab, x: f32, width: f32, selected: bool) -> Element {
    let alpha = if selected { 1.0 } else { UNSELECTED_TAB_ALPHA };
    let name = tab_name(index);
    rsx! {
        r#frame {
            name: {DynName(name.clone())},
            width: width,
            height: TAB_H,
            alpha: alpha,
            mouse_enabled: true,
            onclick: {tab.action()},
            pos_type: "absolute",
            left: x,
            top: 0.0,
            {three_slice(&name, tab.color(), width, TAB_H, TAB_CAP_W, TAB_CAP_W)}
            fontstring {
                name: {DynName(format!("{name}Text"))},
                width: width,
                height: TAB_FONT_SIZE,
                text: {tab.label()},
                font: TAB_FONT,
                font_size: TAB_FONT_SIZE,
                font_color: TAB_TEXT_COLOR,
                justify_h: "CENTER",
                pos_type: "absolute",
                left: 0.0,
                top: TAB_TEXT_TOP,
            }
        }
    }
}

/// The flash is 1 wider on each side and 2 taller than the tab, bottom aligned, in the tab
/// colour, and ignores the tab's alpha (Skins/Dark.lua:219-242,318-320). Its pulse is
/// animated by the chat frame systems.
fn tab_flash(index: usize, tab: ChatTab, x: f32, width: f32, flashing: bool) -> Element {
    let name = tab_flash_name(index);
    let hidden = !flashing;
    let flash_w = width + 2.0;
    rsx! {
        r#frame {
            name: {DynName(name.clone())},
            width: flash_w,
            height: FLASH_H,
            hidden: hidden,
            pos_type: "absolute",
            left: {x - 1.0},
            top: {TAB_H - FLASH_H},
            {three_slice(&name, tab.color(), flash_w, FLASH_H, FLASH_CAP_W, FLASH_CAP_W)}
        }
    }
}

/// Left cap, stretched middle and right cap, tinted `color`.
fn three_slice(
    name: &str,
    color: [f32; 3],
    width: f32,
    height: f32,
    left_w: f32,
    right_w: f32,
) -> Element {
    let [r, g, b] = color;
    let tint = format!("{r},{g},{b},1");
    let parts = [
        ("Left", TAB_LEFT_TEXTURE, 0.0, left_w),
        (
            "Middle",
            TAB_MIDDLE_TEXTURE,
            left_w,
            width - left_w - right_w,
        ),
        ("Right", TAB_RIGHT_TEXTURE, width - right_w, right_w),
    ];
    parts
        .into_iter()
        .flat_map(|(part, file, x, part_w)| {
            rsx! {
                texture {
                    name: {DynName(format!("{name}{part}"))},
                    width: part_w,
                    height: height,
                    texture_file: file,
                    vertex_color: {tint.as_str()},
                    pos_type: "absolute",
                    left: x,
                    top: 0.0,
                }
            }
        })
        .collect()
}

fn chat_button(name: &str, action: &str, icon: &str, x: f32, y: f32, hidden: bool) -> Element {
    rsx! {
        r#frame {
            name: {DynName(name.to_string())},
            width: BUTTON_W,
            height: BUTTON_H,
            hidden: hidden,
            mouse_enabled: true,
            onclick: action,
            pos_type: "absolute",
            left: x,
            top: y,
            texture {
                name: {DynName(format!("{name}Background"))},
                width: BUTTON_W,
                height: BUTTON_H,
                texture_file: BUTTON_TEXTURE,
                vertex_color: BUTTON_COLOR,
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
            texture {
                name: {DynName(format!("{name}Icon"))},
                width: BUTTON_ICON,
                height: BUTTON_ICON,
                texture_file: icon,
                vertex_color: BUTTON_ICON_COLOR,
                pos_type: "absolute",
                left: {(BUTTON_W - BUTTON_ICON) / 2.0},
                top: {(BUTTON_H - BUTTON_ICON) / 2.0},
            }
        }
    }
}

/// Messages stacked up from the bottom, newest last (Display/ScrollingMessages.lua:219-270).
/// Rows are numbered top to bottom as `ChatFrame1MessagesRow{n}`.
fn messages(view: &ChatFrameView, (width, height): (f32, f32)) -> Element {
    let area = chat_text_area(view.tab, width);
    let mut tops = Vec::with_capacity(view.messages.len());
    let mut bottom = height - MESSAGES_BOTTOM_PAD;
    for message in view.messages.iter().rev() {
        let top = bottom - message.rows.len() as f32 * CHAT_LINE_H;
        tops.push(top);
        bottom = top - area.spacing;
    }
    tops.reverse();
    let mut row = 0;
    let mut out = Element::new();
    for (index, (message, top)) in view.messages.iter().zip(tops).enumerate() {
        out.extend(message_decor(index, message, top, &area));
        for (line, chat_row) in message.rows.iter().enumerate() {
            out.extend(message_row(
                row,
                chat_row,
                &area,
                top + line as f32 * CHAT_LINE_H,
            ));
            row += 1;
        }
    }
    out
}

/// Timestamp at the left and, with `show_timestamp_separator` (Core/Config.lua:98), a 2 wide
/// Fade.png bar 4 left of the text from its top to 1 above its bottom
/// (Display/ScrollingMessages.lua:245-266).
fn message_decor(
    index: usize,
    message: &ChatMessageView,
    top: f32,
    area: &ChatTextArea,
) -> Element {
    let Some(timestamp) = message.timestamp.as_deref() else {
        return Element::new();
    };
    let height = message.rows.len() as f32 * CHAT_LINE_H;
    rsx! {
        fontstring {
            name: {DynName(format!("ChatFrame1Message{index}Time"))},
            width: {area.left},
            height: CHAT_LINE_H,
            text: timestamp,
            font: CHAT_FONT,
            font_size: CHAT_FONT_SIZE,
            font_color: TIMESTAMP_COLOR,
            justify_h: "LEFT",
            pos_type: "absolute",
            left: 0.0,
            top: top,
        }
        texture {
            name: {DynName(format!("ChatFrame1Message{index}Separator"))},
            width: 2.0,
            height: {height - 1.0},
            texture_file: SEPARATOR_TEXTURE,
            pos_type: "absolute",
            left: {area.left - 4.0 - 2.0},
            top: top,
        }
    }
}

fn message_row(row: usize, chat_row: &ChatRow, area: &ChatTextArea, top: f32) -> Element {
    let runs: Element = chat_row
        .runs
        .iter()
        .enumerate()
        .flat_map(|(run_index, run)| message_run(row, run_index, run))
        .collect();
    rsx! {
        r#frame {
            name: {DynName(format!("{CHAT_MESSAGES}Row{row}"))},
            width: {area.width},
            height: CHAT_LINE_H,
            pos_type: "absolute",
            left: {area.left},
            top: top,
            {runs}
        }
    }
}

fn message_run(row: usize, run_index: usize, run: &ChatRun) -> Element {
    let [r, g, b, a] = run.color;
    let is_link = run.spell_id.is_some();
    let name = match run.spell_id {
        Some(spell) => format!("{LINK_PREFIX}{row}_{run_index}_{spell}"),
        None => format!("{CHAT_MESSAGES}Row{row}Run{run_index}"),
    };
    rsx! {
        fontstring {
            name: {DynName(name)},
            width: {run.width.ceil() + 1.0},
            height: CHAT_LINE_H,
            text: {run.text.as_str()},
            font: CHAT_FONT,
            font_size: CHAT_FONT_SIZE,
            font_color: {FontColor::new(r, g, b, a)},
            justify_h: "LEFT",
            mouse_enabled: is_link,
            pos_type: "absolute",
            left: {run.x},
            top: 0.0,
        }
    }
}

/// Spell id of the chat link frame at or above `frame_id`.
pub fn chat_spell_link_at(registry: &FrameRegistry, mut frame_id: u64) -> Option<u32> {
    loop {
        let frame = registry.get(frame_id)?;
        if let Some(spell) = frame
            .name
            .as_deref()
            .and_then(|name| name.strip_prefix(LINK_PREFIX))
            .and_then(|rest| rest.rsplit('_').next())
            .and_then(|spell| spell.parse().ok())
        {
            return Some(spell);
        }
        frame_id = frame.parent_id?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::chat_frame::LINK_COLOR;
    use crate::ui::screens::screen_test_helpers::fontstring_text;
    use ui_toolkit::screen::Screen;

    fn run(text: &str, x: f32, spell_id: Option<u32>) -> ChatRun {
        ChatRun {
            text: text.to_string(),
            color: if spell_id.is_some() {
                LINK_COLOR
            } else {
                [1.0; 4]
            },
            x,
            width: 60.0,
            spell_id,
        }
    }

    fn build(rows: Vec<ChatRow>) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(ActiveSkin::Modern);
        shared.insert(ChatFrameView {
            tab: ChatTab::CombatLog,
            messages: vec![ChatMessageView {
                timestamp: None,
                rows,
            }],
            ..ChatFrameView::default()
        });
        Screen::new(chat_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn link_runs_resolve_to_their_spell_and_text_runs_do_not() {
        let reg = build(vec![ChatRow {
            runs: vec![run("Alice's ", 0.0, None), run("Fireball", 60.0, Some(133))],
        }]);
        let link = reg
            .get_by_name("ChatFrame1Link0_1_133")
            .expect("link frame");
        assert_eq!(fontstring_text(&reg, "ChatFrame1Link0_1_133"), "Fireball");
        assert_eq!(chat_spell_link_at(&reg, link), Some(133));
        let text = reg
            .get_by_name("ChatFrame1MessagesRow0Run0")
            .expect("text run");
        assert_eq!(chat_spell_link_at(&reg, text), None);
        assert!(
            reg.get(reg.get_by_name("ChatFrame1EditBox").unwrap())
                .unwrap()
                .hidden
        );
    }
}
