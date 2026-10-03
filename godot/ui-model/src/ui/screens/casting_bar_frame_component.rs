use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::hud_layout::hud_layout;
use crate::ui::screens::inworld_unit_frames_component::CAST_DOCK_W;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_flare::{
    FLARE_INSET, flare_border,
};

#[cfg(all(test, feature = "dev"))]
#[path = "menu_character_layout_test_support.rs"]
mod layout_test_support;

/// The bar fills the cast area.
pub const BAR_W: f32 = CAST_DOCK_W - 8.0;
pub const BAR_H: f32 = 20.0;
const BAR_INSET: f32 = 4.0;
const SPARK_W: f32 = 8.0;
const TEXT_H: f32 = 14.0;
const TIMER_W: f32 = 40.0;

const BORDER_BG: &str = "0.0,0.0,0.0,0.8";
const BAR_BG: &str = "0.15,0.15,0.15,0.9";
const FILL_CAST: &str = "1.0,0.7,0.0,1.0";
const FILL_CHANNEL: &str = "0.0,0.64,0.0,1.0";
const FILL_UNINTERRUPTIBLE: &str = "0.63,0.63,0.63,1.0";
const FILL_INTERRUPTED: &str = "1.0,0.0,0.0,1.0";
/// Retail GlobalStrings `INTERRUPTED`.
pub const INTERRUPTED_TEXT: &str = "Interrupted";
const SPARK_COLOR: &str = "1.0,1.0,1.0,0.8";
/// How a skin draws the player cast bar: bar size, the holder's inset around it, the
/// holder's background and the fill colours.
struct CastBarStyle {
    bar: (f32, f32),
    inset: f32,
    holder_background: &'static str,
    /// The bronze border round the holder.
    border: bool,
    spark: bool,
    cast: &'static str,
    channel: &'static str,
    uninterruptible: &'static str,
}

const MODERN_STYLE: CastBarStyle = CastBarStyle {
    bar: (BAR_W, BAR_H),
    inset: BAR_INSET,
    holder_background: BORDER_BG,
    border: false,
    spark: true,
    cast: FILL_CAST,
    channel: FILL_CHANNEL,
    uninterruptible: FILL_UNINTERRUPTIBLE,
};

/// FlareUI's standalone player cast bar: `playerCastbar` 292×26 (Core.lua:281) in a
/// transparent holder INSET 4 larger under the bronze border, `PLAYER_CAST_COLOR`
/// #5C8FC7, `PLAYER_CAST_CHANNEL` #80BFE0 and `CAST_NOINTERRUPT` grey
/// (UnitFrames.lua:75-79,651-661,2224-2227). The spell icon left of the bar is not drawn.
const FOREVER_STYLE: CastBarStyle = CastBarStyle {
    bar: (292.0, 26.0),
    inset: FLARE_INSET,
    holder_background: "0.0,0.0,0.0,0.0",
    border: true,
    spark: false,
    cast: "0.36,0.56,0.78,1.0",
    channel: "0.50,0.75,0.88,1.0",
    uninterruptible: "0.55,0.55,0.55,1.0",
};

fn cast_bar_style(skin: ActiveSkin) -> &'static CastBarStyle {
    match skin {
        ActiveSkin::Modern => &MODERN_STYLE,
        ActiveSkin::Forever => &FOREVER_STYLE,
    }
}

const SPELL_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const TIMER_COLOR: &str = "1.0,1.0,1.0,1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct CastingBarState {
    pub visible: bool,
    pub spell_name: String,
    pub timer_text: String,
    /// Fill fraction 0.0..=1.0.
    pub progress: f32,
    pub is_channel: bool,
    pub is_interruptible: bool,
    pub is_interrupted: bool,
}

impl Default for CastingBarState {
    fn default() -> Self {
        Self {
            visible: false,
            spell_name: String::new(),
            timer_text: String::new(),
            progress: 0.0,
            is_channel: false,
            is_interruptible: true,
            is_interrupted: false,
        }
    }
}

impl CastingBarState {
    /// Full red bar with "Interrupted", as Retail `CastingBarFrame` shows it.
    pub fn interrupted() -> Self {
        Self {
            visible: true,
            spell_name: INTERRUPTED_TEXT.into(),
            progress: 1.0,
            is_interrupted: true,
            ..Self::default()
        }
    }
}

pub fn casting_bar_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<CastingBarState>()
        .expect("CastingBarState must be in SharedContext");
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let style = cast_bar_style(skin);
    let hide = !state.visible;
    let (bar_w, bar_h) = style.bar;
    let holder = (bar_w + 2.0 * style.inset, bar_h + 2.0 * style.inset);
    let fill_w = bar_w * state.progress.clamp(0.0, 1.0);
    let at = hud_layout(ctx).cast_bar.place(holder);
    let border = if style.border {
        flare_border("PlayerCastingBarFrame", holder)
    } else {
        Element::default()
    };
    rsx! {
        r#frame {
            name: "PlayerCastingBarFrame",
            width: {holder.0},
            height: {holder.1},
            background_color: style.holder_background,
            hidden: hide,
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {bar_background(style, fill_w, bar_fill_color(state, style), state)}
            {border}
        }
    }
}

fn bar_fill_color(state: &CastingBarState, style: &CastBarStyle) -> &'static str {
    if state.is_interrupted {
        FILL_INTERRUPTED
    } else if !state.is_interruptible {
        style.uninterruptible
    } else if state.is_channel {
        style.channel
    } else {
        style.cast
    }
}

fn bar_background(
    style: &CastBarStyle,
    fill_w: f32,
    color: &str,
    state: &CastingBarState,
) -> Element {
    let (bar_w, bar_h) = style.bar;
    let spark = if style.spark {
        spark(fill_w - SPARK_W / 2.0, bar_h)
    } else {
        Element::default()
    };
    rsx! {
        r#frame {
            name: "CastingBarBackground",
            width: {bar_w},
            height: {bar_h},
            background_color: BAR_BG,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
            {fill_bar(fill_w, bar_h, color)}
            {spark}
            {spell_name_text(&state.spell_name, bar_w)}
            {timer_text(&state.timer_text)}
        }
    }
}

fn fill_bar(fill_w: f32, height: f32, color: &str) -> Element {
    rsx! {
        r#frame {
            name: "CastingBarFill",
            width: {fill_w},
            height: {height},
            background_color: color,
            pos_type: "absolute",
            pos_x: 0.0,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn spark(x: f32, bar_h: f32) -> Element {
    rsx! {
        r#frame {
            name: "CastingBarSpark",
            width: {SPARK_W},
            height: {bar_h + 6.0},
            background_color: SPARK_COLOR,
            pos_type: "absolute",
            pos_x: x,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn spell_name_text(name: &str, bar_w: f32) -> Element {
    rsx! {
        fontstring {
            name: "CastingBarSpellName",
            width: {bar_w - TIMER_W},
            height: {TEXT_H},
            text: name,
            font_size: 10.0,
            font_color: SPELL_NAME_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
        }
    }
}

fn timer_text(timer: &str) -> Element {
    rsx! {
        fontstring {
            name: "CastingBarTimer",
            width: {TIMER_W},
            height: {TEXT_H},
            text: timer,
            font_size: 9.0,
            font_color: TIMER_COLOR,
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: 2.0,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

#[cfg(all(test, feature = "dev"))]
mod tests {
    use super::layout_test_support::compute_layout;
    use super::*;
    use crate::ui::screens::screen_test_helpers::fontstring_text;
    use ui_toolkit::frame::Dimension;
    use ui_toolkit::layout::LayoutRect;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    const MODERN_HOLDER_W: f32 = BAR_W + 2.0 * BAR_INSET;
    const MODERN_HOLDER_H: f32 = BAR_H + 2.0 * BAR_INSET;

    fn make_state(progress: f32) -> CastingBarState {
        CastingBarState {
            visible: true,
            spell_name: "Fireball".into(),
            timer_text: "1.5s".into(),
            progress,
            is_channel: false,
            is_interruptible: true,
            is_interrupted: false,
        }
    }

    fn build_registry(progress: f32) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
        shared.insert(make_state(progress));
        Screen::new(casting_bar_frame_screen).sync(&shared, &mut reg);
        reg
    }

    fn layout_reg(progress: f32) -> FrameRegistry {
        let mut reg = build_registry(progress);
        compute_layout(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    #[test]
    fn builds_all_elements() {
        let reg = build_registry(0.5);
        assert!(reg.get_by_name("PlayerCastingBarFrame").is_some());
        assert!(reg.get_by_name("CastingBarBackground").is_some());
        assert!(reg.get_by_name("CastingBarFill").is_some());
        assert!(reg.get_by_name("CastingBarSpark").is_some());
        assert!(reg.get_by_name("CastingBarSpellName").is_some());
        assert!(reg.get_by_name("CastingBarTimer").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
        shared.insert(CastingBarState::default());
        Screen::new(casting_bar_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("PlayerCastingBarFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    #[test]
    fn fill_width_matches_progress() {
        let reg = build_registry(0.6);
        let id = reg.get_by_name("CastingBarFill").expect("fill");
        let frame = reg.get(id).expect("data");
        let expected = BAR_W * 0.6;
        assert_eq!(frame.width, Dimension::Fixed(expected));
    }

    #[test]
    fn fill_color_changes_for_channel() {
        let color = |is_channel, is_interruptible| {
            bar_fill_color(
                &CastingBarState {
                    is_channel,
                    is_interruptible,
                    ..make_state(0.5)
                },
                &MODERN_STYLE,
            )
        };
        assert_eq!(color(false, true), FILL_CAST);
        assert_eq!(color(true, true), FILL_CHANNEL);
        assert_eq!(color(false, false), FILL_UNINTERRUPTIBLE);
        assert_eq!(color(true, false), FILL_UNINTERRUPTIBLE);
    }

    #[test]
    fn interrupted_state_shows_full_red_bar_with_retail_text() {
        let state = CastingBarState::interrupted();
        assert_eq!(bar_fill_color(&state, &MODERN_STYLE), FILL_INTERRUPTED);
        let reg = build_with_state(state);
        assert_eq!(fontstring_text(&reg, "CastingBarSpellName"), "Interrupted");
        let fill = reg.get(reg.get_by_name("CastingBarFill").unwrap()).unwrap();
        assert_eq!(fill.width, Dimension::Fixed(BAR_W));
    }

    // --- Coord validation ---

    #[test]
    fn coord_frame_docked_in_cluster_cast_area() {
        let reg = layout_reg(0.5);
        let r = rect(&reg, "PlayerCastingBarFrame");
        let expected_x = (1920.0 - MODERN_HOLDER_W) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y + r.height - (1080.0 - 152.0)).abs() < 1.0);
        assert!((r.width - CAST_DOCK_W).abs() < 1.0);
        assert!((r.height - MODERN_HOLDER_H).abs() < 1.0);
    }

    #[test]
    fn coord_bar_background_dimensions() {
        let reg = layout_reg(0.5);
        let r = rect(&reg, "CastingBarBackground");
        assert!((r.width - BAR_W).abs() < 1.0);
        assert!((r.height - BAR_H).abs() < 1.0);
    }

    #[test]
    fn coord_fill_bar_left_aligned_with_background() {
        let reg = layout_reg(0.5);
        let bg = rect(&reg, "CastingBarBackground");
        let fill = rect(&reg, "CastingBarFill");
        assert!(
            (fill.x - bg.x).abs() < 1.0,
            "fill should left-align with background"
        );
        assert!((fill.height - BAR_H).abs() < 1.0);
    }

    #[test]
    fn coord_spark_follows_fill_edge() {
        let reg = layout_reg(0.6);
        let bg = rect(&reg, "CastingBarBackground");
        let spark = rect(&reg, "CastingBarSpark");
        let expected_x = bg.x + BAR_W * 0.6 - SPARK_W / 2.0;
        assert!(
            (spark.x - expected_x).abs() < 1.0,
            "spark x: expected {expected_x}, got {}",
            spark.x
        );
        assert!((spark.width - SPARK_W).abs() < 1.0);
    }

    #[test]
    fn bar_children_follow_their_resized_parent() {
        let mut reg = build_registry(0.5);
        let background = reg.get_by_name("CastingBarBackground").unwrap();
        let frame = reg.get_mut(background).unwrap();
        frame.width = Dimension::Fixed(255.0);
        frame.height = Dimension::Fixed(30.0);
        compute_layout(&mut reg);
        let bg = rect(&reg, "CastingBarBackground");
        let fill = rect(&reg, "CastingBarFill");
        let timer = rect(&reg, "CastingBarTimer");
        let spell = rect(&reg, "CastingBarSpellName");
        assert!((fill.x - bg.x).abs() < 1.0);
        assert!((fill.y + fill.height / 2.0 - bg.y - bg.height / 2.0).abs() < 1.0);
        assert!((timer.x + timer.width - bg.x - bg.width + 2.0).abs() < 1.0);
        assert!((spell.x + spell.width / 2.0 - bg.x - bg.width / 2.0).abs() < 1.0);
    }

    // --- Text content tests ---

    fn build_with_state(state: CastingBarState) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
        shared.insert(state);
        Screen::new(casting_bar_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn spell_name_displayed() {
        let reg = build_registry(0.5);
        assert_eq!(fontstring_text(&reg, "CastingBarSpellName"), "Fireball");
    }

    #[test]
    fn timer_text_displayed() {
        let reg = build_registry(0.5);
        assert_eq!(fontstring_text(&reg, "CastingBarTimer"), "1.5s");
    }

    #[test]
    fn fill_width_zero_progress() {
        let reg = build_registry(0.0);
        let id = reg.get_by_name("CastingBarFill").expect("fill");
        let frame = reg.get(id).expect("data");
        assert_eq!(frame.width, Dimension::Fixed(0.0));
    }

    #[test]
    fn fill_width_full_progress() {
        let reg = build_registry(1.0);
        let id = reg.get_by_name("CastingBarFill").expect("fill");
        let frame = reg.get(id).expect("data");
        assert_eq!(frame.width, Dimension::Fixed(BAR_W));
    }

    #[test]
    fn fill_width_clamped_above_one() {
        let reg = build_registry(1.5);
        let id = reg.get_by_name("CastingBarFill").expect("fill");
        let frame = reg.get(id).expect("data");
        assert_eq!(frame.width, Dimension::Fixed(BAR_W));
    }

    #[test]
    fn channel_spell_name_displayed() {
        let state = CastingBarState {
            visible: true,
            spell_name: "Drain Life".into(),
            timer_text: "3.0s".into(),
            progress: 0.8,
            is_channel: true,
            is_interruptible: true,
            is_interrupted: false,
        };
        let reg = build_with_state(state);
        assert_eq!(fontstring_text(&reg, "CastingBarSpellName"), "Drain Life");
        assert_eq!(fontstring_text(&reg, "CastingBarTimer"), "3.0s");
    }

    #[test]
    fn channel_fill_width() {
        let state = CastingBarState {
            visible: true,
            spell_name: "Drain Life".into(),
            timer_text: "3.0s".into(),
            progress: 0.8,
            is_channel: true,
            is_interruptible: true,
            is_interrupted: false,
        };
        let reg = build_with_state(state);
        let id = reg.get_by_name("CastingBarFill").expect("fill");
        let frame = reg.get(id).expect("data");
        let expected = BAR_W * 0.8;
        assert_eq!(frame.width, Dimension::Fixed(expected));
    }

    #[test]
    fn spark_at_zero_progress() {
        let reg = layout_reg(0.0);
        let bg = rect(&reg, "CastingBarBackground");
        let spark = rect(&reg, "CastingBarSpark");
        let expected_x = bg.x - SPARK_W / 2.0;
        assert!(
            (spark.x - expected_x).abs() < 1.0,
            "spark at 0: expected {expected_x}, got {}",
            spark.x
        );
    }

    #[test]
    fn spark_at_full_progress() {
        let reg = layout_reg(1.0);
        let bg = rect(&reg, "CastingBarBackground");
        let spark = rect(&reg, "CastingBarSpark");
        let expected_x = bg.x + BAR_W - SPARK_W / 2.0;
        assert!(
            (spark.x - expected_x).abs() < 1.0,
            "spark at 1.0: expected {expected_x}, got {}",
            spark.x
        );
    }
}
