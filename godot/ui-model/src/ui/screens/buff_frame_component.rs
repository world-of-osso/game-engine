//! Player BuffFrame and DebuffFrame: Retail edit-mode defaults, top right, left of the
//! minimap. Every number cites `retail/AddOns` in the Blizzard UI tree; see
//! `docs/specs/buff-frame.md`.

use std::fmt;

use crate::aura_display_data::{AuraInstance, DebuffType};
use crate::hud_layout::{HudAnchor, hud_layout};
use crate::ui::anchor::FrameName;
use crate::ui::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const BUFF_FRAME: FrameName = FrameName("BuffFrame");
pub const DEBUFF_FRAME: FrameName = FrameName("DebuffFrame");
/// `BUFF_MAX_DISPLAY` / `DEBUFF_MAX_DISPLAY` (BuffFrame.lua:2-3).
pub const MAX_BUFFS: usize = 32;
pub const MAX_DEBUFFS: usize = 16;
/// Edit-mode `IconLimitBuffFrame` 11 / `IconLimitDebuffFrame` 8 (EditModePresetLayouts.lua:421,438).
pub const BUFFS_PER_ROW: usize = 11;
pub const DEBUFFS_PER_ROW: usize = 8;

const BUFF_BUTTON: &str = "BuffButton";
const DEBUFF_BUTTON: &str = "DebuffButton";

/// Horizontal aura button 30×40 (BuffFrame.lua:139-140), icon 30×30 at its top
/// (BuffFrameTemplates.xml:5-11).
const AURA_W: f32 = 30.0;
const AURA_H: f32 = 40.0;
const ICON_SIZE: f32 = 30.0;
/// Edit-mode `IconPadding` 5 (EditModePresetLayouts.lua:423,440) between grid cells.
const ICON_PADDING: f32 = 5.0;
/// The 15-wide CollapseAndExpandButton sits at BuffFrame's TOPRIGHT and the aura container
/// hangs off its TOPLEFT (BuffFrame.xml:14-17, BuffFrame.lua:531-534), hidden or not.
const COLLAPSE_BUTTON_W: f32 = 15.0;

/// `DebuffBorder` 40×40 centred on the icon (BuffFrameTemplates.xml:20-25).
const DEBUFF_BORDER_SIZE: f32 = 40.0;
/// `UiTextureAtlas` 3726, `interface/hud/uidebuffframes.blp`, 256×128.
const DEBUFF_BORDER_ATLAS_FDID: u32 = 7_553_349;
const DEBUFF_BORDER_ATLAS: (f32, f32) = (256.0, 128.0);

/// `GameFontNormalSmall`: FRIZQT 10 with a 1,-1 shadow (FontStyles.xml:56, Fonts.xml:39-45).
const DURATION_FONT_SIZE: f32 = 10.0;
/// `NORMAL_FONT_COLOR`, and `HIGHLIGHT_FONT_COLOR` once under `BUFF_DURATION_WARNING_TIME`
/// (BuffFrame.lua `UpdateDuration`).
const DURATION_COLOR: &str = "1.0,0.82,0.0,1.0";
const DURATION_WARNING_COLOR: &str = "1.0,1.0,1.0,1.0";
/// `NumberFontNormal`: ARIALN 14 outline, white (FontStyles.xml:389, Fonts.xml:874-876).
const COUNT_FONT_SIZE: f32 = 14.0;
/// `TextStatusBarText`: FRIZQT 10 outline, white (GameFontStyles.xml:69, GameFonts.xml:4-6).
const SYMBOL_FONT_SIZE: f32 = 10.0;
const WHITE: &str = "1.0,1.0,1.0,1.0";
const TEXT_SHADOW: &str = "0.0,0.0,0.0,1.0";

/// `BUFF_DURATION_WARNING_TIME` (AuraUtil.lua:2): duration text turns white below it.
pub const BUFF_DURATION_WARNING_TIME: f32 = 90.0;
/// `BUFF_WARNING_TIME` (BuffFrame.lua:1): buttons flash below it.
pub const BUFF_WARNING_TIME: f32 = 31.0;
/// AuraContainerTemplate flash: period 1.5 s bouncing alpha 0.3..1.0
/// (BuffFrameTemplates.xml:69-71; BuffFrame.lua:37-49).
const WARNING_FLASH_PERIOD: f32 = 1.5;
const WARNING_MIN_ALPHA: f32 = 0.3;
const WARNING_MAX_ALPHA: f32 = 1.0;

#[derive(Clone, Debug, PartialEq)]
pub struct BuffIconState {
    pub icon_fdid: u32,
    pub timer_text: String,
    /// Under `BUFF_DURATION_WARNING_TIME`: white duration text.
    pub timer_warning: bool,
    /// Stack count (0 or 1 = hide).
    pub stacks: u32,
    /// Debuffs only: dispel type selecting the border atlas.
    pub dispel: Option<DebuffType>,
    /// Colorblind dispel abbreviation (`AuraUtil.SetAuraSymbol`); empty otherwise.
    pub symbol: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuffFrameState {
    pub buffs: Vec<BuffIconState>,
    pub debuffs: Vec<BuffIconState>,
}

impl BuffFrameState {
    /// Icons in replicated order, so button index `i` is the `i`th buff (or debuff) of
    /// `auras`.
    pub fn from_auras(auras: &[AuraInstance], colorblind_mode: bool) -> Self {
        let icon = |aura: &AuraInstance| BuffIconState {
            icon_fdid: aura.icon_fdid,
            timer_text: aura.timer_text(),
            timer_warning: !aura.is_permanent() && aura.remaining < BUFF_DURATION_WARNING_TIME,
            stacks: aura.stacks,
            dispel: aura.is_debuff.then_some(aura.debuff_type),
            symbol: if aura.is_debuff && colorblind_mode {
                dispel_symbol(aura.debuff_type)
            } else {
                ""
            },
        };
        Self {
            buffs: auras
                .iter()
                .filter(|aura| !aura.is_debuff)
                .take(MAX_BUFFS)
                .map(icon)
                .collect(),
            debuffs: auras
                .iter()
                .filter(|aura| aura.is_debuff)
                .take(MAX_DEBUFFS)
                .map(icon)
                .collect(),
        }
    }
}

/// Textures the host must copy from local CASC before drawing the buff frame.
pub fn buff_frame_texture_fdids(_state: &BuffFrameState) -> Vec<u32> {
    Vec::new()
}

/// enUS `DEBUFF_SYMBOL_*` global strings; `None` has no abbreviation (AuraUtil.lua:5-10).
fn dispel_symbol(dispel: DebuffType) -> &'static str {
    match dispel {
        DebuffType::None => "",
        DebuffType::Magic => "Ma",
        DebuffType::Curse => "Cu",
        DebuffType::Disease => "Di",
        DebuffType::Poison => "Po",
    }
}

/// `ui-debuff-border-*` member rect `(left, right, top, bottom)` in the 256×128 atlas
/// (UiTextureAtlasMember 35477-35484). DebuffFrame shows dispel types (edit-mode
/// `ShowDispelType` 1, EditModePresetLayouts.lua:441), so typed debuffs use the `-icon`
/// member and untyped ones `default-noicon` (Mainline/AuraUtil.lua:20-23).
fn debuff_border_rect(dispel: DebuffType) -> (f32, f32, f32, f32) {
    match dispel {
        DebuffType::None => (43.0, 83.0, 43.0, 83.0),
        DebuffType::Magic => (85.0, 125.0, 43.0, 83.0),
        DebuffType::Curse => (1.0, 41.0, 85.0, 125.0),
        DebuffType::Disease => (43.0, 83.0, 85.0, 125.0),
        DebuffType::Poison => (127.0, 167.0, 1.0, 41.0),
    }
}

/// Normalised `left,right,top,bottom` for a debuff border.
pub fn debuff_border_tex_coords(dispel: DebuffType) -> String {
    let (left, right, top, bottom) = debuff_border_rect(dispel);
    let (width, height) = DEBUFF_BORDER_ATLAS;
    format!(
        "{},{},{},{}",
        left / width,
        right / width,
        top / height,
        bottom / height
    )
}

/// Alpha of an aura button at `clock` seconds: bouncing between 0.3 and 1.0 over a
/// 1.5 s period while under `BUFF_WARNING_TIME`, opaque otherwise or when permanent
/// (`AuraContainerMixin:GetAuraWarningAlphaForDuration`, BuffFrame.lua:56-62).
pub fn aura_warning_alpha(clock: f32, time_left: Option<f32>) -> f32 {
    if !time_left.is_some_and(|left| left < BUFF_WARNING_TIME) {
        return WARNING_MAX_ALPHA;
    }
    let half = WARNING_FLASH_PERIOD / 2.0;
    let phase = clock.rem_euclid(WARNING_FLASH_PERIOD);
    let progress = if phase < half {
        phase / half
    } else {
        2.0 - phase / half
    };
    WARNING_MIN_ALPHA + (WARNING_MAX_ALPHA - WARNING_MIN_ALPHA) * progress
}

/// Registry name of buff (`false`) or debuff (`true`) button `index`.
pub fn aura_button_name(is_debuff: bool, index: usize) -> String {
    let prefix = if is_debuff {
        DEBUFF_BUTTON
    } else {
        BUFF_BUTTON
    };
    format!("{prefix}{index}")
}

/// A buff-frame button under the cursor: `(is_debuff, index)`.
pub fn buff_button_at(registry: &FrameRegistry, mut frame_id: u64) -> Option<(bool, usize)> {
    loop {
        let frame = registry.get(frame_id)?;
        if let Some(hit) = frame.name.as_deref().and_then(parse_button_name) {
            return Some(hit);
        }
        frame_id = frame.parent_id?;
    }
}

fn parse_button_name(name: &str) -> Option<(bool, usize)> {
    let (is_debuff, rest) = match name.strip_prefix(BUFF_BUTTON) {
        Some(rest) => (false, rest),
        None => (true, name.strip_prefix(DEBUFF_BUTTON)?),
    };
    rest.parse().ok().map(|index| (is_debuff, index))
}

/// One aura grid: its frame size counts every slot, shown or not (`AuraFrameMixin:UpdateSize`,
/// BuffFrame.lua:251-273).
struct AuraGrid {
    per_row: usize,
    max: usize,
    /// Right inset of the first column inside the frame.
    inset_right: f32,
}

impl AuraGrid {
    const BUFFS: Self = Self {
        per_row: BUFFS_PER_ROW,
        max: MAX_BUFFS,
        inset_right: COLLAPSE_BUTTON_W,
    };
    const DEBUFFS: Self = Self {
        per_row: DEBUFFS_PER_ROW,
        max: MAX_DEBUFFS,
        inset_right: 0.0,
    };

    fn width(&self) -> f32 {
        (AURA_W + ICON_PADDING) * self.per_row as f32 + self.inset_right
    }

    fn height(&self) -> f32 {
        (AURA_H + ICON_PADDING) * self.max.div_ceil(self.per_row) as f32
    }

    /// Icons grow left from the top right, wrapping down (`IconDirection` Left, `IconWrap`
    /// Down, EditModePresetLayouts.lua:419-420).
    fn cell(&self, index: usize) -> (f32, f32) {
        let right = self.inset_right + (index % self.per_row) as f32 * (AURA_W + ICON_PADDING);
        let top = (index / self.per_row) as f32 * (AURA_H + ICON_PADDING);
        (right, top)
    }
}

pub fn buff_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<BuffFrameState>()
        .expect("BuffFrameState must be in SharedContext");
    let layout = hud_layout(ctx);
    let buffs = aura_frame(
        BUFF_FRAME,
        &AuraGrid::BUFFS,
        &layout.buffs,
        &state.buffs,
        false,
    );
    let debuffs = aura_frame(
        DEBUFF_FRAME,
        &AuraGrid::DEBUFFS,
        &layout.debuffs,
        &state.debuffs,
        true,
    );
    buffs.into_iter().chain(debuffs).collect()
}

fn aura_frame(
    name: FrameName,
    grid: &AuraGrid,
    anchor: &HudAnchor,
    icons: &[BuffIconState],
    is_debuff: bool,
) -> Element {
    let buttons: Element = icons
        .iter()
        .enumerate()
        .flat_map(|(i, icon)| aura_button(aura_button_name(is_debuff, i), grid.cell(i), icon))
        .collect();
    let at = anchor.place((grid.width(), grid.height()));
    rsx! {
        r#frame {
            name: name,
            width: {grid.width()},
            height: {grid.height()},
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {buttons}
        }
    }
}

fn aura_button(name: String, (right, top): (f32, f32), icon: &BuffIconState) -> Element {
    let count = if icon.stacks > 1 {
        icon.stacks.to_string()
    } else {
        String::new()
    };
    let duration_color = if icon.timer_warning {
        DURATION_WARNING_COLOR
    } else {
        DURATION_COLOR
    };
    let border: Element = icon
        .dispel
        .map(|dispel| debuff_border(&name, dispel))
        .unwrap_or_default();
    rsx! {
        r#frame {
            name: {DynName(name.clone())},
            width: AURA_W,
            height: AURA_H,
            mouse_enabled: true,
            pos_type: "absolute",
            right: right,
            top: top,
            texture {
                name: {DynName(format!("{name}Icon"))},
                width: ICON_SIZE,
                height: ICON_SIZE,
                texture_fdid: {icon.icon_fdid},
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
            {border}
            // `Symbol` TOPLEFT 2,-2 (BuffFrameTemplates.xml:32-36).
            fontstring {
                name: {DynName(format!("{name}Symbol"))},
                width: {ICON_SIZE - 2.0},
                height: 12.0,
                text: {icon.symbol},
                font: "FrizQuadrata",
                font_size: SYMBOL_FONT_SIZE,
                font_color: WHITE,
                outline: "OUTLINE",
                justify_h: "LEFT",
                pos_type: "absolute",
                left: 2.0,
                top: 2.0,
            }
            // `Count` BOTTOMRIGHT of the icon at -2,2 (BuffFrameTemplates.xml:37-41).
            fontstring {
                name: {DynName(format!("{name}Count"))},
                width: {ICON_SIZE - 2.0},
                height: 14.0,
                text: {count.as_str()},
                font: "ArialNarrow",
                font_size: COUNT_FONT_SIZE,
                font_color: WHITE,
                outline: "OUTLINE",
                justify_h: "RIGHT",
                pos_type: "absolute",
                right: 2.0,
                top: {ICON_SIZE - 2.0 - 14.0},
            }
            // `Duration` TOP on the icon's BOTTOM (BuffFrameTemplates.xml:13-17).
            fontstring {
                name: {DynName(format!("{name}Duration"))},
                width: {AURA_W + ICON_PADDING * 2.0},
                height: {AURA_H - ICON_SIZE},
                text: {icon.timer_text.as_str()},
                font: "FrizQuadrata",
                font_size: DURATION_FONT_SIZE,
                font_color: duration_color,
                shadow_color: TEXT_SHADOW,
                shadow_offset: "1,-1",
                justify_h: "CENTER",
                pos_type: "absolute",
                left: {-ICON_PADDING},
                top: ICON_SIZE,
            }
        }
    }
}

fn debuff_border(button: &str, dispel: DebuffType) -> Element {
    let coords = debuff_border_tex_coords(dispel);
    let inset = (DEBUFF_BORDER_SIZE - ICON_SIZE) / 2.0;
    rsx! {
        texture {
            name: {DynName(format!("{button}Border"))},
            width: DEBUFF_BORDER_SIZE,
            height: DEBUFF_BORDER_SIZE,
            texture_fdid: DEBUFF_BORDER_ATLAS_FDID,
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            left: {-inset},
            top: {-inset},
        }
    }
}

