use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

const FRAME_W: f32 = 430.0;
const FRAME_H: f32 = 470.0;
const HEADER_H: f32 = 30.0;
const INSET: f32 = 10.0;
const PANEL_GAP: f32 = 10.0;
const PANEL_W: f32 = (FRAME_W - 2.0 * INSET - PANEL_GAP) / 2.0;
const PANEL_H: f32 = FRAME_H - HEADER_H - 48.0;
const ROW_H: f32 = 18.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const PANEL_BG: &str = "0.0,0.0,0.0,0.35";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const TEXT: &str = "1.0,1.0,1.0,1.0";
const SUBTLE: &str = "0.8,0.8,0.8,1.0";
const HEADER_BG: &str = "0.12,0.1,0.08,0.9";
const HEADER_TEXT: &str = "0.8,0.8,0.8,1.0";
const ROW_EVEN: &str = "0.04,0.04,0.04,0.6";
const ROW_ODD: &str = "0.06,0.06,0.06,0.6";

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectEquipmentRow {
    pub slot_name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectTalentRow {
    pub name: String,
    pub points_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct InspectFrameState {
    pub visible: bool,
    pub target_name: String,
    pub status_text: String,
    pub spec_summary: String,
    pub points_remaining: u16,
    pub equipment_rows: Vec<InspectEquipmentRow>,
    pub talent_rows: Vec<InspectTalentRow>,
}

pub fn inspect_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<InspectFrameState>()
        .expect("InspectFrameState must be in SharedContext");
    let hide = !state.visible;
    let title = inspect_frame_title(state);
    rsx! {
        r#frame {
            name: "InspectFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            hidden: hide,
            strata: FrameStrata::Dialog,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "830",
                y: "-80",
            }
            {title_bar(&title)}
            {inspect_frame_summary(state)}
            {equipment_panel(&state.equipment_rows)}
            {talent_panel(&state.talent_rows)}
        }
    }
}

fn inspect_frame_title(state: &InspectFrameState) -> String {
    if state.target_name.is_empty() {
        "Inspect".to_string()
    } else {
        format!("Inspect - {}", state.target_name)
    }
}

fn inspect_frame_summary(state: &InspectFrameState) -> Element {
    let specs_text = format!("Specs: {}", state.spec_summary);
    let points_text = format!("Points Remaining: {}", state.points_remaining);
    rsx! {
        {summary_line("InspectFrameSpecs", &specs_text, HEADER_H + 2.0)}
        {summary_line("InspectFramePoints", &points_text, HEADER_H + 18.0)}
        {summary_line("InspectFrameStatus", &state.status_text, HEADER_H + 34.0)}
    }
}

fn title_bar(title: &str) -> Element {
    rsx! {
        fontstring {
            name: "InspectFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: title,
            font_size: 16.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top, x: "0", y: "0" }
        }
    }
}

fn summary_line(name: &str, text: &str, y_offset: f32) -> Element {
    let name = DynName(name.to_string());
    rsx! {
        fontstring {
            name: name,
            width: {FRAME_W - 2.0 * INSET},
            height: 14.0,
            text: text,
            font_size: 10.0,
            font_color: SUBTLE,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {-y_offset},
            }
        }
    }
}

fn equipment_panel(rows: &[InspectEquipmentRow]) -> Element {
    let content: Element = rows
        .iter()
        .enumerate()
        .flat_map(|(index, row)| equipment_row(index, row))
        .collect();
    rsx! {
        r#frame {
            name: "InspectEquipmentPanel",
            width: {PANEL_W},
            height: {PANEL_H},
            background_color: PANEL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {-(HEADER_H + 56.0)},
            }
            {panel_header("InspectEquipmentHeader", "Equipment")}
            {content}
        }
    }
}

fn talent_panel(rows: &[InspectTalentRow]) -> Element {
    let content = talent_panel_content(rows);
    rsx! {
        r#frame {
            name: "InspectTalentPanel",
            width: {PANEL_W},
            height: {PANEL_H},
            background_color: PANEL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET + PANEL_W + PANEL_GAP},
                y: {-(HEADER_H + 56.0)},
            }
            {panel_header("InspectTalentHeader", "Talents")}
            {content}
        }
    }
}

fn talent_panel_content(rows: &[InspectTalentRow]) -> Element {
    if rows.is_empty() {
        talent_panel_empty_state()
    } else {
        rows.iter()
            .enumerate()
            .flat_map(|(index, row)| talent_row(index, row))
            .collect()
    }
}

fn talent_panel_empty_state() -> Element {
    rsx! {
        fontstring {
            name: "InspectTalentEmpty",
            width: {PANEL_W - 8.0},
            height: 14.0,
            text: "-",
            font_size: 10.0,
            font_color: SUBTLE,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "4",
                y: {-(ROW_H + 4.0)},
            }
        }
    }
}

fn panel_header(name: &str, text: &str) -> Element {
    let name = DynName(name.to_string());
    rsx! {
        r#frame {
            name: name,
            width: {PANEL_W},
            height: {ROW_H},
            background_color: HEADER_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            fontstring {
                name: DynName(format!("{}Label", name.0)),
                width: {PANEL_W},
                height: {ROW_H},
                text: text,
                font_size: 10.0,
                font_color: HEADER_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn equipment_row(index: usize, row: &InspectEquipmentRow) -> Element {
    let bg = if index.is_multiple_of(2) {
        ROW_EVEN
    } else {
        ROW_ODD
    };
    rsx! {
        r#frame {
            name: DynName(format!("InspectEquipmentRow{index}")),
            width: {PANEL_W},
            height: {ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {-((index + 1) as f32 * ROW_H)},
            }
            fontstring {
                name: DynName(format!("InspectEquipmentRow{index}Slot")),
                width: {72.0},
                height: {ROW_H},
                text: {row.slot_name.as_str()},
                font_size: 9.0,
                font_color: HEADER_TEXT,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
            }
            fontstring {
                name: DynName(format!("InspectEquipmentRow{index}Value")),
                width: {PANEL_W - 80.0},
                height: {ROW_H},
                text: {row.value.as_str()},
                font_size: 9.0,
                font_color: TEXT,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "76", y: "0" }
            }
        }
    }
}

fn talent_row(index: usize, row: &InspectTalentRow) -> Element {
    let bg = if index.is_multiple_of(2) {
        ROW_EVEN
    } else {
        ROW_ODD
    };
    rsx! {
        r#frame {
            name: DynName(format!("InspectTalentRow{index}")),
            width: {PANEL_W},
            height: {ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {-((index + 1) as f32 * ROW_H)},
            }
            fontstring {
                name: DynName(format!("InspectTalentRow{index}Name")),
                width: {PANEL_W - 44.0},
                height: {ROW_H},
                text: {row.name.as_str()},
                font_size: 9.0,
                font_color: TEXT,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
            }
            fontstring {
                name: DynName(format!("InspectTalentRow{index}Points")),
                width: {36.0},
                height: {ROW_H},
                text: {row.points_text.as_str()},
                font_size: 9.0,
                font_color: TITLE_COLOR,
                justify_h: "RIGHT",
                anchor { point: AnchorPoint::TopRight, relative_point: AnchorPoint::TopRight, x: "-4", y: "0" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    #[test]
    fn inspect_frame_renders_equipment_rows() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(InspectFrameState {
            visible: true,
            target_name: "Alice".into(),
            status_text: "inspect ready".into(),
            spec_summary: "Protection".into(),
            points_remaining: 50,
            equipment_rows: vec![InspectEquipmentRow {
                slot_name: "Head".into(),
                value: "item 100 / display 200".into(),
            }],
            talent_rows: Vec::new(),
        });
        Screen::new(inspect_frame_screen).sync(&shared, &mut registry);

        let value = registry
            .get(
                registry
                    .get_by_name("InspectEquipmentRow0Value")
                    .expect("equipment value frame"),
            )
            .expect("equipment value");
        let Some(WidgetData::FontString(text)) = value.widget_data.as_ref() else {
            panic!("expected fontstring");
        };
        assert_eq!(text.text, "item 100 / display 200");
    }

    #[test]
    fn inspect_frame_renders_talent_rows() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(InspectFrameState {
            visible: true,
            target_name: "Alice".into(),
            status_text: String::new(),
            spec_summary: "Protection".into(),
            points_remaining: 50,
            equipment_rows: Vec::new(),
            talent_rows: vec![InspectTalentRow {
                name: "Divine Strength".into(),
                points_text: "1/1".into(),
            }],
        });
        Screen::new(inspect_frame_screen).sync(&shared, &mut registry);

        let value = registry
            .get(
                registry
                    .get_by_name("InspectTalentRow0Points")
                    .expect("talent points frame"),
            )
            .expect("talent points");
        let Some(WidgetData::FontString(text)) = value.widget_data.as_ref() else {
            panic!("expected fontstring");
        };
        assert_eq!(text.text, "1/1");
    }
}
