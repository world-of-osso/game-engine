use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// --- Layout constants ---

pub const FRAME_W: f32 = 500.0;
pub const FRAME_H: f32 = 480.0;
const HEADER_H: f32 = 28.0;
const INSET: f32 = 8.0;
const CONTENT_TOP: f32 = HEADER_H + 4.0;
const LIST_W: f32 = FRAME_W - 2.0 * INSET;

const CAT_HEADER_H: f32 = 22.0;
const FACTION_ROW_H: f32 = 24.0;
const ROW_GAP: f32 = 2.0;
const FACTION_INDENT: f32 = 16.0;

const BAR_H: f32 = 12.0;
const BAR_W: f32 = 200.0;
const BAR_X: f32 = LIST_W - BAR_W - 8.0;

const STANDING_LABEL_W: f32 = 80.0;

// --- Colors ---

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const LIST_BG: &str = "0.0,0.0,0.0,0.3";
const CAT_HEADER_BG: &str = "0.12,0.10,0.06,0.9";
const CAT_HEADER_COLOR: &str = "1.0,0.82,0.0,1.0";
const FACTION_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const BAR_BG: &str = "0.1,0.1,0.1,0.9";
const BAR_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const COLLAPSE_ICON_COLOR: &str = "0.8,0.8,0.8,1.0";

const STANDING_HATED: &str = "0.8,0.2,0.2,0.95";
const STANDING_HOSTILE: &str = "0.8,0.3,0.2,0.95";
const STANDING_UNFRIENDLY: &str = "0.7,0.4,0.2,0.95";
const STANDING_NEUTRAL: &str = "0.7,0.7,0.2,0.95";
const STANDING_FRIENDLY: &str = "0.2,0.7,0.2,0.95";
const STANDING_HONORED: &str = "0.2,0.7,0.4,0.95";
const STANDING_REVERED: &str = "0.2,0.5,0.8,0.95";
const STANDING_EXALTED: &str = "0.6,0.3,0.9,0.95";

// --- Data types ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Standing {
    Hated,
    Hostile,
    Unfriendly,
    #[default]
    Neutral,
    Friendly,
    Honored,
    Revered,
    Exalted,
}

impl Standing {
    pub fn label(self) -> &'static str {
        match self {
            Self::Hated => "Hated",
            Self::Hostile => "Hostile",
            Self::Unfriendly => "Unfriendly",
            Self::Neutral => "Neutral",
            Self::Friendly => "Friendly",
            Self::Honored => "Honored",
            Self::Revered => "Revered",
            Self::Exalted => "Exalted",
        }
    }

    pub fn bar_color(self) -> &'static str {
        match self {
            Self::Hated => STANDING_HATED,
            Self::Hostile => STANDING_HOSTILE,
            Self::Unfriendly => STANDING_UNFRIENDLY,
            Self::Neutral => STANDING_NEUTRAL,
            Self::Friendly => STANDING_FRIENDLY,
            Self::Honored => STANDING_HONORED,
            Self::Revered => STANDING_REVERED,
            Self::Exalted => STANDING_EXALTED,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FactionEntry {
    pub name: String,
    pub standing: Standing,
    pub current: u32,
    pub max: u32,
}

impl FactionEntry {
    pub fn progress_fraction(&self) -> f32 {
        if self.max == 0 {
            return if self.standing == Standing::Exalted {
                1.0
            } else {
                0.0
            };
        }
        (self.current as f32 / self.max as f32).min(1.0)
    }

    pub fn progress_text(&self) -> String {
        format!("{}/{}", self.current, self.max)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FactionCategory {
    pub name: String,
    pub collapsed: bool,
    pub factions: Vec<FactionEntry>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ReputationFrameState {
    pub visible: bool,
    pub categories: Vec<FactionCategory>,
}

// --- Screen entry ---

pub fn reputation_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<ReputationFrameState>()
        .expect("ReputationFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "ReputationFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                x: "0",
                y: "0",
            }
            {title_bar()}
            {faction_list(&state.categories)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "ReputationFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Reputation",
            font_size: 16.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "0",
                y: "0",
            }
        }
    }
}

// --- Faction list ---

fn faction_list(categories: &[FactionCategory]) -> Element {
    let list_h = FRAME_H - CONTENT_TOP - INSET;
    let positions = category_positions(categories);
    let rows: Element = positions
        .iter()
        .flat_map(|&(ci, header_y, ref faction_positions)| {
            let cat = &categories[ci];
            let mut elems = category_header(ci, &cat.name, cat.collapsed, header_y);
            if !cat.collapsed {
                for &(fi, fy) in faction_positions {
                    elems.extend(faction_row(ci, fi, &cat.factions[fi], fy));
                }
            }
            elems
        })
        .collect();
    rsx! {
        r#frame {
            name: "ReputationList",
            width: {LIST_W},
            height: {list_h},
            background_color: LIST_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {-CONTENT_TOP},
            }
            {rows}
        }
    }
}

fn category_positions(cats: &[FactionCategory]) -> Vec<(usize, f32, Vec<(usize, f32)>)> {
    let mut y: f32 = 0.0;
    cats.iter()
        .enumerate()
        .map(|(ci, cat)| {
            let header_y = y;
            y += CAT_HEADER_H + ROW_GAP;
            let faction_pos = if cat.collapsed {
                vec![]
            } else {
                cat.factions
                    .iter()
                    .enumerate()
                    .map(|(fi, _)| {
                        let fy = y;
                        y += FACTION_ROW_H + ROW_GAP;
                        (fi, fy)
                    })
                    .collect()
            };
            (ci, header_y, faction_pos)
        })
        .collect()
}

fn category_header(idx: usize, name: &str, collapsed: bool, y: f32) -> Element {
    let id = DynName(format!("RepCat{idx}"));
    let label_id = DynName(format!("RepCat{idx}Label"));
    let icon_id = DynName(format!("RepCat{idx}Icon"));
    let icon_text = if collapsed { "▶" } else { "▼" };
    rsx! {
        r#frame {
            name: id,
            width: {LIST_W - 4.0},
            height: {CAT_HEADER_H},
            background_color: CAT_HEADER_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "2",
                y: {-y},
            }
            fontstring {
                name: icon_id,
                width: 14.0,
                height: {CAT_HEADER_H},
                text: icon_text,
                font_size: 10.0,
                font_color: COLLAPSE_ICON_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "0",
                }
            }
            fontstring {
                name: label_id,
                width: {LIST_W - 26.0},
                height: {CAT_HEADER_H},
                text: name,
                font_size: 11.0,
                font_color: CAT_HEADER_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "20",
                    y: "0",
                }
            }
        }
    }
}

fn faction_row(cat_idx: usize, fac_idx: usize, faction: &FactionEntry, y: f32) -> Element {
    let row_id = DynName(format!("RepFaction{cat_idx}_{fac_idx}"));
    let name_id = DynName(format!("RepFaction{cat_idx}_{fac_idx}Name"));
    rsx! {
        r#frame {
            name: row_id,
            width: {LIST_W - 4.0},
            height: {FACTION_ROW_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "2",
                y: {-y},
            }
            fontstring {
                name: name_id,
                width: {BAR_X - FACTION_INDENT - 4.0},
                height: {FACTION_ROW_H},
                text: {faction.name.as_str()},
                font_size: 10.0,
                font_color: FACTION_NAME_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {FACTION_INDENT},
                    y: "0",
                }
            }
            {reputation_bar(cat_idx, fac_idx, faction)}
        }
    }
}

fn reputation_bar(cat_idx: usize, fac_idx: usize, faction: &FactionEntry) -> Element {
    let bar_id = DynName(format!("RepBar{cat_idx}_{fac_idx}"));
    let fill_id = DynName(format!("RepBar{cat_idx}_{fac_idx}Fill"));
    let text_id = DynName(format!("RepBar{cat_idx}_{fac_idx}Text"));
    let standing_id = DynName(format!("RepBar{cat_idx}_{fac_idx}Standing"));
    let fill_w = faction.progress_fraction() * BAR_W;
    let bar_y = (FACTION_ROW_H - BAR_H) / 2.0;
    let progress = faction.progress_text();
    rsx! {
        r#frame {
            name: bar_id,
            width: {BAR_W},
            height: {BAR_H},
            background_color: BAR_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {BAR_X},
                y: {-bar_y},
            }
            r#frame {
                name: fill_id,
                width: {fill_w},
                height: {BAR_H},
                background_color: {faction.standing.bar_color()},
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "0",
                    y: "0",
                }
            }
            fontstring {
                name: text_id,
                width: {BAR_W},
                height: {BAR_H},
                text: {progress.as_str()},
                font_size: 8.0,
                font_color: BAR_TEXT_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "0",
                    y: "0",
                }
            }
        }
        fontstring {
            name: standing_id,
            width: {STANDING_LABEL_W},
            height: {FACTION_ROW_H},
            text: {faction.standing.label()},
            font_size: 9.0,
            font_color: {faction.standing.bar_color()},
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: "-4",
                y: "0",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn sample_categories() -> Vec<FactionCategory> {
        vec![
            FactionCategory {
                name: "Alliance".into(),
                collapsed: false,
                factions: vec![
                    FactionEntry {
                        name: "Stormwind".into(),
                        standing: Standing::Honored,
                        current: 8000,
                        max: 12000,
                    },
                    FactionEntry {
                        name: "Ironforge".into(),
                        standing: Standing::Friendly,
                        current: 3000,
                        max: 6000,
                    },
                ],
            },
            FactionCategory {
                name: "Horde".into(),
                collapsed: true,
                factions: vec![FactionEntry {
                    name: "Orgrimmar".into(),
                    standing: Standing::Hated,
                    current: 0,
                    max: 36000,
                }],
            },
            FactionCategory {
                name: "Neutral".into(),
                collapsed: false,
                factions: vec![FactionEntry {
                    name: "Cenarion Circle".into(),
                    standing: Standing::Exalted,
                    current: 0,
                    max: 0,
                }],
            },
        ]
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(ReputationFrameState {
            visible: true,
            categories: sample_categories(),
        });
        Screen::new(reputation_frame_screen).sync(&shared, &mut reg);
        reg
    }

    fn layout_registry() -> FrameRegistry {
        let mut reg = build_registry();
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    // --- Structure tests ---

    #[test]
    fn builds_frame_and_title() {
        let reg = build_registry();
        assert!(reg.get_by_name("ReputationFrame").is_some());
        assert!(reg.get_by_name("ReputationFrameTitle").is_some());
        assert!(reg.get_by_name("ReputationList").is_some());
    }

    #[test]
    fn builds_category_headers() {
        let reg = build_registry();
        for i in 0..3 {
            assert!(reg.get_by_name(&format!("RepCat{i}")).is_some());
            assert!(reg.get_by_name(&format!("RepCat{i}Label")).is_some());
            assert!(reg.get_by_name(&format!("RepCat{i}Icon")).is_some());
        }
    }

    #[test]
    fn builds_faction_rows_for_expanded() {
        let reg = build_registry();
        // Category 0 (Alliance) expanded: 2 factions
        assert!(reg.get_by_name("RepFaction0_0").is_some());
        assert!(reg.get_by_name("RepFaction0_0Name").is_some());
        assert!(reg.get_by_name("RepFaction0_1").is_some());
        // Category 2 (Neutral) expanded: 1 faction
        assert!(reg.get_by_name("RepFaction2_0").is_some());
    }

    #[test]
    fn collapsed_category_hides_factions() {
        let reg = build_registry();
        // Category 1 (Horde) collapsed — no faction rows
        assert!(reg.get_by_name("RepFaction1_0").is_none());
    }

    #[test]
    fn builds_reputation_bars() {
        let reg = build_registry();
        assert!(reg.get_by_name("RepBar0_0").is_some());
        assert!(reg.get_by_name("RepBar0_0Fill").is_some());
        assert!(reg.get_by_name("RepBar0_0Text").is_some());
        assert!(reg.get_by_name("RepBar0_0Standing").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(ReputationFrameState::default());
        Screen::new(reputation_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("ReputationFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Data model tests ---

    #[test]
    fn standing_labels() {
        assert_eq!(Standing::Hated.label(), "Hated");
        assert_eq!(Standing::Neutral.label(), "Neutral");
        assert_eq!(Standing::Exalted.label(), "Exalted");
    }

    #[test]
    fn standing_bar_colors_non_empty() {
        for standing in [
            Standing::Hated,
            Standing::Hostile,
            Standing::Unfriendly,
            Standing::Neutral,
            Standing::Friendly,
            Standing::Honored,
            Standing::Revered,
            Standing::Exalted,
        ] {
            assert!(!standing.bar_color().is_empty());
        }
    }

    #[test]
    fn faction_progress_fraction() {
        let f = FactionEntry {
            name: "X".into(),
            standing: Standing::Friendly,
            current: 3000,
            max: 6000,
        };
        assert!((f.progress_fraction() - 0.5).abs() < 0.01);
    }

    #[test]
    fn exalted_progress_is_full() {
        let f = FactionEntry {
            name: "X".into(),
            standing: Standing::Exalted,
            current: 0,
            max: 0,
        };
        assert_eq!(f.progress_fraction(), 1.0);
    }

    #[test]
    fn faction_progress_text() {
        let f = FactionEntry {
            name: "X".into(),
            standing: Standing::Neutral,
            current: 1500,
            max: 3000,
        };
        assert_eq!(f.progress_text(), "1500/3000");
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_registry();
        let r = rect(&reg, "ReputationFrame");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_list_panel() {
        let reg = layout_registry();
        let frame_r = rect(&reg, "ReputationFrame");
        let list_r = rect(&reg, "ReputationList");
        assert!((list_r.x - (frame_r.x + INSET)).abs() < 1.0);
        assert!((list_r.y - (frame_r.y + CONTENT_TOP)).abs() < 1.0);
        assert!((list_r.width - LIST_W).abs() < 1.0);
    }

    #[test]
    fn coord_first_category_at_top() {
        let reg = layout_registry();
        let list_r = rect(&reg, "ReputationList");
        let cat_r = rect(&reg, "RepCat0");
        assert!((cat_r.y - list_r.y).abs() < 1.0);
        assert!((cat_r.height - CAT_HEADER_H).abs() < 1.0);
    }

    #[test]
    fn coord_faction_row_below_header() {
        let reg = layout_registry();
        let cat_r = rect(&reg, "RepCat0");
        let row_r = rect(&reg, "RepFaction0_0");
        let expected_y = cat_r.y + CAT_HEADER_H + ROW_GAP;
        assert!((row_r.y - expected_y).abs() < 1.0);
        assert!((row_r.height - FACTION_ROW_H).abs() < 1.0);
    }

    #[test]
    fn coord_reputation_bar_position() {
        let reg = layout_registry();
        let row_r = rect(&reg, "RepFaction0_0");
        let bar_r = rect(&reg, "RepBar0_0");
        assert!((bar_r.x - (row_r.x + BAR_X)).abs() < 1.0);
        assert!((bar_r.width - BAR_W).abs() < 1.0);
        assert!((bar_r.height - BAR_H).abs() < 1.0);
    }

    #[test]
    fn coord_bar_fill_proportional() {
        let reg = layout_registry();
        let fill_r = rect(&reg, "RepBar0_0Fill");
        // Stormwind: 8000/12000 ≈ 0.667
        let expected_w = (8000.0 / 12000.0) * BAR_W;
        assert!((fill_r.width - expected_w).abs() < 1.0);
    }

    #[test]
    fn coord_collapsed_skips_faction_space() {
        let reg = layout_registry();
        // Cat 1 (Horde) is collapsed, cat 2 (Neutral) header should follow immediately
        let cat1_r = rect(&reg, "RepCat1");
        let cat2_r = rect(&reg, "RepCat2");
        let expected_y = cat1_r.y + CAT_HEADER_H + ROW_GAP;
        assert!((cat2_r.y - expected_y).abs() < 1.0);
    }
}
