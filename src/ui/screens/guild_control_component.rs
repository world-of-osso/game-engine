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

pub const FRAME_W: f32 = 500.0;
pub const FRAME_H: f32 = 440.0;
const HEADER_H: f32 = 28.0;
const SIDEBAR_W: f32 = 140.0;
const SIDEBAR_INSET: f32 = 8.0;
const RANK_ROW_H: f32 = 24.0;
const RANK_ROW_GAP: f32 = 1.0;
const EDITOR_H: f32 = 26.0;
const EDITOR_INSET: f32 = 4.0;
const CHECKBOX_SIZE: f32 = 18.0;
const CHECKBOX_GAP: f32 = 4.0;
const PERM_ROW_H: f32 = 22.0;
const PERM_ROW_GAP: f32 = 2.0;
const PERM_LABEL_W: f32 = 180.0;
const CONTENT_GAP: f32 = 4.0;
const CONTENT_TOP: f32 = HEADER_H + EDITOR_INSET;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const SIDEBAR_BG: &str = "0.0,0.0,0.0,0.4";
const RANK_SELECTED_BG: &str = "0.2,0.15,0.05,0.95";
const RANK_NORMAL_BG: &str = "0.0,0.0,0.0,0.0";
const RANK_SELECTED_COLOR: &str = "1.0,0.82,0.0,1.0";
const RANK_NORMAL_COLOR: &str = "1.0,1.0,1.0,1.0";
const EDITOR_BG: &str = "0.1,0.1,0.1,0.9";
const EDITOR_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const EDITOR_LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";
const CHECKBOX_BG: &str = "0.1,0.1,0.1,0.9";
const CHECKBOX_CHECK_COLOR: &str = "0.0,1.0,0.0,1.0";
const PERM_LABEL_COLOR: &str = "1.0,1.0,1.0,1.0";

pub const MAX_RANKS: usize = 10;
pub const MAX_PERMISSIONS: usize = 12;

#[derive(Clone, Debug, PartialEq)]
pub struct GuildRank {
    pub name: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PermissionRow {
    pub label: String,
    pub checked: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuildControlState {
    pub visible: bool,
    pub ranks: Vec<GuildRank>,
    pub rank_name: String,
    pub permissions: Vec<PermissionRow>,
}

impl Default for GuildControlState {
    fn default() -> Self {
        Self {
            visible: false,
            ranks: vec![],
            rank_name: String::new(),
            permissions: vec![],
        }
    }
}

pub fn guild_control_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<GuildControlState>()
        .expect("GuildControlState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "GuildControlFrame",
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
            {rank_sidebar(&state.ranks)}
            {rank_name_editor(&state.rank_name)}
            {permissions_grid(&state.permissions)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "GuildControlTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Guild Control",
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

fn rank_sidebar(ranks: &[GuildRank]) -> Element {
    let sidebar_y = -CONTENT_TOP;
    let sidebar_h = FRAME_H - CONTENT_TOP - SIDEBAR_INSET;
    let rows: Element = ranks
        .iter()
        .enumerate()
        .take(MAX_RANKS)
        .flat_map(|(i, rank)| rank_row(i, rank))
        .collect();
    rsx! {
        r#frame {
            name: "GuildControlRankSidebar",
            width: {SIDEBAR_W},
            height: {sidebar_h},
            background_color: SIDEBAR_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SIDEBAR_INSET},
                y: {sidebar_y},
            }
            {rows}
        }
    }
}

fn rank_row(idx: usize, rank: &GuildRank) -> Element {
    let row_id = DynName(format!("GuildControlRank{idx}"));
    let label_id = DynName(format!("GuildControlRank{idx}Label"));
    let bg = if rank.selected {
        RANK_SELECTED_BG
    } else {
        RANK_NORMAL_BG
    };
    let color = if rank.selected {
        RANK_SELECTED_COLOR
    } else {
        RANK_NORMAL_COLOR
    };
    let y = -(idx as f32 * (RANK_ROW_H + RANK_ROW_GAP));
    rsx! {
        r#frame {
            name: row_id,
            width: {SIDEBAR_W},
            height: {RANK_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
            fontstring {
                name: label_id,
                width: {SIDEBAR_W - 8.0},
                height: {RANK_ROW_H},
                text: {rank.name.as_str()},
                font_size: 10.0,
                font_color: color,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "0",
                }
            }
        }
    }
}

fn rank_name_editor(name: &str) -> Element {
    let editor_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
    let editor_y = -CONTENT_TOP;
    let editor_w = FRAME_W - editor_x - SIDEBAR_INSET;
    rsx! {
        fontstring {
            name: "GuildControlRankNameLabel",
            width: 80.0,
            height: {EDITOR_H},
            text: "Rank Name:",
            font_size: 10.0,
            font_color: EDITOR_LABEL_COLOR,
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {editor_x},
                y: {editor_y},
            }
        }
        r#frame {
            name: "GuildControlRankNameEditor",
            width: {editor_w - 84.0},
            height: {EDITOR_H},
            background_color: EDITOR_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {editor_x + 84.0},
                y: {editor_y},
            }
            fontstring {
                name: "GuildControlRankNameText",
                width: {editor_w - 92.0},
                height: {EDITOR_H},
                text: name,
                font_size: 10.0,
                font_color: EDITOR_TEXT_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "0",
                }
            }
        }
    }
}

fn permissions_grid(permissions: &[PermissionRow]) -> Element {
    let grid_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
    let grid_y = -(CONTENT_TOP + EDITOR_H + EDITOR_INSET);
    permissions
        .iter()
        .enumerate()
        .take(MAX_PERMISSIONS)
        .flat_map(|(i, perm)| {
            let y = grid_y - i as f32 * (PERM_ROW_H + PERM_ROW_GAP);
            permission_row(i, perm, grid_x, y)
        })
        .collect()
}

fn permission_row(idx: usize, perm: &PermissionRow, x: f32, y: f32) -> Element {
    let cb_id = DynName(format!("GuildControlPerm{idx}Check"));
    let label_id = DynName(format!("GuildControlPerm{idx}Label"));
    let check_text = if perm.checked { "\u{2713}" } else { "" };
    rsx! {
        r#frame {
            name: cb_id,
            width: {CHECKBOX_SIZE},
            height: {CHECKBOX_SIZE},
            background_color: CHECKBOX_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            fontstring {
                name: DynName(format!("GuildControlPerm{idx}CheckText")),
                width: {CHECKBOX_SIZE},
                height: {CHECKBOX_SIZE},
                text: check_text,
                font_size: 14.0,
                font_color: CHECKBOX_CHECK_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
        fontstring {
            name: label_id,
            width: {PERM_LABEL_W},
            height: {PERM_ROW_H},
            text: {perm.label.as_str()},
            font_size: 10.0,
            font_color: PERM_LABEL_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x + CHECKBOX_SIZE + CHECKBOX_GAP},
                y: {y},
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

    fn make_test_state() -> GuildControlState {
        GuildControlState {
            visible: true,
            ranks: vec![
                GuildRank {
                    name: "Guild Master".into(),
                    selected: true,
                },
                GuildRank {
                    name: "Officer".into(),
                    selected: false,
                },
                GuildRank {
                    name: "Member".into(),
                    selected: false,
                },
            ],
            rank_name: "Guild Master".into(),
            permissions: vec![
                PermissionRow {
                    label: "Invite Members".into(),
                    checked: true,
                },
                PermissionRow {
                    label: "Remove Members".into(),
                    checked: true,
                },
                PermissionRow {
                    label: "Promote Members".into(),
                    checked: true,
                },
                PermissionRow {
                    label: "Edit Public Note".into(),
                    checked: false,
                },
            ],
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(guild_control_screen).sync(&shared, &mut reg);
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

    #[test]
    fn builds_frame_and_title() {
        let reg = build_registry();
        assert!(reg.get_by_name("GuildControlFrame").is_some());
        assert!(reg.get_by_name("GuildControlTitle").is_some());
    }

    #[test]
    fn builds_rank_sidebar() {
        let reg = build_registry();
        assert!(reg.get_by_name("GuildControlRankSidebar").is_some());
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("GuildControlRank{i}")).is_some(),
                "GuildControlRank{i} missing"
            );
        }
    }

    #[test]
    fn builds_rank_name_editor() {
        let reg = build_registry();
        assert!(reg.get_by_name("GuildControlRankNameEditor").is_some());
        assert!(reg.get_by_name("GuildControlRankNameText").is_some());
    }

    #[test]
    fn builds_permission_checkboxes() {
        let reg = build_registry();
        for i in 0..4 {
            assert!(
                reg.get_by_name(&format!("GuildControlPerm{i}Check"))
                    .is_some(),
                "GuildControlPerm{i}Check missing"
            );
            assert!(
                reg.get_by_name(&format!("GuildControlPerm{i}Label"))
                    .is_some(),
                "GuildControlPerm{i}Label missing"
            );
        }
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(GuildControlState::default());
        Screen::new(guild_control_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("GuildControlFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_registry();
        let r = rect(&reg, "GuildControlFrame");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_sidebar() {
        let reg = layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let frame_y = (1080.0 - FRAME_H) / 2.0;
        let r = rect(&reg, "GuildControlRankSidebar");
        assert!((r.x - (frame_x + SIDEBAR_INSET)).abs() < 1.0);
        assert!((r.y - (frame_y + CONTENT_TOP)).abs() < 1.0);
        assert!((r.width - SIDEBAR_W).abs() < 1.0);
    }

    #[test]
    fn coord_first_checkbox() {
        let reg = layout_registry();
        let r = rect(&reg, "GuildControlPerm0Check");
        assert!((r.width - CHECKBOX_SIZE).abs() < 1.0);
        assert!((r.height - CHECKBOX_SIZE).abs() < 1.0);
    }
}
