//! Dungeon-entrance difficulty bar after Plumber's "Instance Difficulty Selector"
//! (`Modules/RaidCheck/DifficultySelector.lua`; docs/specs/instances.md, Entrance
//! difficulty bar): the instance name above a gold-ruled bar of difficulty buttons,
//! "(5) Normal  (0/3)", with the selected one boxed. It uses Plumber's own art
//! (`Art/RaidCheck/DifficultySelector.png`, authored at 2x). The host owns proximity,
//! animation and the difficulty commands, and measures label widths with the font.

use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::{BlendMode, TextureData};

use crate::dungeon_entrance_data::DifficultyChoice;
use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

struct DynName(String);

pub const ENTRANCE_BAR_ROOT: FrameName = FrameName("EntranceDifficultyFrame");
pub const ENTRANCE_BAR_TITLE: FrameName = FrameName("EntranceDifficultyTitle");
pub const ENTRANCE_BAR_BOX: FrameName = FrameName("EntranceDifficultyBox");
pub const ENTRANCE_BAR_SPINNER: FrameName = FrameName("EntranceDifficultySpinner");
/// Difficulty button click: `entrance_difficulty:<DifficultyID>`.
pub const ACTION_ENTRANCE_DIFFICULTY_PREFIX: &str = "entrance_difficulty:";

pub const ART: &str = "data/reference/plumber/DifficultySelector.png";
pub const SPINNER_ART: &str = "data/reference/plumber/LoadingIndicator32.tga";

/// Plumber `Def`: `DefaultTopOffset` -40, frame 64 tall, `BarTextureHeight` 48,
/// `BarSidePadding` 20, `ButtonMinWidth` 128, `ButtonHeight` 32, `ButtonGap` 4.
const TOP_OFFSET: f32 = 40.0;
const FRAME_HEIGHT: f32 = 64.0;
const BAR_HEIGHT: f32 = 48.0;
const BAR_TOP: f32 = FRAME_HEIGHT - BAR_HEIGHT;
const SIDE_PADDING: f32 = 20.0;
const BUTTON_MIN_WIDTH: f32 = 128.0;
const BUTTON_HEIGHT: f32 = 32.0;
const BUTTON_TOP: f32 = BAR_TOP + (BAR_HEIGHT - BUTTON_HEIGHT) / 2.0;
const BUTTON_GAP: f32 = 4.0;
/// Label padding: `buttonWidth = max(128, round(maxLabelWidth + 32))`.
const LABEL_PADDING: f32 = 32.0;
/// The two spaces of `"%s  (%d/%d)"` at 14 px.
pub const PROGRESS_GAP: f32 = 7.0;
const TITLE_HEIGHT: f32 = 24.0;
const FONT_SIZE: f32 = 14.0;
/// Box caps are 16 px wide and overhang the button by 6 px each side.
const BOX_CAP: f32 = 16.0;
pub const BOX_OVERHANG: f32 = 6.0;
const SHADOW_SIZE: [f32; 2] = [1200.0, 400.0];
const SHADOW_ALPHA: f32 = 0.65;
const HIGHLIGHT_ALPHA: f32 = 0.5;
const SPINNER_SIZE: f32 = 16.0;
/// Draw order inside the opaque layer (level 1): the gold box (2-3) under the buttons and
/// their labels, the spinner over them.
const BUTTON_LEVEL: f32 = 5.0;
const SPINNER_LEVEL: f32 = 8.0;
/// `MotionFrame`: bar width + 64 by 128, centred on the frame.
const MOTION_EXTRA_WIDTH: f32 = 64.0;
const MOTION_HEIGHT: f32 = 128.0;

/// Theme 0 (gold) texture rects `[left, right, top, bottom]` in 1024 px.
const BAR_LEFT_UV: [f32; 4] = [0.0, 40.0, 0.0, 96.0];
const BAR_CENTER_UV: [f32; 4] = [40.0, 216.0, 0.0, 96.0];
const BAR_RIGHT_UV: [f32; 4] = [216.0, 256.0, 0.0, 96.0];
const BOX_LEFT_UV: [f32; 4] = [264.0, 296.0, 0.0, 96.0];
const BOX_CENTER_UV: [f32; 4] = [296.0, 392.0, 0.0, 96.0];
const BOX_RIGHT_UV: [f32; 4] = [392.0, 424.0, 0.0, 96.0];
const HIGHLIGHT_UV: [f32; 4] = [432.0, 560.0, 0.0, 96.0];
const SHADOW_UV: [f32; 4] = [768.0, 1024.0, 0.0, 256.0];

/// `AREA_NAME_FONT_COLOR`.
const TITLE_COLOR: FontColor = FontColor::new(0.9, 0.81, 0.67, 1.0);
const LABEL_GOLD: [f32; 4] = [1.0, 0.82, 0.0, 1.0];
const LABEL_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// One button's choice with its label and count widths at 14 px.
#[derive(Clone, Debug, PartialEq)]
pub struct EntranceBarChoice {
    pub choice: DifficultyChoice,
    pub label_width: f32,
    pub progress_width: f32,
}

/// The loading spinner centred on the clicked button.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntranceBarSpinner {
    pub difficulty_id: u32,
    /// Counter-clockwise radians.
    pub rotation: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntranceBarState {
    pub title: String,
    pub choices: Vec<EntranceBarChoice>,
    pub selected: Option<u32>,
    pub screen_width: f32,
    /// Whole frame fade.
    pub frame_alpha: f32,
    /// Bar, shadow and unselected buttons: 0.5 idle, 1 with the pointer near.
    pub bar_alpha: f32,
    /// Left edge of the gold box in frame coordinates while it slides.
    pub box_left: Option<f32>,
    pub spinner: Option<EntranceBarSpinner>,
}

/// Plumber's geometry for a set of choices on a screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntranceBarLayout {
    pub button_width: f32,
    pub bar_width: f32,
    /// Screen x of the frame's left edge (the bar is centred at the top).
    pub left: f32,
    pub top: f32,
}

impl EntranceBarLayout {
    pub fn new(choices: &[EntranceBarChoice], screen_width: f32) -> Self {
        let widest = choices
            .iter()
            .map(|choice| choice.label_width + PROGRESS_GAP + choice.progress_width)
            .fold(0.0, f32::max);
        let button_width = (widest + LABEL_PADDING).round().max(BUTTON_MIN_WIDTH);
        let count = choices.len() as f32;
        let bar_width = 2.0 * SIDE_PADDING + count * (button_width + BUTTON_GAP) - BUTTON_GAP;
        Self {
            button_width,
            bar_width,
            left: ((screen_width - bar_width) / 2.0).round(),
            top: TOP_OFFSET,
        }
    }

    /// Left edge of button `index` in frame coordinates.
    pub fn button_left(&self, index: usize) -> f32 {
        SIDE_PADDING + index as f32 * (self.button_width + BUTTON_GAP)
    }

    /// Where the gold box rests on button `index`.
    pub fn box_left(&self, index: usize) -> f32 {
        self.button_left(index) - BOX_OVERHANG
    }

    /// Screen `[x, y, w, h]` of Plumber's `MotionFrame`, the hover area that brightens the bar.
    pub fn motion_rect(&self) -> [f32; 4] {
        let width = self.bar_width + MOTION_EXTRA_WIDTH;
        let center = [
            self.left + self.bar_width / 2.0,
            self.top + FRAME_HEIGHT / 2.0,
        ];
        [
            center[0] - width / 2.0,
            center[1] - MOTION_HEIGHT / 2.0,
            width,
            MOTION_HEIGHT,
        ]
    }
}

pub fn button_name(difficulty_id: u32) -> String {
    format!("EntranceDifficultyButton{difficulty_id}")
}

fn part(difficulty_id: u32, part: &str) -> DynName {
    DynName(format!("EntranceDifficulty{part}{difficulty_id}"))
}

fn uv([left, right, top, bottom]: [f32; 4]) -> String {
    format!(
        "{},{},{},{}",
        left / 1024.0,
        right / 1024.0,
        top / 1024.0,
        bottom / 1024.0
    )
}

fn art_piece(name: DynName, rect: [f32; 4], pixels: [f32; 4]) -> Element {
    art_piece_alpha(name, rect, pixels, 1.0)
}

fn art_piece_alpha(name: DynName, rect: [f32; 4], pixels: [f32; 4], alpha: f32) -> Element {
    let [left, top, width, height] = rect;
    rsx! {
        texture {
            name,
            width,
            height,
            texture_file: ART,
            tex_coords: {uv(pixels)},
            alpha,
            pos_type: "absolute",
            left,
            top,
        }
    }
}

/// Three pieces over `[left, top, width, height]`: fixed caps, stretched centre.
fn three_slice(prefix: &str, rect: [f32; 4], cap: f32, pixels: [[f32; 4]; 3]) -> Element {
    let [left, top, width, height] = rect;
    let center = (width - 2.0 * cap).max(0.0);
    [
        art_piece(
            DynName(format!("{prefix}Left")),
            [left, top, cap, height],
            pixels[0],
        ),
        art_piece(
            DynName(format!("{prefix}Center")),
            [left + cap, top, center, height],
            pixels[1],
        ),
        art_piece(
            DynName(format!("{prefix}Right")),
            [left + cap + center, top, cap, height],
            pixels[2],
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn highlight(entry: &EntranceBarChoice, layout: &EntranceBarLayout, index: usize) -> Element {
    let id = entry.choice.difficulty_id;
    let left = layout.button_left(index);
    rsx! {
        texture {
            name: {part(id, "Highlight")},
            width: {layout.button_width},
            height: BAR_HEIGHT,
            texture_file: ART,
            tex_coords: {uv(HIGHLIGHT_UV)},
            alpha: HIGHLIGHT_ALPHA,
            hidden: true,
            pos_type: "absolute",
            left,
            top: BAR_TOP,
        }
    }
}

/// One 14 px outlined text run of a button, left-justified at `left`.
fn button_text(name: DynName, text: &str, width: f32, left: f32, color: [f32; 4]) -> Element {
    let [r, g, b, a] = color;
    rsx! {
        fontstring {
            name,
            width,
            height: BUTTON_HEIGHT,
            text,
            font: GameFont::FrizQuadrata,
            font_size: FONT_SIZE,
            font_color: FontColor::new(r, g, b, a),
            outline: "OUTLINE",
            justify_h: JustifyH::Left,
            pos_type: "absolute",
            left,
            top: 0.0,
        }
    }
}

/// "(5) Normal" and "(0/3)", two spaces apart and centred together (`FormatButtonText`).
fn button_texts(entry: &EntranceBarChoice, button_width: f32) -> Element {
    let id = entry.choice.difficulty_id;
    let content = entry.label_width + PROGRESS_GAP + entry.progress_width;
    let label_left = ((button_width - content) / 2.0).round();
    let progress_left = label_left + entry.label_width + PROGRESS_GAP;
    let label = button_text(
        part(id, "Label"),
        &entry.choice.label,
        entry.label_width,
        label_left,
        LABEL_GOLD,
    );
    let progress = button_text(
        part(id, "Progress"),
        &entry.choice.progress_text(),
        entry.progress_width,
        progress_left,
        entry.choice.progress_color(),
    );
    label.into_iter().chain(progress).collect()
}

fn button(
    entry: &EntranceBarChoice,
    layout: &EntranceBarLayout,
    index: usize,
    alpha: f32,
) -> Element {
    let id = entry.choice.difficulty_id;
    rsx! {
        button {
            name: {DynName(button_name(id))},
            width: {layout.button_width},
            height: BUTTON_HEIGHT,
            onclick: {format!("{ACTION_ENTRANCE_DIFFICULTY_PREFIX}{id}")},
            button_default_skin: false,
            frame_level: BUTTON_LEVEL,
            alpha,
            pos_type: "absolute",
            left: {layout.button_left(index)},
            top: BUTTON_TOP,
            {button_texts(entry, layout.button_width)}
        }
    }
}

fn gold_box(layout: &EntranceBarLayout, left: f32) -> Element {
    let width = layout.button_width + 2.0 * BOX_OVERHANG;
    let pieces = three_slice(
        "EntranceDifficultyBox",
        [0.0, 0.0, width, BAR_HEIGHT],
        BOX_CAP,
        [BOX_LEFT_UV, BOX_CENTER_UV, BOX_RIGHT_UV],
    );
    rsx! {
        r#frame {
            name: ENTRANCE_BAR_BOX,
            width,
            height: BAR_HEIGHT,
            pos_type: "absolute",
            left,
            top: BAR_TOP,
            {pieces}
        }
    }
}

fn spinner(layout: &EntranceBarLayout, index: usize) -> Element {
    let left = layout.button_left(index) + (layout.button_width - SPINNER_SIZE) / 2.0;
    rsx! {
        texture {
            name: ENTRANCE_BAR_SPINNER,
            frame_level: SPINNER_LEVEL,
            width: SPINNER_SIZE,
            height: SPINNER_SIZE,
            texture_file: SPINNER_ART,
            pos_type: "absolute",
            left,
            top: {BUTTON_TOP + (BUTTON_HEIGHT - SPINNER_SIZE) / 2.0},
        }
    }
}

fn index_of(state: &EntranceBarState, difficulty_id: Option<u32>) -> Option<usize> {
    let id = difficulty_id?;
    state
        .choices
        .iter()
        .position(|entry| entry.choice.difficulty_id == id)
}

/// Plumber's radial black shadow, 1200x400 about the frame's centre.
fn shadow(layout: &EntranceBarLayout) -> Element {
    let [width, height] = SHADOW_SIZE;
    let left = ((layout.bar_width - width) / 2.0).round();
    let top = (FRAME_HEIGHT - height) / 2.0;
    art_piece_alpha(
        DynName("EntranceDifficultyShadow".into()),
        [left, top, width, height],
        SHADOW_UV,
        SHADOW_ALPHA,
    )
}

/// Dim layer: shadow, bar and hover highlights.
fn bar_layer(state: &EntranceBarState, layout: &EntranceBarLayout) -> Element {
    let bar = three_slice(
        "EntranceDifficultyBar",
        [0.0, BAR_TOP, layout.bar_width, BAR_HEIGHT],
        SIDE_PADDING,
        [BAR_LEFT_UV, BAR_CENTER_UV, BAR_RIGHT_UV],
    );
    let mut children = shadow(layout);
    children.extend(bar);
    for (index, entry) in state.choices.iter().enumerate() {
        children.extend(highlight(entry, layout, index));
    }
    rsx! {
        r#frame {
            name: "EntranceDifficultyBarLayer",
            width: {layout.bar_width},
            height: FRAME_HEIGHT,
            alpha: {state.bar_alpha},
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
            {children}
        }
    }
}

fn title(state: &EntranceBarState, layout: &EntranceBarLayout) -> Element {
    rsx! {
        fontstring {
            name: ENTRANCE_BAR_TITLE,
            width: {layout.bar_width},
            height: TITLE_HEIGHT,
            text: {state.title.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: FONT_SIZE,
            font_color: TITLE_COLOR,
            outline: "OUTLINE",
            justify_h: JustifyH::Center,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    }
}

/// Gold box, buttons and spinner. The buttons stay in this layer so a selection change
/// never re-creates them; unselected ones take the bar's alpha, as Plumber's buttons
/// parented to the bar do.
fn opaque_children(state: &EntranceBarState, layout: &EntranceBarLayout) -> Element {
    let mut children = title(state, layout);
    let selected = index_of(state, state.selected);
    if let Some(index) = selected {
        let left = state.box_left.unwrap_or_else(|| layout.box_left(index));
        children.extend(gold_box(layout, left));
    }
    for (index, entry) in state.choices.iter().enumerate() {
        let alpha = if Some(index) == selected {
            1.0
        } else {
            state.bar_alpha
        };
        children.extend(button(entry, layout, index, alpha));
    }
    if let Some(spin) = state.spinner
        && let Some(index) = index_of(state, Some(spin.difficulty_id))
    {
        children.extend(spinner(layout, index));
    }
    children
}

/// Full-alpha layer (Plumber `OpaqueFrame`).
fn opaque_layer(state: &EntranceBarState, layout: &EntranceBarLayout) -> Element {
    rsx! {
        r#frame {
            name: "EntranceDifficultyOpaqueLayer",
            width: {layout.bar_width},
            height: FRAME_HEIGHT,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
            {opaque_children(state, layout)}
        }
    }
}

pub fn entrance_difficulty_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<EntranceBarState>()
        .expect("EntranceBarState must be in SharedContext");
    let layout = EntranceBarLayout::new(&state.choices, state.screen_width);
    rsx! {
        r#frame {
            name: ENTRANCE_BAR_ROOT,
            width: {layout.bar_width},
            height: FRAME_HEIGHT,
            strata: FrameStrata::High,
            alpha: {state.frame_alpha},
            pos_type: "absolute",
            left: {layout.left},
            top: {layout.top},
            {bar_layer(state, &layout)}
            {opaque_layer(state, &layout)}
        }
    }
}

/// Hover and selection visuals the tree cannot express: white labels on the selected or
/// hovered button, its additive glow, and the spinner's rotation.
pub fn apply_entrance_bar_postsetup(state: &EntranceBarState, registry: &mut FrameRegistry) {
    for entry in &state.choices {
        let id = entry.choice.difficulty_id;
        sync_button_visual(registry, id, state.selected == Some(id));
    }
    if let Some(spin) = state.spinner {
        edit_texture(registry, ENTRANCE_BAR_SPINNER.0, |texture| {
            texture.rotation = spin.rotation;
        });
    }
}

/// Plumber `UpdateVisual`: gold, white when selected or hovered; the glow while hovered.
fn sync_button_visual(registry: &mut FrameRegistry, id: u32, selected: bool) {
    let hovered = registry
        .get_by_name(&button_name(id))
        .and_then(|frame| registry.get(frame))
        .is_some_and(|frame| {
            matches!(&frame.widget_data, Some(WidgetData::Button(button)) if button.hovered)
        });
    let color = if hovered || selected {
        LABEL_WHITE
    } else {
        LABEL_GOLD
    };
    if let Some(frame) = registry.get_by_name(&part(id, "Label").0)
        && let Some(frame) = registry.get_mut(frame)
        && let Some(WidgetData::FontString(label)) = &mut frame.widget_data
    {
        label.color = color;
    }
    let highlight = part(id, "Highlight").0;
    edit_texture(registry, &highlight, |texture| {
        texture.blend_mode = BlendMode::Additive;
    });
    if let Some(frame) = registry.get_by_name(&highlight)
        && registry
            .get(frame)
            .is_some_and(|frame| frame.hidden == hovered)
    {
        registry.set_hidden(frame, !hovered);
    }
}

fn edit_texture(registry: &mut FrameRegistry, name: &str, edit: impl FnOnce(&mut TextureData)) {
    let Some(id) = registry.get_by_name(name) else {
        return;
    };
    if let Some(frame) = registry.get_mut(id)
        && let Some(WidgetData::Texture(texture)) = frame.widget_data.as_mut()
    {
        edit(texture);
    }
}
