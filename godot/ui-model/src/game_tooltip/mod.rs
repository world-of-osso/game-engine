//! Retail `GameTooltip` content and placement for the native client (docs/specs/unit-tooltip.md,
//! "Native Godot coverage"): what each hovered source shows, where `SetOwner` /
//! `GameTooltip_SetDefaultAnchor` put it, the screen clamp and the comparison tooltips.
//! The host resolves the hovered frame or unit and renders [`GameTooltipView`].

pub mod hud;
pub mod item;
pub mod merchant;
pub mod render;
pub mod spell;
pub mod unit;

use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;

use crate::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::tooltip_presentation::{
    GRAY_FONT_COLOR, TOOLTIP_DESCRIPTION_COLOR, TooltipLineState, TooltipPresentation,
    item_id_line, rgba_string,
};
pub use item::ShoppingTooltip;
use render::retail_tooltip;
pub use render::tooltip_size;

/// `GameTooltipDefaultContainer` (GameTooltip.xml:242-247): BOTTOMRIGHT of UIParent at
/// x -9, y 85; `GameTooltip_SetDefaultAnchor` puts the tooltip's BOTTOMRIGHT there.
const DEFAULT_ANCHOR_RIGHT: f32 = 9.0;
const DEFAULT_ANCHOR_BOTTOM: f32 = 85.0;
/// `ANCHOR_CURSOR`: the tooltip's TOPLEFT 20 above the cursor (wow-ui-sim
/// `DEFAULT_CURSOR_Y_OFFSET`, `build_cursor_anchor`).
const CURSOR_OFFSET_Y: f32 = 20.0;

/// The record a tooltip describes; [`place`] ends the tooltip with its grey ID line (a
/// user-requested deviation from Retail, like idTip-style addons).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TooltipRecord {
    Creature(u32),
    Spell(u32),
    Item(u32),
}

impl TooltipRecord {
    fn id_line(self) -> TooltipLineState {
        let text = match self {
            Self::Creature(id) => format!("Creature ID: {id}"),
            Self::Spell(id) => format!("Spell ID: {id}"),
            Self::Item(id) => return item_id_line(id),
        };
        TooltipLineState::colored(text, GRAY_FONT_COLOR)
    }
}

/// `GameTooltip:SetOwner(owner, anchorType)` anchor types (Widget API), plus the two
/// computed ones Retail builds from `ANCHOR_NONE`/`ANCHOR_LEFT`/`ANCHOR_RIGHT`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerSide {
    /// `ANCHOR_RIGHT`: the tooltip's BOTTOMLEFT on the owner's TOPRIGHT.
    Right,
    /// `ANCHOR_LEFT`: BOTTOMRIGHT on the owner's TOPLEFT.
    Left,
    /// `ANCHOR_TOP`: BOTTOM on the owner's TOP.
    Top,
    /// `ANCHOR_BOTTOM`: TOP on the owner's BOTTOM.
    Bottom,
    /// `ANCHOR_TOPLEFT`: BOTTOMLEFT on the owner's TOPLEFT.
    TopLeft,
    /// `ANCHOR_TOPRIGHT`: BOTTOMRIGHT on the owner's TOPRIGHT.
    TopRight,
    /// `ANCHOR_BOTTOMLEFT`: TOPRIGHT on the owner's BOTTOMLEFT.
    BottomLeft,
    /// `ANCHOR_BOTTOMRIGHT`: TOPLEFT on the owner's BOTTOMRIGHT.
    BottomRight,
    /// `ContainerFrameItemButton_CalculateItemTooltipAnchors` (ContainerFrame.lua:1448-1458):
    /// right when the owner's right edge is left of the screen centre, else left.
    BagSlot,
    /// TargetFrame.xml:35-40 aura `OnEnter`: left when the owner's centre is right of the
    /// screen centre, else right.
    ByCenter,
}

/// Where a tooltip goes.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum TooltipAnchor {
    /// `GameTooltip_SetDefaultAnchor`: bottom right of the screen.
    #[default]
    Default,
    /// `ANCHOR_CURSOR`: follows the cursor.
    Cursor,
    /// `ANCHOR_CURSOR_RIGHT` with no offset: the tooltip's BOTTOMLEFT on the cursor.
    CursorRight,
    /// `SetOwner(owner, side)`; `rect` is the owner's `[x, y, w, h]` in UI units, y down.
    Owner { rect: [f32; 4], side: OwnerSide },
}

/// A tooltip's content before placement.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GameTooltip {
    pub content: TooltipPresentation,
    pub record: Option<TooltipRecord>,
    pub anchor: TooltipAnchor,
}

impl GameTooltip {
    pub fn new(content: TooltipPresentation, record: Option<TooltipRecord>) -> Self {
        Self {
            content: TooltipPresentation {
                visible: true,
                ..content
            },
            record,
            anchor: TooltipAnchor::Default,
        }
    }

    pub fn owned(self, rect: [f32; 4], side: OwnerSide) -> Self {
        Self {
            anchor: TooltipAnchor::Owner { rect, side },
            ..self
        }
    }

    pub fn at_cursor(self) -> Self {
        Self {
            anchor: TooltipAnchor::Cursor,
            ..self
        }
    }

    /// The Forever skin takes FlareUI's tooltip anchor: its `GameTooltip_SetDefaultAnchor`
    /// hook re-owns default-anchored tooltips at `ANCHOR_CURSOR_RIGHT` with offset 0, 0
    /// (Tooltips.lua:410-423,459; `anchor`/`anchorFrames` "cursorOffset", Core.lua:350-353).
    pub fn for_skin(self, skin: ActiveSkin) -> Self {
        match (skin, self.anchor) {
            (ActiveSkin::Forever, TooltipAnchor::Default) => Self {
                anchor: TooltipAnchor::CursorRight,
                ..self
            },
            _ => self,
        }
    }
}

/// What the host renders: the main tooltip and up to two comparison tooltips
/// (`ShoppingTooltip1`/`2`), all placed in UI units.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GameTooltipView {
    pub main: TooltipPresentation,
    pub shopping: [ShoppingTooltip; 2],
    /// Health fraction of the unit a unit tooltip shows, for `GameTooltipStatusBar`.
    pub health: Option<f32>,
}

/// `GameTooltipStatusBar` (GameTooltip.xml:9-23): 8 tall, TOPLEFT 2 in from the tooltip's
/// BOTTOMLEFT and 1 below it, as wide as the tooltip less 2 on each side, filled with
/// `Interface\TargetingFrame\UI-TargetingFrame-BarFill` in `HealthBar_OnValueChanged`'s
/// green.
const HEALTH_BAR_H: f32 = 8.0;
const HEALTH_BAR_INSET: f32 = 2.0;
const HEALTH_BAR_FILL_FDID: u32 = 137_014;
const HEALTH_BAR_GREEN: &str = "0.0,1.0,0.0,1.0";

/// `ShoppingTooltipTemplate.CompareHeader` (GameTooltip.xml:111-128): 22 tall, its
/// BOTTOMLEFT 1 below the tooltip's TOPLEFT, the label 30 narrower than the header.
const COMPARE_HEADER_H: f32 = 22.0;
const COMPARE_HEADER_PADDING: f32 = 30.0;
/// `tooltip-compare-label`: UiTextureAtlas 3479 (7304549, 128×32), 94×29 at (1, 1).
const COMPARE_LABEL: AtlasArt = AtlasArt {
    fdid: 7_304_549,
    atlas: (128.0, 32.0),
    rect: (1.0, 95.0, 1.0, 30.0),
};

struct DynName(String);

impl std::fmt::Display for DynName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The main `GameTooltip` (frames `TooltipFrame`, `TooltipTitle`, `TooltipLine{i}…`) and
/// the comparison tooltips (`ShoppingTooltip{n}Frame`, `ShoppingTooltip{n}Header`, …).
pub fn game_tooltip_screen(ctx: &SharedContext) -> Element {
    let view = ctx
        .get::<GameTooltipView>()
        .expect("GameTooltipView must be in SharedContext");
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let mut elements = retail_tooltip(&view.main, "Tooltip", skin);
    elements.extend(health_bar(&view.main, view.health));
    for (index, shopping) in view.shopping.iter().enumerate() {
        let prefix = format!("ShoppingTooltip{}", index + 1);
        elements.extend(compare_header(&prefix, shopping));
        elements.extend(retail_tooltip(&shopping.tooltip, &prefix, skin));
    }
    elements
}

fn health_bar(main: &TooltipPresentation, health: Option<f32>) -> Element {
    let fraction = health.unwrap_or(0.0).clamp(0.0, 1.0);
    let hidden = !main.visible || health.is_none();
    let [main_w, main_h] = tooltip_size(main);
    let width = main_w - 2.0 * HEALTH_BAR_INSET;
    let (x, y) = (main.x + HEALTH_BAR_INSET, main.y + main_h + 1.0);
    let fill = width * fraction;
    let fill_hidden = hidden || fill <= 0.0;
    let fill_coords = format!("0,{fraction},0,1");
    rsx! {
        r#frame {
            name: "TooltipStatusBar",
            width: {width},
            height: {HEALTH_BAR_H},
            hidden: {hidden},
            strata: "TOOLTIP",
            pos_type: "absolute",
            anchor: "screen",
            pos_x: {x},
            pos_y: {y},
            texture {
                name: "TooltipStatusBarFill",
                width: {fill.max(1.0)},
                height: {HEALTH_BAR_H},
                hidden: {fill_hidden},
                texture_fdid: {HEALTH_BAR_FILL_FDID},
                tex_coords: {fill_coords.as_str()},
                vertex_color: HEALTH_BAR_GREEN,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: 0.0,
            }
        }
    }
}

fn compare_header(prefix: &str, shopping: &ShoppingTooltip) -> Element {
    let label_w = measure_text(&shopping.header, GameFont::FrizQuadrata, render::TEXT_SIZE)
        .map_or(0.0, |(width, _)| width.ceil());
    let width = label_w + COMPARE_HEADER_PADDING;
    let hidden = !shopping.tooltip.visible || shopping.header.is_empty();
    let (x, y) = (
        shopping.tooltip.x,
        shopping.tooltip.y - COMPARE_HEADER_H + 1.0,
    );
    let coords = COMPARE_LABEL.tex_coords(1.0);
    rsx! {
        r#frame {
            name: {DynName(format!("{prefix}Header"))},
            width: {width},
            height: {COMPARE_HEADER_H},
            hidden: {hidden},
            strata: "TOOLTIP",
            pos_type: "absolute",
            anchor: "screen",
            pos_x: {x},
            pos_y: {y},
            texture {
                name: {DynName(format!("{prefix}HeaderBackground"))},
                width: {width},
                height: {COMPARE_HEADER_H},
                texture_fdid: {COMPARE_LABEL.fdid},
                tex_coords: {coords.as_str()},
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: 0.0,
            }
            fontstring {
                name: {DynName(format!("{prefix}HeaderLabel"))},
                width: {width},
                height: {COMPARE_HEADER_H},
                text: {shopping.header.as_str()},
                font: "FrizQuadrata",
                font_size: {render::TEXT_SIZE},
                font_color: {rgba_string(TOOLTIP_DESCRIPTION_COLOR)},
                justify_h: "CENTER",
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: 0.0,
            }
        }
    }
}

/// The screen and cursor in UI units, y down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipScreen {
    pub size: [f32; 2],
    pub cursor: [f32; 2],
}

/// Place `tooltip` at its anchor, clamped to the screen (`clampedToScreen`,
/// SharedTooltipTemplates.xml:10), ending it with its record's ID line.
pub fn place(tooltip: GameTooltip, screen: TooltipScreen) -> TooltipPresentation {
    let mut content = tooltip.content;
    if let Some(record) = tooltip.record {
        content.lines.push(record.id_line());
    }
    let [width, height] = tooltip_size(&content);
    let [x, y] = anchor_origin(tooltip.anchor, screen, [width, height]);
    content.visible = true;
    content.x = x.clamp(0.0, (screen.size[0] - width).max(0.0));
    content.y = y.clamp(0.0, (screen.size[1] - height).max(0.0));
    content
}

/// Top-left of a `[w, h]` tooltip at `anchor`, before clamping.
fn anchor_origin(anchor: TooltipAnchor, screen: TooltipScreen, [w, h]: [f32; 2]) -> [f32; 2] {
    let [sw, sh] = screen.size;
    match anchor {
        TooltipAnchor::Default => [
            sw - DEFAULT_ANCHOR_RIGHT - w,
            sh - DEFAULT_ANCHOR_BOTTOM - h,
        ],
        TooltipAnchor::Cursor => [screen.cursor[0], screen.cursor[1] - CURSOR_OFFSET_Y],
        TooltipAnchor::CursorRight => [screen.cursor[0], screen.cursor[1] - h],
        TooltipAnchor::Owner { rect, side } => owner_origin(rect, side, sw, [w, h]),
    }
}

fn owner_origin(
    [ox, oy, ow, oh]: [f32; 4],
    side: OwnerSide,
    sw: f32,
    [w, h]: [f32; 2],
) -> [f32; 2] {
    let right = [ox + ow, oy - h];
    let left = [ox - w, oy - h];
    match side {
        OwnerSide::Right => right,
        OwnerSide::Left => left,
        OwnerSide::Top => [ox + (ow - w) / 2.0, oy - h],
        OwnerSide::Bottom => [ox + (ow - w) / 2.0, oy + oh],
        OwnerSide::TopLeft => [ox, oy - h],
        OwnerSide::TopRight => [ox + ow - w, oy - h],
        OwnerSide::BottomLeft => [ox - w, oy + oh],
        OwnerSide::BottomRight => [ox + ow, oy + oh],
        OwnerSide::BagSlot if ox + ow < sw / 2.0 => right,
        OwnerSide::BagSlot => left,
        OwnerSide::ByCenter if ox + ow / 2.0 > sw / 2.0 => left,
        OwnerSide::ByCenter => right,
    }
}

/// `TooltipComparisonManager:AnchorShoppingTooltips` (TooltipComparisonManager.lua:48-140):
/// the comparison tooltips sit top-aligned beside the placed main tooltip. A left or right
/// `SetOwner` anchor keeps its side when the comparisons fit there, else they go to the side
/// with more room; the main tooltip slides inward when they still do not fit. On the right
/// the second comparison is next to the main tooltip; on the left the first one is.
pub fn place_comparisons(
    main: &mut TooltipPresentation,
    anchor: TooltipAnchor,
    comparisons: Vec<ShoppingTooltip>,
    screen: TooltipScreen,
) -> [ShoppingTooltip; 2] {
    let comparisons: Vec<_> = comparisons.into_iter().take(2).collect();
    let widths: Vec<f32> = comparisons
        .iter()
        .map(|shopping| tooltip_size(&shopping.tooltip)[0])
        .collect();
    let total: f32 = widths.iter().sum();
    let main_w = tooltip_size(main)[0];
    let (left, right_dist) = (main.x, screen.size[0] - (main.x + main_w));
    let side = match anchor {
        TooltipAnchor::Owner { side, .. } => Some(side),
        _ => None,
    };
    let on_left = match side {
        Some(OwnerSide::Left | OwnerSide::TopLeft | OwnerSide::BottomLeft) if total < left => true,
        Some(OwnerSide::Right | OwnerSide::TopRight | OwnerSide::BottomRight)
            if total < right_dist =>
        {
            false
        }
        _ => right_dist < left,
    };
    if on_left && total > left {
        main.x += total - left;
    } else if !on_left && total > right_dist {
        main.x -= total - right_dist;
    }
    main.x = main.x.clamp(0.0, (screen.size[0] - main_w).max(0.0));
    // Outward from the main tooltip: the first comparison on the left, the second on the
    // right (secondary next to the main tooltip, primary beyond it).
    let order: Vec<usize> = if on_left {
        (0..comparisons.len()).collect()
    } else {
        (0..comparisons.len()).rev().collect()
    };
    let mut placed = [ShoppingTooltip::hidden(), ShoppingTooltip::hidden()];
    let mut edge = if on_left { main.x } else { main.x + main_w };
    let mut comparisons: Vec<Option<ShoppingTooltip>> = comparisons.into_iter().map(Some).collect();
    for index in order {
        let Some(mut shopping) = comparisons[index].take() else {
            continue;
        };
        let [width, height] = tooltip_size(&shopping.tooltip);
        let tooltip = &mut shopping.tooltip;
        tooltip.visible = true;
        tooltip.x = if on_left { edge - width } else { edge };
        edge = if on_left { edge - width } else { edge + width };
        tooltip.y = main.y.clamp(0.0, (screen.size[1] - height).max(0.0));
        placed[index] = shopping;
    }
    placed
}

/// Tests measure text with the repository's fonts and read the loaded item catalog.
#[cfg(test)]
pub(crate) fn set_test_data_root() {
    crate::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .expect("tooltip test data root");
    crate::item_catalog::wait_for_item_catalog();
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: TooltipScreen = TooltipScreen {
        size: [1920.0, 1080.0],
        cursor: [800.0, 600.0],
    };

    fn five_lines() -> GameTooltip {
        GameTooltip::new(
            TooltipPresentation {
                lines: vec![TooltipLineState::new("line"); 5],
                ..TooltipPresentation::hidden()
            },
            None,
        )
    }

    /// Width of the five-line test tooltip.
    fn w() -> f32 {
        set_test_data_root();
        tooltip_size(&five_lines().content)[0]
    }

    fn beside(x: f32, y: f32, side: OwnerSide) -> (f32, f32, f32) {
        let tooltip = five_lines();
        let height = tooltip_size(&tooltip.content)[1];
        let placed = place(tooltip.owned([x, y, 36.0, 36.0], side), SCREEN);
        assert!(placed.visible);
        (placed.x, placed.y, height)
    }

    #[test]
    fn owner_anchors_follow_the_retail_anchor_types() {
        let (x, y, h) = beside(600.0, 500.0, OwnerSide::Right);
        assert_eq!((x, y), (636.0, 500.0 - h));
        let (x, y, h) = beside(600.0, 500.0, OwnerSide::Left);
        assert_eq!((x, y), (600.0 - w(), 500.0 - h));
        let (x, y, h) = beside(600.0, 500.0, OwnerSide::Top);
        assert_eq!((x, y), (618.0 - w() / 2.0, 500.0 - h));
        let (x, y, _) = beside(600.0, 500.0, OwnerSide::Bottom);
        assert_eq!((x, y), (618.0 - w() / 2.0, 536.0));
        let (x, y, h) = beside(600.0, 500.0, OwnerSide::TopLeft);
        assert_eq!((x, y), (600.0, 500.0 - h));
        let (x, y, h) = beside(600.0, 500.0, OwnerSide::TopRight);
        assert_eq!((x, y), (636.0 - w(), 500.0 - h));
        let (x, y, _) = beside(1600.0, 20.0, OwnerSide::BottomLeft);
        assert_eq!((x, y), (1600.0 - w(), 56.0));
        let (x, y, _) = beside(600.0, 20.0, OwnerSide::BottomRight);
        assert_eq!((x, y), (636.0, 56.0));
    }

    #[test]
    fn bag_slots_and_target_auras_pick_their_side_by_screen_half() {
        assert_eq!(beside(600.0, 500.0, OwnerSide::BagSlot).0, 636.0);
        assert_eq!(beside(1700.0, 500.0, OwnerSide::BagSlot).0, 1700.0 - w());
        // The right edge decides for bags: 928 + 36 > 960.
        assert_eq!(beside(928.0, 500.0, OwnerSide::BagSlot).0, 928.0 - w());
        // The centre decides for auras: 928 + 18 < 960.
        assert_eq!(beside(928.0, 500.0, OwnerSide::ByCenter).0, 964.0);
        assert_eq!(beside(1000.0, 200.0, OwnerSide::ByCenter).0, 1000.0 - w());
    }

    #[test]
    fn the_default_anchor_is_bottom_right_and_cursor_follows_the_pointer() {
        let tooltip = five_lines();
        let height = tooltip_size(&tooltip.content)[1];
        let placed = place(tooltip.clone(), SCREEN);
        assert_eq!(placed.x + w(), 1920.0 - 9.0);
        assert_eq!(placed.y + height, 1080.0 - 85.0);
        let placed = place(tooltip.at_cursor(), SCREEN);
        assert_eq!((placed.x, placed.y), (800.0, 580.0));
    }

    #[test]
    fn forever_puts_default_anchored_tooltips_right_of_the_cursor() {
        let tooltip = five_lines();
        let height = tooltip_size(&tooltip.content)[1];
        let placed = place(tooltip.clone().for_skin(ActiveSkin::Forever), SCREEN);
        // ANCHOR_CURSOR_RIGHT: the tooltip's BOTTOMLEFT on the cursor (800, 600).
        assert_eq!((placed.x, placed.y), (800.0, 600.0 - height));
        // Modern keeps GameTooltip_SetDefaultAnchor's bottom right.
        let placed = place(tooltip.clone().for_skin(ActiveSkin::Modern), SCREEN);
        assert_eq!(placed.x + w(), 1920.0 - 9.0);
        // FlareUI only hooks the default anchor: owned tooltips keep their owner.
        let owned = tooltip.owned([600.0, 500.0, 36.0, 36.0], OwnerSide::Right);
        let placed = place(owned.for_skin(ActiveSkin::Forever), SCREEN);
        assert_eq!((placed.x, placed.y), (636.0, 500.0 - height));
    }

    #[test]
    fn forever_cursor_tooltips_stay_on_screen_at_the_right_edge() {
        let tooltip = five_lines().for_skin(ActiveSkin::Forever);
        let height = tooltip_size(&tooltip.content)[1];
        let inside = |placed: &TooltipPresentation| {
            placed.x >= 0.0
                && placed.y >= 0.0
                && placed.x + w() <= 1920.0
                && placed.y + height <= 1080.0
        };
        // Bottom right: pushed left onto the screen edge, still ending on the cursor's row.
        let bottom_right = TooltipScreen {
            cursor: [1915.0, 1078.0],
            ..SCREEN
        };
        let placed = place(tooltip.clone(), bottom_right);
        assert!(inside(&placed));
        assert_eq!(placed.x + w(), 1920.0);
        assert_eq!(placed.y + height, 1078.0);
        // Top right: a tooltip taller than the space above the cursor drops to the top edge.
        let top_right = TooltipScreen {
            cursor: [1915.0, 5.0],
            ..SCREEN
        };
        let placed = place(tooltip, top_right);
        assert!(inside(&placed));
        assert_eq!((placed.x + w(), placed.y), (1920.0, 0.0));
    }

    #[test]
    fn tooltips_clamp_to_every_screen_edge() {
        let (x, y, _) = beside(1900.0, 10.0, OwnerSide::Right);
        assert_eq!((x, y), (1920.0 - w(), 0.0));
        let (x, y, h) = beside(10.0, 1070.0, OwnerSide::BottomLeft);
        assert_eq!((x, y), (0.0, 1080.0 - h));
        let corner = TooltipScreen {
            cursor: [1915.0, 2.0],
            ..SCREEN
        };
        let placed = place(five_lines().at_cursor(), corner);
        assert_eq!((placed.x, placed.y), (1920.0 - w(), 0.0));
    }

    #[test]
    fn every_record_ends_with_its_grey_id_line_and_others_have_none() {
        for (record, text) in [
            (TooltipRecord::Creature(38), "Creature ID: 38"),
            (TooltipRecord::Spell(116), "Spell ID: 116"),
            (TooltipRecord::Item(2589), "Item ID: 2589"),
        ] {
            let tooltip = GameTooltip {
                record: Some(record),
                ..five_lines()
            };
            let placed = place(tooltip, SCREEN);
            assert_eq!(placed.lines.len(), 6);
            let last = placed.lines.last().unwrap();
            assert_eq!(
                (last.left_text.as_str(), last.left_color),
                (text, GRAY_FONT_COLOR)
            );
        }
        assert_eq!(place(five_lines(), SCREEN).lines.len(), 5);
    }

    #[test]
    fn the_screen_draws_the_main_tooltip_and_headed_comparisons() {
        set_test_data_root();
        use ui_toolkit::frame::WidgetData;
        use ui_toolkit::registry::FrameRegistry;
        use ui_toolkit::screen::Screen;
        let main = TooltipPresentation {
            visible: true,
            title: "Defias Rapier".into(),
            x: 900.0,
            y: 500.0,
            ..five_lines().content
        };
        let shopping = ShoppingTooltip {
            header: "Equipped".into(),
            tooltip: TooltipPresentation {
                visible: true,
                title: "Worn Shortsword".into(),
                x: 640.0,
                y: 500.0,
                ..five_lines().content
            },
        };
        let view_main = main.clone();
        let view = GameTooltipView {
            main,
            shopping: [shopping, ShoppingTooltip::hidden()],
            health: Some(0.25),
        };
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(view);
        shared.insert(ActiveSkin::Modern);
        Screen::new(game_tooltip_screen).sync(&shared, &mut registry);
        let frame = |name: &str| {
            registry
                .get(
                    registry
                        .get_by_name(name)
                        .unwrap_or_else(|| panic!("{name}")),
                )
                .unwrap()
        };
        let text = |name: &str| match frame(name).widget_data.as_ref() {
            Some(WidgetData::FontString(text)) => text.text.clone(),
            other => panic!("{name}: {other:?}"),
        };
        assert_eq!(text("TooltipTitle"), "Defias Rapier");
        assert_eq!(text("ShoppingTooltip1Title"), "Worn Shortsword");
        assert_eq!(text("ShoppingTooltip1HeaderLabel"), "Equipped");
        assert!(!frame("ShoppingTooltip1Frame").hidden);
        assert!(!frame("ShoppingTooltip1Header").hidden);
        assert!(frame("ShoppingTooltip2Frame").hidden);
        assert!(frame("ShoppingTooltip2Header").hidden);
        // The health bar under the main tooltip, a quarter full.
        assert!(!frame("TooltipStatusBar").hidden);
        let main_w = tooltip_size(&view_main)[0];
        assert_eq!(
            frame("TooltipStatusBarFill").width,
            ui_toolkit::frame::Dimension::Fixed((main_w - 4.0) * 0.25)
        );
    }

    #[test]
    fn comparisons_sit_beside_the_main_tooltip_on_the_side_with_room() {
        let compare = |title: &str| ShoppingTooltip {
            header: "Equipped".into(),
            tooltip: TooltipPresentation {
                title: title.into(),
                ..five_lines().content
            },
        };
        // ANCHOR_RIGHT with room on the right: the second comparison is next to the main one.
        let anchor = TooltipAnchor::Owner {
            rect: [300.0, 600.0, 36.0, 36.0],
            side: OwnerSide::Right,
        };
        let mut main = place(
            GameTooltip {
                anchor,
                ..five_lines()
            },
            SCREEN,
        );
        let [first, second] =
            place_comparisons(&mut main, anchor, vec![compare("1"), compare("2")], SCREEN);
        assert_eq!((second.tooltip.x, second.tooltip.y), (main.x + w(), main.y));
        assert_eq!(first.tooltip.x, main.x + 2.0 * w());
        // The default anchor at the bottom right has more room on the left.
        let mut main = place(five_lines(), SCREEN);
        let [first, second] = place_comparisons(
            &mut main,
            TooltipAnchor::Default,
            vec![compare("1")],
            SCREEN,
        );
        assert_eq!(first.tooltip.x, main.x - w());
        assert!(first.tooltip.visible && !second.tooltip.visible);
        // A left-anchored tooltip keeps its side when the comparison fits there.
        let anchor = TooltipAnchor::Owner {
            rect: [600.0, 600.0, 36.0, 36.0],
            side: OwnerSide::Left,
        };
        let mut main = place(
            GameTooltip {
                anchor,
                ..five_lines()
            },
            SCREEN,
        );
        let [first, _] = place_comparisons(&mut main, anchor, vec![compare("1")], SCREEN);
        assert_eq!((main.x, first.tooltip.x), (600.0 - w(), 600.0 - 2.0 * w()));
        // Room on neither side: the main tooltip slides inward, staying on the screen.
        let narrow = TooltipScreen {
            size: [1.5 * w(), 1080.0],
            ..SCREEN
        };
        let mut main = TooltipPresentation {
            x: 0.25 * w(),
            ..five_lines().content
        };
        let [first, _] = place_comparisons(
            &mut main,
            TooltipAnchor::Default,
            vec![compare("1")],
            narrow,
        );
        assert_eq!((main.x, first.tooltip.x), (0.0, w()));
    }
}
