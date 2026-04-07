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

// Bank tab permissions
const BANK_PERM_TOP_OFFSET: f32 = 12.0;
const BANK_TAB_ROW_H: f32 = 22.0;
const BANK_TAB_ROW_GAP: f32 = 2.0;
const BANK_TAB_LABEL_W: f32 = 80.0;
const BANK_TAB_CHECK_GAP: f32 = 20.0;
const BANK_LIMIT_W: f32 = 60.0;
const BANK_HEADER_COLOR: &str = "0.8,0.8,0.8,1.0";

pub const MAX_RANKS: usize = 10;
pub const MAX_PERMISSIONS: usize = 12;
pub const MAX_BANK_TAB_PERMS: usize = 8;

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
pub struct BankTabPermission {
    pub tab_name: String,
    pub can_view: bool,
    pub can_deposit: bool,
    pub can_withdraw: bool,
    pub withdraw_limit: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GuildControlState {
    pub visible: bool,
    pub ranks: Vec<GuildRank>,
    pub rank_name: String,
    pub permissions: Vec<PermissionRow>,
    pub bank_tab_permissions: Vec<BankTabPermission>,
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
            {bank_tab_permissions_panel(&state.bank_tab_permissions)}
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
    let (bg, color) = if rank.selected {
        (RANK_SELECTED_BG, RANK_SELECTED_COLOR)
    } else {
        (RANK_NORMAL_BG, RANK_NORMAL_COLOR)
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
            {rank_row_label(label_id, &rank.name, color)}
        }
    }
}

fn rank_row_label(id: DynName, text: &str, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {SIDEBAR_W - 8.0},
            height: {RANK_ROW_H},
            text: text,
            font_size: 10.0,
            font_color: color,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
        }
    }
}

fn rank_name_editor(name: &str) -> Element {
    let editor_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
    let editor_y = -CONTENT_TOP;
    let editor_w = FRAME_W - editor_x - SIDEBAR_INSET;
    rsx! {
        {editor_label("GuildControlRankNameLabel", "Rank Name:", editor_x, editor_y)}
        {editor_input("GuildControlRankNameEditor", "GuildControlRankNameText", name, editor_w - 84.0, editor_x + 84.0, editor_y)}
    }
}

fn editor_label(name: &str, text: &str, x: f32, y: f32) -> Element {
    rsx! {
        fontstring {
            name: DynName(name.into()),
            width: 80.0,
            height: {EDITOR_H},
            text: text,
            font_size: 10.0,
            font_color: EDITOR_LABEL_COLOR,
            justify_h: "RIGHT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {y} }
        }
    }
}

fn editor_input(name: &str, text_name: &str, value: &str, w: f32, x: f32, y: f32) -> Element {
    rsx! {
        r#frame {
            name: DynName(name.into()),
            width: {w},
            height: {EDITOR_H},
            background_color: EDITOR_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            fontstring {
                name: DynName(text_name.into()),
                width: {w - 8.0},
                height: {EDITOR_H},
                text: value,
                font_size: 10.0,
                font_color: EDITOR_TEXT_COLOR,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
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
    let check_text = if perm.checked { "\u{2713}" } else { "" };
    rsx! {
        {perm_checkbox(DynName(format!("GuildControlPerm{idx}Check")), DynName(format!("GuildControlPerm{idx}CheckText")), check_text, x, y)}
        {perm_label(DynName(format!("GuildControlPerm{idx}Label")), &perm.label, x + CHECKBOX_SIZE + CHECKBOX_GAP, y)}
    }
}

fn perm_checkbox(id: DynName, text_id: DynName, check: &str, x: f32, y: f32) -> Element {
    rsx! {
        r#frame {
            name: id,
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
                name: text_id,
                width: {CHECKBOX_SIZE},
                height: {CHECKBOX_SIZE},
                text: check,
                font_size: 14.0,
                font_color: CHECKBOX_CHECK_COLOR,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
    }
}

fn perm_label(id: DynName, text: &str, x: f32, y: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {PERM_LABEL_W},
            height: {PERM_ROW_H},
            text: text,
            font_size: 10.0,
            font_color: PERM_LABEL_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {y} }
        }
    }
}

// --- Bank tab permissions ---

fn bank_tab_permissions_panel(tabs: &[BankTabPermission]) -> Element {
    let panel_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
    let perm_grid_h = MAX_PERMISSIONS as f32 * (PERM_ROW_H + PERM_ROW_GAP);
    let panel_y = -(CONTENT_TOP + EDITOR_H + EDITOR_INSET + perm_grid_h + BANK_PERM_TOP_OFFSET);
    let header = bank_perm_header(panel_x, panel_y);
    let rows: Element = tabs
        .iter()
        .enumerate()
        .take(MAX_BANK_TAB_PERMS)
        .flat_map(|(i, tab)| {
            let y = panel_y - BANK_TAB_ROW_H - i as f32 * (BANK_TAB_ROW_H + BANK_TAB_ROW_GAP);
            bank_tab_perm_row(i, tab, panel_x, y)
        })
        .collect();
    rsx! {
        {header}
        {rows}
    }
}

fn bank_perm_header(x: f32, y: f32) -> Element {
    let col2 = x + BANK_TAB_LABEL_W;
    let col3 = col2 + BANK_TAB_CHECK_GAP + CHECKBOX_SIZE;
    let col4 = col3 + BANK_TAB_CHECK_GAP + CHECKBOX_SIZE;
    let col5 = col4 + BANK_TAB_CHECK_GAP + CHECKBOX_SIZE;
    rsx! {
        fontstring { name: "GuildControlBankPermHeaderTab", width: {BANK_TAB_LABEL_W}, height: {BANK_TAB_ROW_H}, text: "Bank Tab", font_size: 9.0, font_color: BANK_HEADER_COLOR, justify_h: "LEFT", anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {y} } }
        fontstring { name: "GuildControlBankPermHeaderView", width: 40.0, height: {BANK_TAB_ROW_H}, text: "View", font_size: 9.0, font_color: BANK_HEADER_COLOR, justify_h: "CENTER", anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {col2}, y: {y} } }
        fontstring { name: "GuildControlBankPermHeaderDeposit", width: 50.0, height: {BANK_TAB_ROW_H}, text: "Deposit", font_size: 9.0, font_color: BANK_HEADER_COLOR, justify_h: "CENTER", anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {col3}, y: {y} } }
        fontstring { name: "GuildControlBankPermHeaderWithdraw", width: 60.0, height: {BANK_TAB_ROW_H}, text: "Withdraw", font_size: 9.0, font_color: BANK_HEADER_COLOR, justify_h: "CENTER", anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {col4}, y: {y} } }
        fontstring { name: "GuildControlBankPermHeaderLimit", width: {BANK_LIMIT_W}, height: {BANK_TAB_ROW_H}, text: "Limit", font_size: 9.0, font_color: BANK_HEADER_COLOR, justify_h: "CENTER", anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {col5}, y: {y} } }
    }
}

fn bank_tab_perm_row(idx: usize, tab: &BankTabPermission, x: f32, y: f32) -> Element {
    let name_id = DynName(format!("GuildControlBankTab{idx}Name"));
    let view_id = DynName(format!("GuildControlBankTab{idx}View"));
    let dep_id = DynName(format!("GuildControlBankTab{idx}Deposit"));
    let wit_id = DynName(format!("GuildControlBankTab{idx}Withdraw"));
    let limit_id = DynName(format!("GuildControlBankTab{idx}Limit"));
    let col2 = x + BANK_TAB_LABEL_W;
    let col3 = col2 + BANK_TAB_CHECK_GAP + CHECKBOX_SIZE;
    let col4 = col3 + BANK_TAB_CHECK_GAP + CHECKBOX_SIZE;
    let col5 = col4 + BANK_TAB_CHECK_GAP + CHECKBOX_SIZE;
    let view_text = if tab.can_view { "\u{2713}" } else { "" };
    let dep_text = if tab.can_deposit { "\u{2713}" } else { "" };
    let wit_text = if tab.can_withdraw { "\u{2713}" } else { "" };
    rsx! {
        fontstring { name: name_id, width: {BANK_TAB_LABEL_W}, height: {BANK_TAB_ROW_H}, text: {tab.tab_name.as_str()}, font_size: 9.0, font_color: PERM_LABEL_COLOR, justify_h: "LEFT", anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: {y} } }
        r#frame { name: view_id, width: {CHECKBOX_SIZE}, height: {CHECKBOX_SIZE}, background_color: CHECKBOX_BG, anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {col2}, y: {y} } fontstring { name: DynName(format!("GuildControlBankTab{idx}ViewText")), width: {CHECKBOX_SIZE}, height: {CHECKBOX_SIZE}, text: view_text, font_size: 14.0, font_color: CHECKBOX_CHECK_COLOR, justify_h: "CENTER", anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft } } }
        r#frame { name: dep_id, width: {CHECKBOX_SIZE}, height: {CHECKBOX_SIZE}, background_color: CHECKBOX_BG, anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {col3}, y: {y} } fontstring { name: DynName(format!("GuildControlBankTab{idx}DepositText")), width: {CHECKBOX_SIZE}, height: {CHECKBOX_SIZE}, text: dep_text, font_size: 14.0, font_color: CHECKBOX_CHECK_COLOR, justify_h: "CENTER", anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft } } }
        r#frame { name: wit_id, width: {CHECKBOX_SIZE}, height: {CHECKBOX_SIZE}, background_color: CHECKBOX_BG, anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {col4}, y: {y} } fontstring { name: DynName(format!("GuildControlBankTab{idx}WithdrawText")), width: {CHECKBOX_SIZE}, height: {CHECKBOX_SIZE}, text: wit_text, font_size: 14.0, font_color: CHECKBOX_CHECK_COLOR, justify_h: "CENTER", anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft } } }
        fontstring { name: limit_id, width: {BANK_LIMIT_W}, height: {BANK_TAB_ROW_H}, text: {tab.withdraw_limit.as_str()}, font_size: 9.0, font_color: PERM_LABEL_COLOR, justify_h: "CENTER", anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {col5}, y: {y} } }
    }
}

#[cfg(test)]
#[path = "guild_control_component_tests.rs"]
mod tests;
