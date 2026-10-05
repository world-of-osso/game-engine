//! Retail art and layout helpers shared by the quest screens (objective tracker, quest
//! log, quest giver frame). Atlas constants cite `UiTextureAtlasMember.CommittedName`
//! (member id) and the `UiTextureAtlas` texture FileDataID.

use std::fmt;

use ui_toolkit::atlas::{AtlasSource, active_skin, resolve_region};
use ui_toolkit::rsx;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::panel_style_data::MetalTopLeft;
use crate::ui::screens::inworld_unit_frames_component::{FULL_PORTRAIT, PortraitSlot};
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::strata::FrameStrata;

pub struct DynName(pub String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

const fn art(fdid: u32, atlas: (f32, f32), rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt { fdid, atlas, rect }
}

/// UiTextureAtlas 2547 `interface/questframe/questtracker.blp`.
const TRACKER: (u32, (f32, f32)) = (5_320_671, (512.0, 256.0));

const fn tracker(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(TRACKER.0, TRACKER.1, rect)
}

/// `ui-questtracker-primary-objective-header` (23703), 300×40.
pub const TRACKER_PRIMARY_HEADER: AtlasArt = tracker((1.0, 301.0, 123.0, 163.0));
/// `ui-questtracker-secondary-objective-header` (23706), 300×30.
pub const TRACKER_SECONDARY_HEADER: AtlasArt = tracker((1.0, 301.0, 165.0, 195.0));
/// `ui-questtracker-tracker-check` (23707), drawn 16×16.
pub const TRACKER_CHECK: AtlasArt = tracker((438.0, 457.0, 31.0, 50.0));
/// `ui-questtrackerbutton-collapse-all` (23709), 18×19.
pub const TRACKER_COLLAPSE_ALL: AtlasArt = tracker((480.0, 498.0, 31.0, 50.0));
/// `ui-questtrackerbutton-expand-all` (23711), 18×19.
pub const TRACKER_EXPAND_ALL: AtlasArt = tracker((478.0, 496.0, 62.0, 81.0));
/// `ui-questtrackerbutton-secondary-collapse` (23716), 16×16.
pub const TRACKER_SECONDARY_COLLAPSE: AtlasArt = tracker((458.0, 474.0, 104.0, 120.0));
/// `ui-questtrackerbutton-secondary-expand` (23718), 16×16.
pub const TRACKER_SECONDARY_EXPAND: AtlasArt = tracker((323.0, 339.0, 123.0, 139.0));

/// `UI-QuestPoi-QuestNumber` (23605), atlas 2549 `questpoi.blp`, 32×32.
pub const POI_NUMBER: AtlasArt = art(5_320_914, (256.0, 128.0), (67.0, 99.0, 35.0, 67.0));
/// `Quest-In-Progress-Icon-yellow` (25059), atlas 2670 (2x), shown 32×32.
pub const POI_IN_PROGRESS: AtlasArt = art(5_423_566, (256.0, 128.0), (67.0, 131.0, 1.0, 65.0));
/// `UI-QuestIcon-TurnIn-Normal` (10221), atlas 1575, 32×32.
pub const POI_TURN_IN: AtlasArt = art(3_509_168, (128.0, 64.0), (69.0, 101.0, 1.0, 33.0));

/// `Interface/GossipFrame/AvailableQuestIcon` (whole 16×16 file).
pub const GOSSIP_AVAILABLE_ICON: u32 = 132_049;
/// `Interface/GossipFrame/ActiveQuestIcon`: a complete quest in the log.
pub const GOSSIP_ACTIVE_ICON: u32 = 132_048;
/// `SideInProgressquesticon` (26250), atlas 2733, 16×18: an incomplete quest.
pub const GOSSIP_IN_PROGRESS_ICON: AtlasArt = art(5_666_025, (64.0, 64.0), (37.0, 53.0, 1.0, 19.0));

/// Retail/Forever AddOns/Blizzard_UIPanels_Game/Mainline/QuestFrameTemplates.xml:10.
pub const QUEST_PARCHMENT: &str = "QuestBG-Parchment";
/// Zone header plate: Retail UiTextureAtlasMember.csv:3107, Forever:3107.
/// No XML/Lua reference in either Blizzard source export; this client's existing plate.
pub const QUEST_LOG_DIVIDER: &str = "questlog_divider";

/// Retail/Forever AddOns/Blizzard_SharedXML/Mainline/SharedUIPanelTemplates.xml:137.
const CLOSE_BUTTON: &str = "RedButton-Exit";
/// `Interface\FrameGeneral\UI-Background-Rock`, tiled window background.
const WINDOW_BACKGROUND: u32 = 374_155;
/// Retail/Forever AddOns/Blizzard_SharedXML/Mainline/SharedUIPanelTemplates.xml:76.
const TOP_TILE_STREAKS: &str = "_UI-Frame-TopTileStreaks";

/// `NORMAL_FONT_COLOR`.
pub const NORMAL_FONT_COLOR: &str = "1.0,0.82,0.0,1.0";
/// `HIGHLIGHT_FONT_COLOR`.
pub const HIGHLIGHT_FONT_COLOR: &str = "1.0,1.0,1.0,1.0";
/// `QuestFont` / `QuestTitleFont` on parchment.
pub const QUEST_TEXT_COLOR: &str = "0.0,0.0,0.0,1.0";
/// `QuestFontNormalSmall` headers ("Rewards", "Required items:").
pub const QUEST_SMALL_HEADER_COLOR: &str = "0.30,0.18,0.0,1.0";

/// Absolutely positioned texture showing one atlas member.
pub fn atlas_texture(name: String, art: &AtlasArt, rect: (f32, f32, f32, f32)) -> Element {
    let coords = art.tex_coords(1.0);
    let (x, y, width, height) = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// Resolve active-skin art into the existing FDID/UV representation, retaining Modern tree bytes.
fn read_active_atlas_art(name: &str) -> AtlasArt {
    let skin = active_skin();
    let region = resolve_region(name, skin)
        .unwrap_or_else(|| panic!("atlas {name} has no member under {skin:?}"));
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("atlas {name} must resolve to a DB2 FileDataID under {skin:?}")
    };
    art(
        fdid,
        (1.0, 1.0),
        (region.left, region.right, region.top, region.bottom),
    )
}

pub fn named_atlas_texture(name: String, atlas: &str, rect: (f32, f32, f32, f32)) -> Element {
    atlas_texture(name, &read_active_atlas_art(atlas), rect)
}

/// Retail `ButtonFrameTemplate` chrome for a window of `width`×`height`: rock background,
/// top tile streaks, the `metal_frame` NineSlice with the portrait ring, title text and close button
/// (`<prefix>CloseButton`, `onclick = close_action`).
pub fn window_chrome(
    prefix: &str,
    (width, height): (f32, f32),
    title: &str,
    close_action: &str,
) -> Element {
    let mut elements = window_background(prefix, width, height);
    elements.extend(portrait_border(
        prefix,
        (width, height),
        title,
        close_action,
    ));
    elements
}

/// `PortraitFrameTemplate` without a background (the window draws its own, like
/// `FlightMapFrame`): the portrait `metal_frame` border, title and close button.
pub fn portrait_border(
    prefix: &str,
    (width, height): (f32, f32),
    title: &str,
    close_action: &str,
) -> Element {
    let mut elements = metal_border(prefix, width, height, MetalTopLeft::Portrait);
    elements.extend(window_title(prefix, width, title, PORTRAIT_TITLE_LEFT));
    elements.extend(close_button(prefix, width, close_action));
    elements
}

/// `PortraitFrameBaseTemplate` `PortraitContainer.portrait` (SharedUIPanelTemplates.xml:
/// 551-572): 62×62 at TOPLEFT (-5, 7), rounded by `CircleMask` (`TempPortraitAlphaMask`,
/// FileDataID 130924) from the portrait's TOPLEFT (2, 0) to its BOTTOMRIGHT (-2, 4).
pub const fn window_portrait_slot(frame: &'static str) -> PortraitSlot {
    PortraitSlot {
        frame,
        rect: (-5.0, -7.0, 62.0, 62.0),
        mask_fdid: 130_924,
        mask_rect: (-3.0, -7.0, 58.0, 58.0),
        tex_coords: FULL_PORTRAIT,
    }
}

/// The slot the host renders `SetPortraitTexture(portrait, unit)` into, at the
/// `PortraitContainer`'s frameLevel 400 above the metal ring.
pub fn window_portrait(slot: &PortraitSlot) -> Element {
    let (x, y, width, height) = slot.rect;
    rsx! {
        r#frame {
            name: {DynName(slot.frame.into())},
            width,
            height,
            frame_level: 400.0,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// `PANEL_BACKGROUND_COLOR` (GlobalColor 191, ARGB 0xCC1F1E21).
const PANEL_BACKGROUND_COLOR: &str = "0.122,0.118,0.129,0.8";
/// `uiframebackground-nineslice-cornerbottomleft` (17053) / `-cornerbottomright`
/// (17054), atlas 2120 `4700695` 64×32, 16×16.
const FLAT_CORNER_BOTTOM_LEFT: AtlasArt = art(4_700_695, (64.0, 32.0), (1.0, 17.0, 1.0, 17.0));
const FLAT_CORNER_BOTTOM_RIGHT: AtlasArt = art(4_700_695, (64.0, 32.0), (19.0, 35.0, 1.0, 17.0));

/// Retail `DefaultPanelFlatTemplate` (SharedUIPanelTemplates.xml:527-536): the
/// `FlatPanelBackgroundTemplate` (:404-436) tinted `PANEL_BACKGROUND_COLOR` at
/// TOPLEFT 6,-20 / BOTTOMRIGHT -2,2, the `ButtonFrameTemplateNoPortrait` metal border,
/// `TitleContainer` (30,-1)..(-24,-1) and the close button.
pub fn flat_panel_chrome(
    prefix: &str,
    (width, height): (f32, f32),
    title: &str,
    close_action: &str,
) -> Element {
    let mut elements = flat_background(prefix, (6.0, 20.0, width - 2.0, height - 2.0));
    elements.extend(metal_border(prefix, width, height, MetalTopLeft::Plain));
    elements.extend(window_title(prefix, width, title, FLAT_TITLE_LEFT));
    elements.extend(close_button(prefix, width, close_action));
    elements
}

/// Retail `PortraitFrameFlatTemplate` (SharedUIPanelTemplates.xml:618-627,643-647): the
/// `FlatPanelBackgroundTemplate` at TOPLEFT 2,-20 / BOTTOMRIGHT -2,3, the portrait metal
/// border, `TitleContainer` from `title_left` (`SetTitleOffsets`) and the close button.
pub fn portrait_flat_chrome(
    prefix: &str,
    (width, height): (f32, f32),
    (title, title_left): (&str, f32),
    close_action: &str,
) -> Element {
    let mut elements = flat_background(prefix, (2.0, 20.0, width - 2.0, height - 3.0));
    elements.extend(metal_border(prefix, width, height, MetalTopLeft::Portrait));
    elements.extend(window_title(prefix, width, title, title_left));
    elements.extend(close_button(prefix, width, close_action));
    elements
}

/// `FlatPanelBackgroundTemplate` (SharedUIPanelTemplates.xml:404-436) over the window
/// rect `(left, top, right, bottom)`.
fn flat_background(prefix: &str, (left, top, right, bottom): (f32, f32, f32, f32)) -> Element {
    let corner = 16.0;
    let fill = |name: &str, rect: (f32, f32, f32, f32)| {
        let (x, y, w, h) = rect;
        rsx! {
            r#frame {
                name: {DynName(format!("{prefix}Bg{name}"))},
                width: w,
                height: h,
                background_color: PANEL_BACKGROUND_COLOR,
                pos_type: "absolute",
                left: x,
                top: y,
            }
        }
    };
    let mut elements = fill(
        "TopSection",
        (left, top, right - left, bottom - corner - top),
    );
    elements.extend(fill(
        "BottomEdge",
        (
            left + corner,
            bottom - corner,
            right - left - 2.0 * corner,
            corner,
        ),
    ));
    for (name, art, x) in [
        ("BottomLeft", &FLAT_CORNER_BOTTOM_LEFT, left),
        ("BottomRight", &FLAT_CORNER_BOTTOM_RIGHT, right - corner),
    ] {
        let coords = art.tex_coords(1.0);
        elements.extend(rsx! {
            texture {
                name: {DynName(format!("{prefix}Bg{name}"))},
                width: corner,
                height: corner,
                texture_fdid: {art.fdid},
                tex_coords: {coords.as_str()},
                vertex_color: PANEL_BACKGROUND_COLOR,
                pos_type: "absolute",
                left: x,
                top: {bottom - corner},
            }
        });
    }
    elements
}

fn window_background(prefix: &str, width: f32, height: f32) -> Element {
    let mut elements = rsx! {
        texture {
            name: {DynName(format!("{prefix}Bg"))},
            width: {width - 4.0},
            height: {height - 23.0},
            texture_fdid: WINDOW_BACKGROUND,
            pos_type: "absolute",
            left: 2.0,
            top: 21.0,
        }
    };
    elements.extend(named_atlas_texture(
        format!("{prefix}TopTileStreaks"),
        TOP_TILE_STREAKS,
        (6.0, 21.0, width - 8.0, 43.0),
    ));
    elements
}

/// The `layout` metal border on a frame [`MetalTopLeft::outset`] larger than the window.
pub(crate) fn metal_border(prefix: &str, width: f32, height: f32, layout: MetalTopLeft) -> Element {
    let [left, top, right, bottom] = layout.outset();
    rsx! {
        r#frame {
            name: {DynName(format!("{prefix}NineSlice"))},
            width: {width + left + right},
            height: {height + top + bottom},
            style: {layout.style_name()},
            pos_type: "absolute",
            left: {-left},
            top: {-top},
        }
    }
}

/// `TitleContainer` left inset: 58 beside a portrait, 30 without.
const PORTRAIT_TITLE_LEFT: f32 = 58.0;
const FLAT_TITLE_LEFT: f32 = 30.0;

/// `TitleContainer` (left, -1)..(-24, -1), `GameFontNormal` centred 5 below its top.
fn window_title(prefix: &str, width: f32, title: &str, left: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{prefix}TitleText"))},
            width: {width - left - 24.0},
            height: 14.0,
            text: title,
            font: GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: NORMAL_FONT_COLOR,
            shadow_color: "0.0,0.0,0.0,1.0",
            shadow_offset: "1,-1",
            justify_h: "CENTER",
            pos_type: "absolute",
            left: left,
            top: 6.0,
        }
    }
}

/// `UIPanelCloseButtonDefaultAnchors`: 24×24 at TOPRIGHT (+1, 0).
fn close_button(prefix: &str, width: f32, action: &str) -> Element {
    let art = read_active_atlas_art(CLOSE_BUTTON);
    let coords = art.tex_coords(1.0);
    rsx! {
        button {
            name: {DynName(format!("{prefix}CloseButton"))},
            width: 24.0,
            height: 24.0,
            onclick: action,
            pos_type: "absolute",
            left: {width + 1.0 - 24.0},
            top: 0.0,
            texture {
                name: {DynName(format!("{prefix}CloseButtonNormal"))},
                width: 24.0,
                height: 24.0,
                texture_fdid: {art.fdid},
                tex_coords: {coords.as_str()},
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
        }
    }
}

/// `UIPanelButtonTemplate` (Retail default nine-slice button art as used by StaticPopup).
/// A disabled button shows the disabled art and carries no action.
pub fn panel_button(
    name: String,
    text: &str,
    action: &str,
    enabled: bool,
    rect: (f32, f32, f32, f32),
) -> Element {
    let (x, y, width, height) = rect;
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    rsx! {
        button {
            name: {DynName(name)},
            width,
            height,
            text,
            font_size: 12.0,
            onclick: action,
            disabled,
            strata: FrameStrata::Dialog,
            button_atlas_up: "defaultbutton-nineslice-up",
            button_atlas_pressed: "defaultbutton-nineslice-pressed",
            button_atlas_highlight: "defaultbutton-nineslice-highlight",
            button_atlas_disabled: "defaultbutton-nineslice-disabled",
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// Lines `text` takes when word-wrapped at `width` (explicit `\n` breaks kept).
pub fn wrapped_line_count(text: &str, width: f32, font_size: f32) -> usize {
    if text.is_empty() {
        return 0;
    }
    text.split('\n')
        .map(|paragraph| paragraph_line_count(paragraph, width, font_size))
        .sum()
}

/// Greedy word wrap measuring the original text between breaks, so runs of spaces
/// (quest texts use two after a full stop) count like the renderer lays them out.
fn paragraph_line_count(paragraph: &str, width: f32, font_size: f32) -> usize {
    let measure = |s: &str| {
        measure_text(s, GameFont::FrizQuadrata, font_size)
            .expect("FrizQuadrata text measurement")
            .0
    };
    let word_ends: Vec<usize> = paragraph
        .char_indices()
        .filter(|(_, c)| *c == ' ')
        .map(|(i, _)| i)
        .chain(std::iter::once(paragraph.len()))
        .collect();
    let mut lines = 1;
    let mut line_start = 0;
    let mut last_fit: Option<usize> = None;
    for end in word_ends {
        if paragraph[line_start..end].trim().is_empty() {
            continue;
        }
        if measure(paragraph[line_start..end].trim_start()) <= width {
            last_fit = Some(end);
        } else if let Some(fit) = last_fit {
            lines += 1;
            line_start = fit;
            last_fit = Some(end);
        }
    }
    lines
}

/// Height of wrapped text at the font's measured line height.
pub fn wrapped_text_height(text: &str, width: f32, font_size: f32) -> f32 {
    wrapped_line_count(text, width, font_size) as f32 * line_height(font_size)
}

/// Frame height for a wrapped fontstring. The toolkit centres the first line in the
/// frame (`JustifyV::Middle`, no `justify_v` attribute) and lets wrapped lines flow
/// below it unclipped, so a one-line frame top-aligns the text like Retail's
/// `justifyV="TOP"`; callers advance their layout by [`wrapped_text_height`].
pub fn line_height(font_size: f32) -> f32 {
    measure_text("Ag", GameFont::FrizQuadrata, font_size)
        .expect("FrizQuadrata text measurement")
        .1
}
