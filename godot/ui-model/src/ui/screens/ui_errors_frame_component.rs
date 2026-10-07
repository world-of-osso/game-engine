use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::ui_errors_data::{ERROR_RGB, ErrorLine, MAX_ERROR_LINES, UiErrorsData};
use crate::ui::widgets::font_string::{FontColor, GameFont};

pub const UI_ERRORS_FRAME: FrameName = FrameName("UIErrorsFrame");

/// Retail `UIErrorsFrame`: 512x60, TOP of UIParent at y -122.
const FRAME_W: f32 = 512.0;
const FRAME_TOP: f32 = 122.0;
const LINE_H: f32 = 20.0;
const FONT_SIZE: f32 = 16.0;

struct DynName(String);

pub fn ui_error_line_name(index: usize) -> String {
    format!("UIErrorsFrameLine{}", index + 1)
}

pub fn ui_errors_frame_screen(ctx: &SharedContext) -> Element {
    let errors = ctx
        .get::<UiErrorsData>()
        .expect("UiErrorsData must be in SharedContext");
    let lines: Element = (0..MAX_ERROR_LINES)
        .flat_map(|index| {
            let line = errors.lines.get(index);
            error_line(index, line)
        })
        .collect();
    rsx! {
        r#frame {
            name: UI_ERRORS_FRAME,
            width: FRAME_W,
            height: {LINE_H * MAX_ERROR_LINES as f32},
            strata: FrameStrata::Dialog,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: FRAME_TOP,
            {lines}
        }
    }
}

fn error_line(index: usize, line: Option<&ErrorLine>) -> Element {
    let hide = line.is_none();
    let text = line.map_or("", |line| line.text.as_str());
    let [r, g, b] = line.map_or(ERROR_RGB, |line| line.color);
    let color = FontColor::new(r, g, b, line.map_or(0.0, ErrorLine::alpha));
    rsx! {
        fontstring {
            name: {DynName(ui_error_line_name(index))},
            width: FRAME_W,
            height: LINE_H,
            text: text,
            font: GameFont::FrizQuadrata,
            font_size: FONT_SIZE,
            font_color: color,
            justify_h: "CENTER",
            strata: FrameStrata::Dialog,
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
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::Screen;

    fn build(errors: UiErrorsData) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(errors);
        Screen::new(ui_errors_frame_screen).sync(&shared, &mut reg);
        reg
    }

    fn hidden(reg: &FrameRegistry, name: &str) -> bool {
        reg.get(reg.get_by_name(name).expect(name)).unwrap().hidden
    }

    #[test]
    fn shows_newest_first_and_hides_unused_lines() {
        let mut errors = UiErrorsData::default();
        errors.add("Out of range.");
        errors.add("Invalid target");
        let reg = build(errors);
        assert!(reg.get_by_name("UIErrorsFrame").is_some());
        assert_eq!(
            fontstring_text(&reg, "UIErrorsFrameLine1"),
            "Invalid target"
        );
        assert_eq!(fontstring_text(&reg, "UIErrorsFrameLine2"), "Out of range.");
        assert!(!hidden(&reg, "UIErrorsFrameLine1"));
        assert!(!hidden(&reg, "UIErrorsFrameLine2"));
        assert!(hidden(&reg, "UIErrorsFrameLine3"));
    }

    #[test]
    fn quest_progress_uses_yellow_and_keeps_error_lines_red() {
        let mut errors = UiErrorsData::default();
        errors.add("Out of range.");
        errors.add_info("Blackrock Worg slain: 1/6");
        let reg = build(errors);
        for (name, expected_text, expected_color) in [
            (
                "UIErrorsFrameLine1",
                "Blackrock Worg slain: 1/6",
                [1.0, 1.0, 0.0, 1.0],
            ),
            ("UIErrorsFrameLine2", "Out of range.", [1.0, 0.1, 0.1, 1.0]),
        ] {
            let frame = reg.get(reg.get_by_name(name).unwrap()).unwrap();
            let Some(WidgetData::FontString(text)) = frame.widget_data.as_ref() else {
                panic!("message text missing");
            };
            assert_eq!(text.text, expected_text);
            assert_eq!(text.color, expected_color);
            assert!(!frame.hidden);
        }
    }

    #[test]
    fn fading_line_uses_red_with_line_alpha() {
        let mut errors = UiErrorsData::default();
        errors.add("Out of range.");
        errors.lines[0].age = 3.25;
        let reg = build(errors);
        let frame = reg
            .get(reg.get_by_name("UIErrorsFrameLine1").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(fs)) = frame.widget_data.as_ref() else {
            panic!("line is a FontString");
        };
        assert_eq!(fs.color[..3], ERROR_RGB);
        assert!((fs.color[3] - 0.5).abs() < 0.01);
    }
}
