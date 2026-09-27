use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::raid_warning::{MAX_MESSAGES, RaidWarnings};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

pub const RAID_WARNING_FRAME: FrameName = FrameName("RaidWarningFrame");

/// Retail `RaidWarningFrame`: 800 wide, TOP of UIParent at y -182, `GameFontNormalHuge`
/// lines (`MIN_TEXT_HEIGHT` 20 … `DEFAULT_MAX_TEXT_HEIGHT` 30).
const FRAME_W: f32 = 800.0;
const FRAME_TOP: f32 = 182.0;
const LINE_H: f32 = 30.0;
const FONT_SIZE: f32 = 20.0;

struct DynName(String);

pub fn raid_warning_line_name(index: usize) -> String {
    format!("RaidWarningFrameLine{}", index + 1)
}

pub fn raid_warning_frame_screen(ctx: &SharedContext) -> Element {
    let warnings = ctx
        .get::<RaidWarnings>()
        .expect("RaidWarnings must be in SharedContext");
    let lines: Element = (0..MAX_MESSAGES)
        .flat_map(|index| warning_line(index, warnings.lines.get(index)))
        .collect();
    rsx! {
        r#frame {
            name: RAID_WARNING_FRAME,
            width: FRAME_W,
            height: {LINE_H * MAX_MESSAGES as f32},
            strata: FrameStrata::High,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: FRAME_TOP,
            {lines}
        }
    }
}

fn warning_line(index: usize, line: Option<&crate::ui::raid_warning::RaidWarningLine>) -> Element {
    let hide = line.is_none();
    let text = line.map_or("", |line| line.text.as_str());
    let [r, g, b] = line.map_or([1.0; 3], |line| line.color);
    let color = FontColor::new(r, g, b, line.map_or(0.0, |line| line.alpha()));
    rsx! {
        fontstring {
            name: {DynName(raid_warning_line_name(index))},
            width: FRAME_W,
            height: LINE_H,
            text: text,
            font: GameFont::FrizQuadrata,
            font_size: FONT_SIZE,
            font_color: color,
            justify_h: "CENTER",
            strata: FrameStrata::High,
            hidden: hide,
            pos_type: "absolute",
            left: 0.0,
            top: {LINE_H * index as f32},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::screen_test_helpers::fontstring_text;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::Screen;

    #[test]
    fn boss_emotes_show_oldest_first_and_unused_lines_hide() {
        let mut warnings = RaidWarnings::default();
        warnings.add("Hogger enrages!", [1.0, 0.867, 0.0]);
        warnings.add("Randolph Moloch vanishes!", [1.0, 0.867, 0.0]);
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(warnings);
        Screen::new(raid_warning_frame_screen).sync(&shared, &mut reg);
        assert_eq!(
            fontstring_text(&reg, "RaidWarningFrameLine1"),
            "Hogger enrages!"
        );
        assert_eq!(
            fontstring_text(&reg, "RaidWarningFrameLine2"),
            "Randolph Moloch vanishes!"
        );
        let hidden = |name: &str| reg.get(reg.get_by_name(name).unwrap()).unwrap().hidden;
        assert!(!hidden("RaidWarningFrameLine2"));
        assert!(hidden("RaidWarningFrameLine3"));
    }
}
