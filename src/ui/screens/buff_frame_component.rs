use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

const ICON_SIZE: f32 = 30.0;
const ICON_GAP: f32 = 2.0;
const ICONS_PER_ROW: usize = 10;
const ROW_GAP: f32 = 14.0;
const DEBUFF_GAP: f32 = 4.0;
const TIMER_H: f32 = 12.0;
const STACK_SIZE: f32 = 14.0;
const TOOLTIP_W: f32 = 200.0;
const TOOLTIP_H: f32 = 60.0;

const BUFF_BG: &str = "0.0,0.0,0.0,0.5";
const DEBUFF_BG: &str = "0.4,0.0,0.0,0.5";
const TIMER_COLOR: &str = "1.0,1.0,1.0,0.9";
const STACK_COLOR: &str = "1.0,1.0,1.0,1.0";
const TOOLTIP_BG: &str = "0.0,0.0,0.0,0.92";
const TOOLTIP_TITLE_COLOR: &str = "1.0,1.0,1.0,1.0";
const TOOLTIP_DESC_COLOR: &str = "1.0,0.82,0.0,1.0";
const TOOLTIP_SOURCE_COLOR: &str = "0.6,0.6,0.6,1.0";

pub const MAX_BUFFS: usize = 32;
pub const MAX_DEBUFFS: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct BuffIconState {
    pub icon_fdid: u32,
    pub timer_text: String,
    /// Stack count (0 or 1 = hide).
    pub stacks: u32,
    /// Tooltip name.
    pub name: String,
    /// Tooltip description (e.g. "Increases haste by 5%").
    pub description: String,
    /// Source of the buff (e.g. caster name).
    pub source: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuffFrameState {
    pub buffs: Vec<BuffIconState>,
    pub debuffs: Vec<BuffIconState>,
}

pub fn buff_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<BuffFrameState>()
        .expect("BuffFrameState must be in SharedContext");
    rsx! {
        {buff_grid(&state.buffs)}
        {debuff_grid(&state.buffs, &state.debuffs)}
        {buff_tooltip()}
    }
}

fn buff_grid(buffs: &[BuffIconState]) -> Element {
    let grid_w = ICONS_PER_ROW as f32 * (ICON_SIZE + ICON_GAP) - ICON_GAP;
    let buff_rows = (buffs.len().min(MAX_BUFFS) + ICONS_PER_ROW - 1) / ICONS_PER_ROW.max(1);
    let grid_h = buff_rows.max(1) as f32 * (ICON_SIZE + ROW_GAP);
    let icons: Element = buffs
        .iter()
        .enumerate()
        .take(MAX_BUFFS)
        .flat_map(|(i, buff)| buff_icon(i, buff, "Buff"))
        .collect();
    rsx! {
        r#frame {
            name: "BuffFrame",
            width: {grid_w},
            height: {grid_h},
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: "-205",
                y: "-8",
            }
            {icons}
        }
    }
}

fn debuff_grid(buffs: &[BuffIconState], debuffs: &[BuffIconState]) -> Element {
    let grid_w = ICONS_PER_ROW as f32 * (ICON_SIZE + ICON_GAP) - ICON_GAP;
    let buff_rows = (buffs.len().min(MAX_BUFFS) + ICONS_PER_ROW - 1) / ICONS_PER_ROW.max(1);
    let debuff_y_offset = buff_rows.max(1) as f32 * (ICON_SIZE + ROW_GAP) + DEBUFF_GAP;
    let debuff_rows = (debuffs.len().min(MAX_DEBUFFS) + ICONS_PER_ROW - 1) / ICONS_PER_ROW.max(1);
    let grid_h = debuff_rows.max(1) as f32 * (ICON_SIZE + ROW_GAP);
    let icons: Element = debuffs
        .iter()
        .enumerate()
        .take(MAX_DEBUFFS)
        .flat_map(|(i, debuff)| buff_icon(i, debuff, "Debuff"))
        .collect();
    rsx! {
        r#frame {
            name: "DebuffFrame",
            width: {grid_w},
            height: {grid_h},
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: "-205",
                y: {-(8.0 + debuff_y_offset)},
            }
            {icons}
        }
    }
}

fn buff_icon(index: usize, buff: &BuffIconState, prefix: &str) -> Element {
    let col = index % ICONS_PER_ROW;
    let row = index / ICONS_PER_ROW;
    let x = col as f32 * (ICON_SIZE + ICON_GAP);
    let y = -(row as f32 * (ICON_SIZE + ROW_GAP));
    let icon_name = DynName(format!("{prefix}Icon{index}"));
    let bg = if prefix == "Buff" { BUFF_BG } else { DEBUFF_BG };
    let stack_text = if buff.stacks > 1 {
        format!("{}", buff.stacks)
    } else {
        String::new()
    };
    rsx! {
        r#frame {
            name: icon_name,
            width: {ICON_SIZE},
            height: {ICON_SIZE},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            {icon_timer(DynName(format!("{prefix}Icon{index}Timer")), &buff.timer_text)}
            {icon_stacks(DynName(format!("{prefix}Icon{index}Stack")), &stack_text)}
        }
    }
}

fn icon_timer(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {ICON_SIZE},
            height: {TIMER_H},
            text: text,
            font_size: 8.0,
            font_color: TIMER_COLOR,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::Bottom, relative_point: AnchorPoint::Bottom, x: "0", y: {TIMER_H} }
        }
    }
}

fn icon_stacks(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {STACK_SIZE},
            height: {STACK_SIZE},
            text: text,
            font_size: 10.0,
            font_color: STACK_COLOR,
            justify_h: "RIGHT",
            anchor { point: AnchorPoint::BottomRight, relative_point: AnchorPoint::BottomRight, x: "-1", y: "1" }
        }
    }
}

fn tooltip_line(name: DynName, h: f32, font_size: f32, color: &str, y: f32) -> Element {
    rsx! {
        fontstring {
            name: name,
            width: {TOOLTIP_W - 8.0},
            height: {h},
            text: "",
            font_size: font_size,
            font_color: color,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: {y} }
        }
    }
}

fn buff_tooltip() -> Element {
    use crate::ui::strata::FrameStrata;
    rsx! {
        r#frame {
            name: "BuffTooltip",
            width: {TOOLTIP_W},
            height: {TOOLTIP_H},
            background_color: TOOLTIP_BG,
            strata: FrameStrata::Tooltip,
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
            }
            {tooltip_line(DynName("BuffTooltipTitle".into()), 16.0, 11.0, TOOLTIP_TITLE_COLOR, -4.0)}
            {tooltip_line(DynName("BuffTooltipDesc".into()), 16.0, 9.0, TOOLTIP_DESC_COLOR, -22.0)}
            {tooltip_line(DynName("BuffTooltipSource".into()), 14.0, 8.0, TOOLTIP_SOURCE_COLOR, -40.0)}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_icon(timer: &str) -> BuffIconState {
        BuffIconState {
            icon_fdid: 12345,
            timer_text: timer.into(),
            stacks: 0,
            name: "Test Buff".into(),
            description: "Does something".into(),
            source: "Player".into(),
        }
    }

    fn make_state(buff_count: usize, debuff_count: usize) -> BuffFrameState {
        BuffFrameState {
            buffs: (0..buff_count)
                .map(|i| make_icon(&format!("{i}m")))
                .collect(),
            debuffs: (0..debuff_count)
                .map(|i| make_icon(&format!("{i}s")))
                .collect(),
        }
    }

    fn build_registry(buff_count: usize, debuff_count: usize) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_state(buff_count, debuff_count));
        Screen::new(buff_frame_screen).sync(&shared, &mut reg);
        reg
    }

    fn layout_reg(buff_count: usize, debuff_count: usize) -> FrameRegistry {
        let mut reg = build_registry(buff_count, debuff_count);
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    #[test]
    fn builds_buff_and_debuff_frames() {
        let reg = build_registry(3, 2);
        assert!(reg.get_by_name("BuffFrame").is_some());
        assert!(reg.get_by_name("DebuffFrame").is_some());
    }

    #[test]
    fn builds_buff_icons_with_timers() {
        let reg = build_registry(5, 0);
        for i in 0..5 {
            assert!(
                reg.get_by_name(&format!("BuffIcon{i}")).is_some(),
                "BuffIcon{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("BuffIcon{i}Timer")).is_some(),
                "BuffIcon{i}Timer missing"
            );
        }
        assert!(reg.get_by_name("BuffIcon5").is_none());
    }

    #[test]
    fn builds_debuff_icons_with_timers() {
        let reg = build_registry(0, 4);
        for i in 0..4 {
            assert!(
                reg.get_by_name(&format!("DebuffIcon{i}")).is_some(),
                "DebuffIcon{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("DebuffIcon{i}Timer")).is_some(),
                "DebuffIcon{i}Timer missing"
            );
        }
    }

    #[test]
    fn empty_state_builds_frames_only() {
        let reg = build_registry(0, 0);
        assert!(reg.get_by_name("BuffFrame").is_some());
        assert!(reg.get_by_name("DebuffFrame").is_some());
        assert!(reg.get_by_name("BuffIcon0").is_none());
        assert!(reg.get_by_name("DebuffIcon0").is_none());
    }

    // --- Coord validation ---

    #[test]
    fn coord_buff_frame_right_aligned() {
        let reg = layout_reg(5, 0);
        let r = rect(&reg, "BuffFrame");
        let grid_w = ICONS_PER_ROW as f32 * (ICON_SIZE + ICON_GAP) - ICON_GAP;
        let expected_x = 1920.0 - 205.0 - grid_w;
        assert!(
            (r.x - expected_x).abs() < 1.0,
            "x: expected {expected_x}, got {}",
            r.x
        );
        assert!((r.y - 8.0).abs() < 1.0);
    }

    #[test]
    fn coord_buff_icon_wraps_to_second_row() {
        let reg = layout_reg(12, 0);
        let first = rect(&reg, "BuffIcon0");
        let eleventh = rect(&reg, "BuffIcon10");
        let row_offset = ICON_SIZE + ROW_GAP;
        assert!(
            (eleventh.y - first.y - row_offset).abs() < 1.0,
            "second row y offset: expected {row_offset}, got {}",
            eleventh.y - first.y
        );
    }

    #[test]
    fn coord_debuff_below_buffs() {
        let reg = layout_reg(5, 3);
        let buff_frame = rect(&reg, "BuffFrame");
        let debuff_frame = rect(&reg, "DebuffFrame");
        assert!(
            debuff_frame.y > buff_frame.y,
            "debuff frame should be below buff frame"
        );
    }

    #[test]
    fn coord_icon_dimensions() {
        let reg = layout_reg(1, 1);
        let buff = rect(&reg, "BuffIcon0");
        assert!((buff.width - ICON_SIZE).abs() < 1.0);
        assert!((buff.height - ICON_SIZE).abs() < 1.0);
        let debuff = rect(&reg, "DebuffIcon0");
        assert!((debuff.width - ICON_SIZE).abs() < 1.0);
        assert!((debuff.height - ICON_SIZE).abs() < 1.0);
    }

    #[test]
    fn buff_icons_have_stack_count_overlay() {
        let reg = build_registry(3, 2);
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("BuffIcon{i}Stack")).is_some(),
                "BuffIcon{i}Stack missing"
            );
        }
        for i in 0..2 {
            assert!(
                reg.get_by_name(&format!("DebuffIcon{i}Stack")).is_some(),
                "DebuffIcon{i}Stack missing"
            );
        }
    }

    #[test]
    fn tooltip_frame_exists_and_hidden() {
        let reg = build_registry(1, 0);
        let id = reg.get_by_name("BuffTooltip").expect("BuffTooltip");
        let frame = reg.get(id).expect("data");
        assert!(frame.hidden, "tooltip should start hidden");
        assert!(reg.get_by_name("BuffTooltipTitle").is_some());
        assert!(reg.get_by_name("BuffTooltipDesc").is_some());
        assert!(reg.get_by_name("BuffTooltipSource").is_some());
    }

    #[test]
    fn coord_buff_icon_horizontal_spacing() {
        let reg = layout_reg(3, 0);
        let icon0 = rect(&reg, "BuffIcon0");
        let icon1 = rect(&reg, "BuffIcon1");
        let expected = ICON_SIZE + ICON_GAP;
        let actual = icon1.x - icon0.x;
        assert!(
            (actual - expected).abs() < 1.0,
            "icon spacing: expected {expected}, got {actual}"
        );
    }

    #[test]
    fn coord_debuff_frame_right_aligned() {
        let reg = layout_reg(0, 3);
        let r = rect(&reg, "DebuffFrame");
        let grid_w = ICONS_PER_ROW as f32 * (ICON_SIZE + ICON_GAP) - ICON_GAP;
        let expected_x = 1920.0 - 205.0 - grid_w;
        assert!(
            (r.x - expected_x).abs() < 1.0,
            "debuff x: expected {expected_x}, got {}",
            r.x
        );
    }

    // --- Text content tests ---

    fn fontstring_text(reg: &FrameRegistry, name: &str) -> String {
        use ui_toolkit::frame::WidgetData;
        let id = reg.get_by_name(name).expect(name);
        let frame = reg.get(id).expect("frame data");
        match frame.widget_data.as_ref() {
            Some(WidgetData::FontString(fs)) => fs.text.clone(),
            _ => panic!("{name} is not a FontString"),
        }
    }

    fn build_with_state(state: BuffFrameState) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(state);
        Screen::new(buff_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn timer_text_displayed() {
        let reg = build_registry(3, 0);
        assert_eq!(fontstring_text(&reg, "BuffIcon0Timer"), "0m");
        assert_eq!(fontstring_text(&reg, "BuffIcon1Timer"), "1m");
        assert_eq!(fontstring_text(&reg, "BuffIcon2Timer"), "2m");
    }

    #[test]
    fn stack_count_hidden_at_zero_and_one() {
        let state = BuffFrameState {
            buffs: vec![
                BuffIconState {
                    stacks: 0,
                    ..make_icon("5m")
                },
                BuffIconState {
                    stacks: 1,
                    ..make_icon("3m")
                },
            ],
            debuffs: vec![],
        };
        let reg = build_with_state(state);
        assert_eq!(fontstring_text(&reg, "BuffIcon0Stack"), "");
        assert_eq!(fontstring_text(&reg, "BuffIcon1Stack"), "");
    }

    #[test]
    fn stack_count_shown_above_one() {
        let state = BuffFrameState {
            buffs: vec![BuffIconState {
                stacks: 5,
                ..make_icon("10s")
            }],
            debuffs: vec![BuffIconState {
                stacks: 3,
                ..make_icon("8s")
            }],
        };
        let reg = build_with_state(state);
        assert_eq!(fontstring_text(&reg, "BuffIcon0Stack"), "5");
        assert_eq!(fontstring_text(&reg, "DebuffIcon0Stack"), "3");
    }

    #[test]
    fn debuff_timer_text() {
        let reg = build_registry(0, 2);
        assert_eq!(fontstring_text(&reg, "DebuffIcon0Timer"), "0s");
        assert_eq!(fontstring_text(&reg, "DebuffIcon1Timer"), "1s");
    }

    #[test]
    fn debuff_y_shifts_with_buff_row_count() {
        let one_row = layout_reg(5, 1);
        let two_rows = layout_reg(15, 1);
        let debuff_one = rect(&one_row, "DebuffFrame");
        let debuff_two = rect(&two_rows, "DebuffFrame");
        assert!(
            debuff_two.y > debuff_one.y,
            "more buff rows should push debuffs further down"
        );
        let row_step = ICON_SIZE + ROW_GAP;
        let shift = debuff_two.y - debuff_one.y;
        assert!(
            (shift - row_step).abs() < 1.0,
            "shift: expected {row_step}, got {shift}"
        );
    }

    #[test]
    fn max_buffs_capped() {
        let reg = build_registry(40, 0);
        for i in 0..MAX_BUFFS {
            assert!(
                reg.get_by_name(&format!("BuffIcon{i}")).is_some(),
                "BuffIcon{i} missing"
            );
        }
        assert!(reg.get_by_name(&format!("BuffIcon{MAX_BUFFS}")).is_none());
    }

    #[test]
    fn max_debuffs_capped() {
        let reg = build_registry(0, 20);
        for i in 0..MAX_DEBUFFS {
            assert!(
                reg.get_by_name(&format!("DebuffIcon{i}")).is_some(),
                "DebuffIcon{i} missing"
            );
        }
        assert!(
            reg.get_by_name(&format!("DebuffIcon{MAX_DEBUFFS}"))
                .is_none()
        );
    }
}
