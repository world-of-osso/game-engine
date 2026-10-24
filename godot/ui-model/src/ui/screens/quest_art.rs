//! Retail art and layout helpers shared by the quest screens (objective tracker, quest
//! log, quest giver frame). Atlas constants cite `UiTextureAtlasMember.CommittedName`
//! (member id) and the `UiTextureAtlas` texture FileDataID.

use std::fmt;

use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region, thread_skin};
use ui_toolkit::rsx;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::widgets::texture::BlendMode;

use crate::panel_style_data::MetalTopLeft;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::screens::inworld_unit_frames_component::{FULL_PORTRAIT, PortraitSlot};
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
/// `ui-questtracker-objective-nub` (23702), Scenario's incomplete objective.
pub const TRACKER_NUB: AtlasArt = tracker((492.0, 511.0, 1.0, 20.0));
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
/// `worldquest-icon` (27230), atlas743; QuestUtil's normal WorldQuest POI, 32×32.
pub const POI_WORLD_QUEST: AtlasArt = art(1_339_312, (512.0, 512.0), (334.0, 366.0, 400.0, 432.0));
/// `UI-QuestPoi-InnerGlow` (23600), atlas2549; POIButton.xml uses additive blending.
pub const POI_INNER_GLOW: AtlasArt = art(5_320_914, (256.0, 128.0), (1.0, 33.0, 67.0, 99.0));
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
    atlas_texture_blend(name, art, rect, BlendMode::Blend)
}

/// One region constructor; omission uses the authored BLEND default above.
pub fn atlas_texture_blend(
    name: String,
    art: &AtlasArt,
    rect: (f32, f32, f32, f32),
    mode: BlendMode,
) -> Element {
    let blend_mode = mode.as_str();
    let coords = art.tex_coords(1.0);
    let (x, y, width, height) = rect;
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            blend_mode,
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// Resolve active-skin art into the existing FDID/UV representation, retaining Modern tree bytes.
fn read_active_atlas_art(name: &str) -> AtlasArt {
    let skin = thread_skin();
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
    named_atlas_texture_blend(name, atlas, rect, BlendMode::Blend)
}

pub fn named_atlas_texture_blend(
    name: String,
    atlas: &str,
    rect: (f32, f32, f32, f32),
    mode: BlendMode,
) -> Element {
    atlas_texture_blend(name, &read_active_atlas_art(atlas), rect, mode)
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

/// Retail `BasicFrameTemplate`: rectangular metal corner, no portrait well.
pub fn window_chrome_no_portrait(
    prefix: &str,
    (width, height): (f32, f32),
    title: &str,
    close_action: &str,
) -> Element {
    let mut elements = window_background(prefix, width, height);
    elements.extend(metal_border(prefix, width, height, MetalTopLeft::Plain));
    elements.extend(window_title(prefix, width, title, FLAT_TITLE_LEFT));
    elements.extend(close_button(prefix, width, close_action));
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

/// Static `SetPortraitToAsset` art in the same mask bounds as a live portrait.
/// The native icon-mask pass rounds these texture frames.
pub fn window_portrait_texture(slot: &PortraitSlot, fdid: u32) -> Element {
    let (left, top, width, height) = slot.mask_rect;
    rsx! {
        texture {
            name: {DynName(slot.frame.into())},
            width,
            height,
            frame_level: 400.0,
            texture_fdid: fdid,
            pos_type: "absolute",
            left,
            top,
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

/// Launcher-only taller title strip. Shared window chrome above is unchanged.
pub fn launcher_panel_chrome((width, height): (f32, f32), header_height: f32) -> Element {
    let mut elements = flat_background("Launcher", (6.0, 20.0, width - 2.0, height - 2.0));
    elements.extend(metal_border("Launcher", width, height, MetalTopLeft::Plain));
    elements.extend(rsx! {
        r#frame { name: "LauncherHeaderBar", width, height: header_height,
            pos_type: "absolute", left: 0.0, top: 0.0,
            {launcher_header_art(width, header_height)}
            {launcher_header_controls(width, header_height)}
        }
    });
    elements
}

fn launcher_header_art(width: f32, height: f32) -> Element {
    let corner_width = match thread_skin() {
        ActiveSkin::Modern => 75.0,
        ActiveSkin::Forever => 95.0,
    };
    let [left, _, right, _] = MetalTopLeft::Plain.outset();
    let middle_x = corner_width - left;
    let middle_width = width + left + right - corner_width * 2.0;
    let right_x = width + right - corner_width;
    [
        ("Left", "UI-Frame-Metal-CornerTopLeft", -left, corner_width),
        ("Middle", "_UI-Frame-Metal-EdgeTop", middle_x, middle_width),
        (
            "Right",
            "UI-Frame-Metal-CornerTopRight",
            right_x,
            corner_width,
        ),
    ]
    .into_iter()
    .flat_map(|(name, atlas, x, span)| {
        let art = read_launcher_title_band(atlas);
        atlas_texture(
            format!("LauncherHeader{name}"),
            &art,
            (x, 0.0, span, height),
        )
    })
    .collect()
}

fn read_launcher_title_band(atlas: &str) -> AtlasArt {
    // Existing bevel/end-cap pixels, relative to the active atlas member's top.
    let (source_height, band_top, band_bottom) = match thread_skin() {
        ActiveSkin::Modern => (150.0, 32.0, 80.0),
        ActiveSkin::Forever => (190.0, 26.0, 78.0),
    };
    let mut art = read_active_atlas_art(atlas);
    let member_top = art.rect.2;
    let member_height = art.rect.3 - member_top;
    art.rect.2 = member_top + member_height * band_top / source_height;
    art.rect.3 = member_top + member_height * band_bottom / source_height;
    art
}

fn launcher_header_controls(width: f32, height: f32) -> Element {
    let close_size = 24.0;
    rsx! {
        fontstring { name: "LauncherTitleText", width: {width - FLAT_TITLE_LEFT - 24.0}, height,
            text: "Launcher", font: GameFont::FrizQuadrata, font_size: 20.0,
            font_color: NORMAL_FONT_COLOR, shadow_color: "0.0,0.0,0.0,1.0", shadow_offset: "1,-1",
            justify_h: "CENTER", justify_v: "MIDDLE",
            pos_type: "absolute", left: FLAT_TITLE_LEFT, top: 0.0 }
        button { name: "LauncherCloseButton", width: close_size, height: close_size,
            onclick: crate::launcher::ACTION_CLOSE, pos_type: "absolute",
            left: {width - close_size - 4.0}, top: {(height - close_size) / 2.0},
            button_template: "UIPanelCloseButton",
        }
    }
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
    let streak = read_active_atlas_art(TOP_TILE_STREAKS);
    let streak_coords = streak.tex_coords(1.0);
    // SharedUIPanelTemplates.xml:598–609,663–675: one rock region preserves the
    // tile origin under the portrait; splitting it at y51 would restart the pattern.
    rsx! {
        texture {
            name: {DynName(format!("{prefix}Bg"))},
            width: {width - 4.0},
            height: {height - 23.0},
            texture_fdid: WINDOW_BACKGROUND,
            horiz_tile: true,
            vert_tile: true,
            pos_type: "absolute",
            left: 2.0,
            top: 21.0,
        }
        // Source _UI-Frame-TopTileStreaks:256×43, horizontal repeat only (:76–78).
        texture {
            name: {DynName(format!("{prefix}TopTileStreaks"))},
            width: {width - 8.0},
            height: 43.0,
            texture_fdid: {streak.fdid},
            tex_coords: {streak_coords.as_str()},
            horiz_tile: true,
            pos_type: "absolute",
            left: 6.0,
            top: 21.0,
        }
    }
}

/// The `layout` metal border on a frame [`MetalTopLeft::outset`] larger than the window.
pub(crate) fn metal_border(prefix: &str, width: f32, height: f32, layout: MetalTopLeft) -> Element {
    let [left, top, right, bottom] = layout.outset();
    let mut elements = rsx! {
        r#frame {
            name: {DynName(format!("{prefix}NineSlice"))},
            width: {width + left + right},
            height: {height + top + bottom},
            style: {layout.style_name()},
            pos_type: "absolute",
            left: {-left},
            top: {-top},
        }
    };
    if layout == MetalTopLeft::Portrait {
        elements.extend(portrait_ring(prefix));
    }
    elements
}

/// NineSliceLayouts.lua:20 OVERLAY portrait corner. Draw separately from the low-level
/// background border so the masked 3D portrait cannot paint over the gold rim.
fn portrait_ring(prefix: &str) -> Element {
    let art = read_active_atlas_art("UI-Frame-PortraitMetal-CornerTopLeft");
    let coords = art.tex_coords(1.0);
    let region = resolve_region("UI-Frame-PortraitMetal-CornerTopLeft", thread_skin())
        .expect("portrait metal corner atlas");
    rsx! { texture {
        name: {DynName(format!("{prefix}PortraitRing"))},
        width: {region.width / 2.0},
        height: {region.height / 2.0},
        frame_level: 401.0,
        texture_fdid: {art.fdid},
        tex_coords: {coords.as_str()},
        pos_type: "absolute",
        left: -13.0,
        top: -16.0,
    } }
}

/// `TitleContainer` left inset: 58 beside a portrait, 30 without.
const PORTRAIT_TITLE_LEFT: f32 = 58.0;
const FLAT_TITLE_LEFT: f32 = 30.0;

/// `TitleContainer` (left, -1)..(-24, -1), `GameFontNormal` centred 5 below its top.
/// Retail SharedUIPanelTemplates.xml:576 puts the title at frameLevel 510, above
/// the portrait ring (401); narrow containers otherwise lose their leading glyphs.
fn window_title(prefix: &str, width: f32, title: &str, left: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{prefix}TitleText"))},
            frame_level: 510.0,
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

/// `UIPanelCloseButtonDefaultAnchors` TOPRIGHT offset as (right, down): Retail `(1, 0)`
/// (Mainline/SharedUIPanelTemplates.lua:139-141), Forever `(-2, 1)` with WoW's y up, so
/// one pixel above the top (wowforever Camelot/SharedUIPanelTemplates.lua:3-5).
fn close_button_offset() -> (f32, f32) {
    match thread_skin() {
        ActiveSkin::Forever => (-2.0, -1.0),
        ActiveSkin::Modern => (1.0, 0.0),
    }
}

/// `UIPanelCloseButtonDefaultAnchors`: 24×24 at the skin's TOPRIGHT offset.
pub fn close_button(prefix: &str, width: f32, action: &str) -> Element {
    let (offset_x, offset_top) = close_button_offset();
    rsx! {
        button {
            name: {DynName(format!("{prefix}CloseButton"))},
            width: 24.0,
            height: 24.0,
            onclick: action,
            pos_type: "absolute",
            left: {width + offset_x - 24.0},
            top: offset_top,
            button_template: "UIPanelCloseButton",
        }
    }
}

/// Retail `UIButtonFitToTextBehaviorMixin:FitToText` with the inherited
/// `fitTextCanWidthDecrease=true`, `fitTextWidthPadding=40` defaults
/// (Blizzard_SharedXML/SecureUIPanelTemplates.lua:83-91, .xml:39-43).
/// Only callers that use Retail's SetTextToFit should opt into this width.
pub fn panel_button_fit_to_text_width(text: &str) -> f32 {
    measure_text(text, GameFont::FrizQuadrata, 12.0)
        .expect("UIPanelButton GameFontNormal metrics")
        .0
        + 40.0
}

/// SharedButtonSmallTemplate:138×28, inheriting BigRedThreeSliceButtonTemplate.
/// Core expands the named template to its authored three slices.
pub fn shared_button_small(
    name: &str,
    text: &str,
    action: &str,
    enabled: bool,
    (left, top): (f32, f32),
) -> Element {
    let disabled = !enabled;
    let action = if enabled { action } else { "" };
    let font_color = if enabled {
        NORMAL_FONT_COLOR
    } else {
        "0.5,0.5,0.5,1.0"
    };
    rsx! { button {
        name: {DynName(name.into())}, text, width: 138., height: 28.,
        left, top, pos_type: "absolute", font_size: 12., font_color,
        onclick: action, disabled,
        button_template: "SharedButtonSmallTemplate",
    } }
}

/// `UIPanelButtonTemplate`: SecureUIPanelTemplates.xml:39–78 sheet strips and highlight.
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
            button_template: "UIPanelButtonTemplate",
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

/// Lines `text` takes when word-wrapped at `width` (explicit `\n` breaks kept).
pub fn wrapped_line_count(text: &str, width: f32, font_size: f32) -> usize {
    wrapped_line_count_with_font(text, width, GameFont::FrizQuadrata, font_size)
}

/// The same wrapping calculation for source templates using a different typeface.
pub fn wrapped_line_count_with_font(
    text: &str,
    width: f32,
    font: GameFont,
    font_size: f32,
) -> usize {
    if text.is_empty() {
        return 0;
    }
    text.split('\n')
        .map(|paragraph| paragraph_line_count(paragraph, width, font, font_size))
        .sum()
}

/// Greedy word wrap measuring the original text between breaks, so runs of spaces
/// (quest texts use two after a full stop) count like the renderer lays them out.
fn paragraph_line_count(paragraph: &str, width: f32, font: GameFont, font_size: f32) -> usize {
    let measure = |s: &str| {
        measure_text(s, font, font_size)
            .expect("Authored font text measurement")
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
