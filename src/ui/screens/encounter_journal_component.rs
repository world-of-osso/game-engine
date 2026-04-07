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

pub const FRAME_W: f32 = 750.0;
pub const FRAME_H: f32 = 500.0;
const HEADER_H: f32 = 28.0;
const SIDEBAR_W: f32 = 200.0;
const SIDEBAR_INSET: f32 = 8.0;
const TAB_H: f32 = 26.0;
const TAB_GAP: f32 = 2.0;
const INSTANCE_ROW_H: f32 = 24.0;
const INSTANCE_ROW_GAP: f32 = 1.0;
const CONTENT_GAP: f32 = 4.0;
const CONTENT_TOP: f32 = HEADER_H + TAB_GAP;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const SIDEBAR_BG: &str = "0.0,0.0,0.0,0.4";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const INSTANCE_SELECTED_BG: &str = "0.2,0.15,0.05,0.95";
const INSTANCE_NORMAL_BG: &str = "0.0,0.0,0.0,0.0";
const INSTANCE_SELECTED_COLOR: &str = "1.0,0.82,0.0,1.0";
const INSTANCE_NORMAL_COLOR: &str = "1.0,1.0,1.0,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";

// Boss list / detail layout
const BOSS_LIST_W: f32 = 160.0;
const BOSS_ROW_H: f32 = 24.0;
const BOSS_ROW_GAP: f32 = 1.0;
const BOSS_LIST_INSET: f32 = 4.0;
const DETAIL_INSET: f32 = 4.0;
const ABILITY_ICON_SIZE: f32 = 28.0;
const ABILITY_ROW_H: f32 = 48.0;
const ABILITY_ROW_GAP: f32 = 4.0;
const BOSS_NAME_H: f32 = 22.0;

const BOSS_SELECTED_BG: &str = "0.2,0.15,0.05,0.95";
const BOSS_NORMAL_BG: &str = "0.0,0.0,0.0,0.0";
const BOSS_SELECTED_COLOR: &str = "1.0,0.82,0.0,1.0";
const BOSS_NORMAL_COLOR: &str = "1.0,1.0,1.0,1.0";
const BOSS_NAME_COLOR: &str = "1.0,0.82,0.0,1.0";
const ABILITY_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const ABILITY_DESC_COLOR: &str = "0.8,0.8,0.8,1.0";
const ABILITY_ICON_BG: &str = "0.1,0.1,0.1,0.9";

// Loot tab layout
const LOOT_FILTER_H: f32 = 26.0;
const LOOT_FILTER_GAP: f32 = 8.0;
const LOOT_FILTER_W: f32 = 120.0;
const LOOT_HEADER_H: f32 = 20.0;
const LOOT_ROW_H: f32 = 26.0;
const LOOT_ROW_GAP: f32 = 1.0;
const LOOT_ICON_SIZE: f32 = 22.0;
const LOOT_INSET: f32 = 4.0;
const LOOT_FILTER_BG: &str = "0.08,0.07,0.06,0.88";
const LOOT_FILTER_COLOR: &str = "0.6,0.6,0.6,1.0";
const LOOT_HEADER_BG: &str = "0.12,0.1,0.08,0.9";
const LOOT_HEADER_COLOR: &str = "0.8,0.8,0.8,1.0";
const LOOT_ROW_EVEN: &str = "0.04,0.04,0.04,0.6";
const LOOT_ROW_ODD: &str = "0.06,0.06,0.06,0.6";
const LOOT_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const LOOT_SLOT_COLOR: &str = "0.7,0.7,0.7,1.0";
const LOOT_ICON_BG: &str = "0.1,0.1,0.1,0.9";

pub const MAX_INSTANCES: usize = 15;
pub const MAX_BOSSES: usize = 10;
pub const MAX_ABILITIES: usize = 8;
pub const MAX_LOOT_ITEMS: usize = 10;
pub const LOOT_COLUMNS: &[(&str, f32)] =
    &[("", 0.08), ("Item", 0.42), ("Slot", 0.25), ("Drop %", 0.25)];

#[derive(Clone, Debug, PartialEq)]
pub struct EJTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstanceEntry {
    pub name: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BossEntry {
    pub name: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BossAbility {
    pub name: String,
    pub description: String,
    pub icon_fdid: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LootItem {
    pub name: String,
    pub slot: String,
    pub drop_pct: String,
    pub icon_fdid: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncounterJournalState {
    pub visible: bool,
    pub tabs: Vec<EJTab>,
    pub instances: Vec<InstanceEntry>,
    pub bosses: Vec<BossEntry>,
    pub selected_boss_name: String,
    pub abilities: Vec<BossAbility>,
    pub loot_items: Vec<LootItem>,
    pub loot_slot_filter: String,
    pub loot_class_filter: String,
}

impl Default for EncounterJournalState {
    fn default() -> Self {
        Self {
            visible: false,
            tabs: vec![
                EJTab {
                    name: "Dungeons".into(),
                    active: true,
                },
                EJTab {
                    name: "Raids".into(),
                    active: false,
                },
                EJTab {
                    name: "Tier".into(),
                    active: false,
                },
            ],
            instances: vec![],
            bosses: vec![],
            selected_boss_name: String::new(),
            abilities: vec![],
            loot_items: vec![],
            loot_slot_filter: "All Slots".into(),
            loot_class_filter: "All Classes".into(),
        }
    }
}

pub fn encounter_journal_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<EncounterJournalState>()
        .expect("EncounterJournalState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "EncounterJournal",
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
            {sidebar_tabs(&state.tabs)}
            {instance_list(&state.instances)}
            {boss_content(&state.bosses, &state.selected_boss_name, &state.abilities)}
            {loot_tab(&state.loot_items, &state.loot_slot_filter, &state.loot_class_filter)}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "EncounterJournalTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Encounter Journal",
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

fn sidebar_tabs(tabs: &[EJTab]) -> Element {
    let tab_w = SIDEBAR_W;
    tabs.iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let y = -(HEADER_H + i as f32 * (TAB_H + TAB_GAP));
            sidebar_tab(i, tab, tab_w, y)
        })
        .collect()
}

fn sidebar_tab(i: usize, tab: &EJTab, tab_w: f32, y: f32) -> Element {
    let tab_id = DynName(format!("EJTab{i}"));
    let label_id = DynName(format!("EJTab{i}Label"));
    let (bg, color) = if tab.active {
        (TAB_BG_ACTIVE, TAB_TEXT_ACTIVE)
    } else {
        (TAB_BG_INACTIVE, TAB_TEXT_INACTIVE)
    };
    rsx! {
        r#frame {
            name: tab_id,
            width: {tab_w},
            height: {TAB_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SIDEBAR_INSET},
                y: {y},
            }
            {ej_tab_label(label_id, &tab.name, tab_w, color)}
        }
    }
}

fn ej_tab_label(id: DynName, text: &str, w: f32, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: {TAB_H},
            text: text,
            font_size: 11.0,
            font_color: color,
            justify_h: "CENTER",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn instance_list(instances: &[InstanceEntry]) -> Element {
    let tabs_h = 3.0 * (TAB_H + TAB_GAP);
    let list_y = -(HEADER_H + tabs_h + CONTENT_GAP);
    let list_h = FRAME_H - HEADER_H - tabs_h - CONTENT_GAP - SIDEBAR_INSET;
    let rows: Element = instances
        .iter()
        .enumerate()
        .take(MAX_INSTANCES)
        .flat_map(|(i, inst)| instance_row(i, inst))
        .collect();
    rsx! {
        r#frame {
            name: "EJInstanceList",
            width: {SIDEBAR_W},
            height: {list_h},
            background_color: SIDEBAR_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SIDEBAR_INSET},
                y: {list_y},
            }
            {rows}
        }
    }
}

fn instance_row(idx: usize, inst: &InstanceEntry) -> Element {
    let row_id = DynName(format!("EJInstance{idx}"));
    let label_id = DynName(format!("EJInstance{idx}Label"));
    let (bg, color) = if inst.selected {
        (INSTANCE_SELECTED_BG, INSTANCE_SELECTED_COLOR)
    } else {
        (INSTANCE_NORMAL_BG, INSTANCE_NORMAL_COLOR)
    };
    let y = -(idx as f32 * (INSTANCE_ROW_H + INSTANCE_ROW_GAP));
    rsx! {
        r#frame {
            name: row_id,
            width: {SIDEBAR_W},
            height: {INSTANCE_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
            {instance_row_label(label_id, &inst.name, color)}
        }
    }
}

fn instance_row_label(id: DynName, text: &str, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {SIDEBAR_W - 8.0},
            height: {INSTANCE_ROW_H},
            text: text,
            font_size: 10.0,
            font_color: color,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
        }
    }
}

fn boss_content(bosses: &[BossEntry], selected_name: &str, abilities: &[BossAbility]) -> Element {
    let content_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - content_x - SIDEBAR_INSET;
    let content_h = FRAME_H - CONTENT_TOP - SIDEBAR_INSET;
    rsx! {
        r#frame {
            name: "EJContentArea",
            width: {content_w},
            height: {content_h},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {content_x},
                y: {content_y},
            }
            {boss_list_panel(bosses, content_h)}
            {boss_detail_panel(selected_name, abilities, content_w, content_h)}
        }
    }
}

fn boss_list_panel(bosses: &[BossEntry], parent_h: f32) -> Element {
    let rows: Element = bosses
        .iter()
        .enumerate()
        .take(MAX_BOSSES)
        .flat_map(|(i, boss)| boss_row(i, boss))
        .collect();
    rsx! {
        r#frame {
            name: "EJBossList",
            width: {BOSS_LIST_W},
            height: {parent_h - 2.0 * BOSS_LIST_INSET},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {BOSS_LIST_INSET},
                y: {-BOSS_LIST_INSET},
            }
            {rows}
        }
    }
}

fn boss_row(idx: usize, boss: &BossEntry) -> Element {
    let row_id = DynName(format!("EJBoss{idx}"));
    let label_id = DynName(format!("EJBoss{idx}Label"));
    let (bg, color) = if boss.selected {
        (BOSS_SELECTED_BG, BOSS_SELECTED_COLOR)
    } else {
        (BOSS_NORMAL_BG, BOSS_NORMAL_COLOR)
    };
    let y = -(idx as f32 * (BOSS_ROW_H + BOSS_ROW_GAP));
    rsx! {
        r#frame {
            name: row_id,
            width: {BOSS_LIST_W},
            height: {BOSS_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
            {boss_row_label(label_id, &boss.name, color)}
        }
    }
}

fn boss_row_label(id: DynName, text: &str, color: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {BOSS_LIST_W - 8.0},
            height: {BOSS_ROW_H},
            text: text,
            font_size: 10.0,
            font_color: color,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
        }
    }
}

fn boss_detail_panel(
    boss_name: &str,
    abilities: &[BossAbility],
    parent_w: f32,
    parent_h: f32,
) -> Element {
    let detail_x = BOSS_LIST_INSET + BOSS_LIST_W + DETAIL_INSET;
    let detail_w = parent_w - detail_x - DETAIL_INSET;
    let detail_h = parent_h - 2.0 * DETAIL_INSET;
    let ability_rows = build_ability_rows(abilities, detail_w);
    rsx! {
        r#frame {
            name: "EJBossDetail",
            width: {detail_w},
            height: {detail_h},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {detail_x},
                y: {-DETAIL_INSET},
            }
            {boss_detail_name(boss_name, detail_w)}
            {ability_rows}
        }
    }
}

fn boss_detail_name(name: &str, w: f32) -> Element {
    rsx! {
        fontstring {
            name: "EJBossDetailName",
            width: {w},
            height: {BOSS_NAME_H},
            text: name,
            font_size: 14.0,
            font_color: BOSS_NAME_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn build_ability_rows(abilities: &[BossAbility], w: f32) -> Element {
    abilities
        .iter()
        .enumerate()
        .take(MAX_ABILITIES)
        .flat_map(|(i, ability)| ability_row(i, ability, w))
        .collect()
}

fn ability_row(idx: usize, ability: &BossAbility, parent_w: f32) -> Element {
    let row_id = DynName(format!("EJAbility{idx}"));
    let icon_id = DynName(format!("EJAbility{idx}Icon"));
    let y = -(BOSS_NAME_H + idx as f32 * (ABILITY_ROW_H + ABILITY_ROW_GAP));
    let text_x = ABILITY_ICON_SIZE + 8.0;
    let text_w = parent_w - text_x;
    rsx! {
        r#frame {
            name: row_id,
            width: {parent_w},
            height: {ABILITY_ROW_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
            {ability_icon(icon_id)}
            {ability_name_label(DynName(format!("EJAbility{idx}Name")), &ability.name, text_w, text_x)}
            {ability_desc_label(DynName(format!("EJAbility{idx}Desc")), &ability.description, text_w, text_x)}
        }
    }
}

fn ability_icon(id: DynName) -> Element {
    rsx! {
        r#frame {
            name: id,
            width: {ABILITY_ICON_SIZE},
            height: {ABILITY_ICON_SIZE},
            background_color: ABILITY_ICON_BG,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
        }
    }
}

fn ability_name_label(id: DynName, text: &str, w: f32, x: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: 16.0,
            text: text,
            font_size: 11.0,
            font_color: ABILITY_NAME_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: "0" }
        }
    }
}

fn ability_desc_label(id: DynName, text: &str, w: f32, x: f32) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: 28.0,
            text: text,
            font_size: 9.0,
            font_color: ABILITY_DESC_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {x}, y: "-16" }
        }
    }
}

// --- Loot tab ---

fn loot_tab(items: &[LootItem], slot_filter: &str, class_filter: &str) -> Element {
    let content_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - content_x - SIDEBAR_INSET;
    let content_h = FRAME_H - CONTENT_TOP - SIDEBAR_INSET;
    rsx! {
        r#frame {
            name: "EJLootTab",
            width: {content_w},
            height: {content_h},
            hidden: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {content_x},
                y: {content_y},
            }
            {loot_filter_row(slot_filter, class_filter)}
            {loot_header(content_w)}
            {loot_rows(items, content_w)}
        }
    }
}

fn loot_filter_row(slot_filter: &str, class_filter: &str) -> Element {
    let class_x = LOOT_INSET + LOOT_FILTER_W + LOOT_FILTER_GAP;
    rsx! {
        {loot_filter_dropdown("EJLootSlotFilter", slot_filter, LOOT_INSET)}
        {loot_filter_dropdown("EJLootClassFilter", class_filter, class_x)}
    }
}

fn loot_filter_dropdown(name: &str, text: &str, x: f32) -> Element {
    let frame_id = DynName(name.into());
    let text_id = DynName(format!("{name}Text"));
    rsx! {
        r#frame {
            name: frame_id,
            width: {LOOT_FILTER_W},
            height: {LOOT_FILTER_H},
            background_color: LOOT_FILTER_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-LOOT_INSET},
            }
            fontstring {
                name: text_id,
                width: {LOOT_FILTER_W - 8.0},
                height: {LOOT_FILTER_H},
                text: text,
                font_size: 10.0,
                font_color: LOOT_FILTER_COLOR,
                justify_h: "LEFT",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "4", y: "0" }
            }
        }
    }
}

fn loot_header(parent_w: f32) -> Element {
    let header_y = -(LOOT_INSET + LOOT_FILTER_H + LOOT_INSET);
    let header_w = parent_w - 2.0 * LOOT_INSET;
    let cols: Element = LOOT_COLUMNS
        .iter()
        .enumerate()
        .flat_map(|(i, (name, _))| {
            let x = loot_col_x(header_w, i);
            let w = loot_col_w(header_w, i);
            loot_header_cell(i, name, x, w)
        })
        .collect();
    rsx! {
        r#frame {
            name: "EJLootHeader",
            width: {header_w},
            height: {LOOT_HEADER_H},
            background_color: LOOT_HEADER_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {LOOT_INSET},
                y: {header_y},
            }
            {cols}
        }
    }
}

fn loot_header_cell(idx: usize, text: &str, x: f32, w: f32) -> Element {
    let cell_id = DynName(format!("EJLootCol{idx}"));
    rsx! {
        fontstring {
            name: cell_id,
            width: {w},
            height: {LOOT_HEADER_H},
            text,
            font_size: 9.0,
            font_color: LOOT_HEADER_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: "0",
            }
        }
    }
}

fn loot_rows(items: &[LootItem], parent_w: f32) -> Element {
    let row_w = parent_w - 2.0 * LOOT_INSET;
    let top = LOOT_INSET + LOOT_FILTER_H + LOOT_INSET + LOOT_HEADER_H;
    items
        .iter()
        .enumerate()
        .take(MAX_LOOT_ITEMS)
        .flat_map(|(i, item)| loot_row(i, item, row_w, top))
        .collect()
}

fn loot_row(idx: usize, item: &LootItem, row_w: f32, top: f32) -> Element {
    let row_id = DynName(format!("EJLoot{idx}"));
    let icon_id = DynName(format!("EJLoot{idx}Icon"));
    let name_id = DynName(format!("EJLoot{idx}Name"));
    let slot_id = DynName(format!("EJLoot{idx}Slot"));
    let drop_id = DynName(format!("EJLoot{idx}Drop"));
    let y = -(top + idx as f32 * (LOOT_ROW_H + LOOT_ROW_GAP));
    let bg = if idx % 2 == 0 {
        LOOT_ROW_EVEN
    } else {
        LOOT_ROW_ODD
    };
    let icon_col_w = loot_col_w(row_w, 0);
    let name_x = loot_col_x(row_w, 1);
    let name_w = loot_col_w(row_w, 1);
    let slot_x = loot_col_x(row_w, 2);
    let slot_w = loot_col_w(row_w, 2);
    let drop_x = loot_col_x(row_w, 3);
    let drop_w = loot_col_w(row_w, 3);
    rsx! {
        r#frame {
            name: row_id,
            width: {row_w},
            height: {LOOT_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {LOOT_INSET},
                y: {y},
            }
            r#frame {
                name: icon_id,
                width: {LOOT_ICON_SIZE},
                height: {LOOT_ICON_SIZE},
                background_color: LOOT_ICON_BG,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "2",
                    y: "-2",
                }
            }
            fontstring {
                name: name_id,
                width: {name_w},
                height: {LOOT_ROW_H},
                text: {item.name.as_str()},
                font_size: 10.0,
                font_color: LOOT_NAME_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {name_x},
                    y: "0",
                }
            }
            fontstring {
                name: slot_id,
                width: {slot_w},
                height: {LOOT_ROW_H},
                text: {item.slot.as_str()},
                font_size: 9.0,
                font_color: LOOT_SLOT_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {slot_x},
                    y: "0",
                }
            }
            fontstring {
                name: drop_id,
                width: {drop_w},
                height: {LOOT_ROW_H},
                text: {item.drop_pct.as_str()},
                font_size: 9.0,
                font_color: LOOT_SLOT_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {drop_x},
                    y: "0",
                }
            }
        }
    }
}

fn loot_col_x(row_w: f32, col: usize) -> f32 {
    let mut x = 0.0;
    for i in 0..col {
        x += LOOT_COLUMNS[i].1 * row_w;
    }
    x
}

fn loot_col_w(row_w: f32, col: usize) -> f32 {
    LOOT_COLUMNS[col].1 * row_w
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state() -> EncounterJournalState {
        EncounterJournalState {
            visible: true,
            instances: vec![
                InstanceEntry {
                    name: "Deadmines".into(),
                    selected: true,
                },
                InstanceEntry {
                    name: "Shadowfang Keep".into(),
                    selected: false,
                },
                InstanceEntry {
                    name: "Blackfathom Deeps".into(),
                    selected: false,
                },
            ],
            ..Default::default()
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("EncounterJournal").is_some());
        assert!(reg.get_by_name("EncounterJournalTitle").is_some());
    }

    #[test]
    fn builds_three_tabs() {
        let reg = build_registry();
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("EJTab{i}")).is_some(),
                "EJTab{i} missing"
            );
        }
    }

    #[test]
    fn builds_instance_list() {
        let reg = build_registry();
        assert!(reg.get_by_name("EJInstanceList").is_some());
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("EJInstance{i}")).is_some(),
                "EJInstance{i} missing"
            );
        }
    }

    #[test]
    fn builds_content_area() {
        let reg = build_registry();
        assert!(reg.get_by_name("EJContentArea").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(EncounterJournalState::default());
        Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("EncounterJournal").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_registry();
        let r = rect(&reg, "EncounterJournal");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_first_tab() {
        let reg = layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let frame_y = (1080.0 - FRAME_H) / 2.0;
        let r = rect(&reg, "EJTab0");
        assert!((r.x - (frame_x + SIDEBAR_INSET)).abs() < 1.0);
        assert!((r.y - (frame_y + HEADER_H)).abs() < 1.0);
        assert!((r.width - SIDEBAR_W).abs() < 1.0);
    }

    #[test]
    fn coord_content_area() {
        let reg = layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let frame_y = (1080.0 - FRAME_H) / 2.0;
        let r = rect(&reg, "EJContentArea");
        let expected_x = frame_x + SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - (frame_y + CONTENT_TOP)).abs() < 1.0);
    }

    // --- Boss list / detail tests ---

    fn make_boss_state() -> EncounterJournalState {
        let mut state = make_test_state();
        state.bosses = vec![
            BossEntry {
                name: "Edwin VanCleef".into(),
                selected: true,
            },
            BossEntry {
                name: "Cookie".into(),
                selected: false,
            },
        ];
        state.selected_boss_name = "Edwin VanCleef".into();
        state.abilities = vec![
            BossAbility {
                name: "Deadly Poison".into(),
                description: "Coats weapons with poison.".into(),
                icon_fdid: 12345,
            },
            BossAbility {
                name: "Summon Pirates".into(),
                description: "Calls nearby pirates to aid.".into(),
                icon_fdid: 12346,
            },
        ];
        state
    }

    fn boss_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_boss_state());
        Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn boss_list_builds_entries() {
        let reg = boss_registry();
        assert!(reg.get_by_name("EJBossList").is_some());
        assert!(reg.get_by_name("EJBoss0").is_some());
        assert!(reg.get_by_name("EJBoss1").is_some());
        assert!(reg.get_by_name("EJBoss0Label").is_some());
    }

    #[test]
    fn boss_detail_builds_name_and_abilities() {
        let reg = boss_registry();
        assert!(reg.get_by_name("EJBossDetail").is_some());
        assert!(reg.get_by_name("EJBossDetailName").is_some());
        for i in 0..2 {
            assert!(
                reg.get_by_name(&format!("EJAbility{i}")).is_some(),
                "EJAbility{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("EJAbility{i}Icon")).is_some(),
                "EJAbility{i}Icon missing"
            );
            assert!(
                reg.get_by_name(&format!("EJAbility{i}Name")).is_some(),
                "EJAbility{i}Name missing"
            );
            assert!(
                reg.get_by_name(&format!("EJAbility{i}Desc")).is_some(),
                "EJAbility{i}Desc missing"
            );
        }
    }

    // --- Loot tab tests ---

    fn make_loot_state() -> EncounterJournalState {
        let mut state = make_test_state();
        state.loot_items = vec![
            LootItem {
                name: "Cruel Barb".into(),
                slot: "One-Hand Sword".into(),
                drop_pct: "15%".into(),
                icon_fdid: 11111,
            },
            LootItem {
                name: "Cape of the Brotherhood".into(),
                slot: "Back".into(),
                drop_pct: "18%".into(),
                icon_fdid: 22222,
            },
        ];
        state
    }

    fn loot_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_loot_state());
        Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn loot_tab_builds_filters() {
        let reg = loot_registry();
        assert!(reg.get_by_name("EJLootTab").is_some());
        assert!(reg.get_by_name("EJLootSlotFilter").is_some());
        assert!(reg.get_by_name("EJLootClassFilter").is_some());
    }

    #[test]
    fn loot_tab_builds_header() {
        let reg = loot_registry();
        assert!(reg.get_by_name("EJLootHeader").is_some());
        for i in 0..LOOT_COLUMNS.len() {
            assert!(
                reg.get_by_name(&format!("EJLootCol{i}")).is_some(),
                "EJLootCol{i} missing"
            );
        }
    }

    #[test]
    fn loot_tab_builds_item_rows() {
        let reg = loot_registry();
        for i in 0..2 {
            assert!(
                reg.get_by_name(&format!("EJLoot{i}")).is_some(),
                "EJLoot{i} missing"
            );
            assert!(
                reg.get_by_name(&format!("EJLoot{i}Icon")).is_some(),
                "EJLoot{i}Icon missing"
            );
            assert!(
                reg.get_by_name(&format!("EJLoot{i}Name")).is_some(),
                "EJLoot{i}Name missing"
            );
            assert!(
                reg.get_by_name(&format!("EJLoot{i}Slot")).is_some(),
                "EJLoot{i}Slot missing"
            );
            assert!(
                reg.get_by_name(&format!("EJLoot{i}Drop")).is_some(),
                "EJLoot{i}Drop missing"
            );
        }
    }

    // --- Additional coord validation ---

    fn boss_layout_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_boss_state());
        Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);
        reg
    }

    #[test]
    fn coord_boss_list_position() {
        let reg = boss_layout_registry();
        let content = rect(&reg, "EJContentArea");
        let bl = rect(&reg, "EJBossList");
        assert!((bl.x - (content.x + BOSS_LIST_INSET)).abs() < 1.0);
        assert!((bl.y - (content.y + BOSS_LIST_INSET)).abs() < 1.0);
        assert!((bl.width - BOSS_LIST_W).abs() < 1.0);
    }

    #[test]
    fn coord_boss_detail_right_of_list() {
        let reg = boss_layout_registry();
        let content = rect(&reg, "EJContentArea");
        let detail = rect(&reg, "EJBossDetail");
        let expected_x = content.x + BOSS_LIST_INSET + BOSS_LIST_W + DETAIL_INSET;
        assert!(
            (detail.x - expected_x).abs() < 1.0,
            "x: expected {expected_x}, got {}",
            detail.x
        );
    }

    #[test]
    fn coord_loot_tab_filters() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_loot_state());
        Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);

        let slot_f = rect(&reg, "EJLootSlotFilter");
        let class_f = rect(&reg, "EJLootClassFilter");
        assert!((slot_f.width - LOOT_FILTER_W).abs() < 1.0);
        assert!((class_f.width - LOOT_FILTER_W).abs() < 1.0);
        let spacing = class_f.x - slot_f.x;
        let expected = LOOT_FILTER_W + LOOT_FILTER_GAP;
        assert!(
            (spacing - expected).abs() < 1.0,
            "filter spacing: expected {expected}, got {spacing}"
        );
    }
}
