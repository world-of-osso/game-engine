//! Retail RaidInfoFrameSocialView.lua/XML: saved rows, selection and extension.
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::{DynName, flat_panel_chrome, panel_button};
use crate::ui::strata::FrameStrata;
use shared::protocol::{InstanceLockInfo, RaidDifficultySet, SetSavedInstanceExtend};
use std::collections::HashMap;
use std::path::Path;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const FRAME: &str = "RaidInfoFrame";
pub const SCROLL_BOX: &str = "RaidInfoScrollBox";
const LIST_HEIGHT: f32 = 157.0;
pub const ROW_HEIGHT: f32 = 40.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedInstanceRow {
    pub lock: InstanceLockInfo,
    pub name: String,
    pub difficulty: String,
    pub extend_disabled: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RaidInfoState {
    pub visible: bool,
    pub rows: Vec<SavedInstanceRow>,
    pub selected: Option<(u32, u32)>,
}
impl RaidInfoState {
    pub fn selected_row(&self) -> Option<&SavedInstanceRow> {
        self.rows
            .iter()
            .find(|row| self.selected == Some((row.lock.map_id, row.lock.difficulty_id)))
    }
    pub fn extend_label(&self) -> &'static str {
        match self.selected_row() {
            Some(row) if row.lock.extended => "Cancel Raid Lock Extension",
            Some(row) if !row.lock.locked => "Reactivate Raid Lock",
            _ => "Extend Raid Lock",
        }
    }
    pub fn action(&mut self, action: &str) -> Option<SetSavedInstanceExtend> {
        match action {
            "raid_info:close" => self.visible = false,
            "raid_info:extend" => {
                let row = self.selected_row()?;
                if row.extend_disabled {
                    return None;
                }
                return Some(SetSavedInstanceExtend {
                    map_id: row.lock.map_id,
                    difficulty_id: row.lock.difficulty_id,
                    extend: !row.lock.extended,
                });
            }
            _ => {
                let index = action
                    .strip_prefix("raid_info:select:")?
                    .parse::<usize>()
                    .ok()?;
                let row = self.rows.get(index)?;
                self.selected = Some((row.lock.map_id, row.lock.difficulty_id));
            }
        }
        None
    }
}

/// Refresh once a cached active reset timer elapses, including extended lock snapshots.
pub fn snapshot_reached_expiry(locks: &[InstanceLockInfo], elapsed: u32) -> bool {
    locks
        .iter()
        .any(|lock| lock.time_remaining_secs > 0 && elapsed >= lock.time_remaining_secs)
}

#[derive(Default)]
pub struct RaidCatalog {
    names: HashMap<u32, String>,
    instance_types: HashMap<u32, u32>,
    difficulties: HashMap<u32, String>,
    extend_disabled: HashMap<(u32, u32), bool>,
}
impl RaidCatalog {
    pub fn load(db2: &Path) -> Result<Self, String> {
        let mut result = Self::default();
        for row in crate::dungeon_entrance_data::read_rows(
            &db2.join("Map.csv"),
            &["ID", "MapName_lang", "InstanceType"],
        )? {
            result.names.insert(number(&row[0])?, row[1].clone());
            result
                .instance_types
                .insert(number(&row[0])?, number(&row[2])?);
        }
        for row in crate::dungeon_entrance_data::read_rows(
            &db2.join("Difficulty.csv"),
            &["ID", "Name_lang"],
        )? {
            result.difficulties.insert(number(&row[0])?, row[1].clone());
        }
        for row in crate::dungeon_entrance_data::read_rows(
            &db2.join("MapDifficulty.csv"),
            &["MapID", "DifficultyID", "Flags"],
        )? {
            result.extend_disabled.insert(
                (number(&row[0])?, number(&row[1])?),
                number(&row[2])? & 0x10 != 0,
            );
        }
        Ok(result)
    }
    pub fn is_instance(&self, map_id: u32) -> bool {
        self.instance_types
            .get(&map_id)
            .is_some_and(|kind| *kind != 0)
    }

    pub fn rows(
        &self,
        locks: &[InstanceLockInfo],
        elapsed: u32,
    ) -> Result<Vec<SavedInstanceRow>, String> {
        locks
            .iter()
            .map(|lock| {
                let mut lock = *lock;
                lock.time_remaining_secs = lock.time_remaining_secs.saturating_sub(elapsed);
                lock.locked = lock.locked && lock.time_remaining_secs > 0;
                Ok(SavedInstanceRow {
                    name: self
                        .names
                        .get(&lock.map_id)
                        .ok_or_else(|| format!("Unknown saved map {}", lock.map_id))?
                        .clone(),
                    difficulty: self
                        .difficulties
                        .get(&lock.difficulty_id)
                        .ok_or_else(|| format!("Unknown saved difficulty {}", lock.difficulty_id))?
                        .clone(),
                    extend_disabled: *self
                        .extend_disabled
                        .get(&(lock.map_id, lock.difficulty_id))
                        .ok_or_else(|| {
                            format!(
                                "No MapDifficulty for saved map {} difficulty {}",
                                lock.map_id, lock.difficulty_id
                            )
                        })?,
                    lock,
                })
            })
            .collect()
    }
}
fn number(value: &str) -> Result<u32, String> {
    value
        .parse()
        .map_err(|e| format!("Invalid DB2 integer {value}: {e}"))
}

/// Retail UnitPopup.lua SetRaidDifficulties maps primary Normal/Heroic to legacy size.
pub fn raid_selection_requests(
    selection: RaidDifficultySet,
    id: u32,
    legacy: bool,
) -> Vec<shared::protocol::SetRaidDifficulty> {
    use shared::protocol::SetRaidDifficulty;
    let size = if legacy {
        id
    } else {
        match selection.legacy_difficulty_id {
            3 | 5 => 3,
            _ => 4,
        }
    };
    if legacy {
        if selection.difficulty_id == 16 {
            return Vec::new();
        }
        return vec![SetRaidDifficulty {
            difficulty_id: if selection.difficulty_id == 15 {
                size + 2
            } else {
                size
            },
            legacy: true,
        }];
    }
    let mut requests = vec![SetRaidDifficulty {
        difficulty_id: id,
        legacy: false,
    }];
    if id != 16 {
        requests.push(SetRaidDifficulty {
            difficulty_id: if id == 15 { size + 2 } else { size },
            legacy: true,
        });
    }
    requests
}

pub fn screen(ctx: &SharedContext) -> Element {
    let state = ctx.get::<RaidInfoState>().expect("Raid Info state");
    let hidden = !state.visible;
    let mut content = flat_panel_chrome(FRAME, (345.0, 250.0), "", "raid_info:close");
    content.extend(label(
        "RaidInfoInstance",
        "Instance",
        (18.0, 12.0, 150.0),
        ("GameFontHighlight", "1,1,1,1"),
    ));
    content.extend(label(
        "RaidInfoExpires",
        "Lock Expires",
        (184.0, 12.0, 122.0),
        ("GameFontHighlight", "1,1,1,1"),
    ));
    content.extend(saved_instance_list(ctx, state));
    let enabled = state.selected_row().is_some_and(|row| !row.extend_disabled);
    content.extend(panel_button(
        "RaidInfoExtendButton".into(),
        state.extend_label(),
        "raid_info:extend",
        enabled,
        (50.0, 210.0, 245.0, 30.0),
    ));
    crate::panel_lifecycle::bind_close(
        rsx! { r#frame { name: "RaidInfoFrame", width:345.0,height:250.0,hidden:hidden,strata:FrameStrata::Dialog,
        mouse_enabled:true,pos_type:"absolute",left:360.0,top:140.0,{content} } },
        "raid_info",
    )
}
fn saved_instance_list(ctx: &SharedContext, state: &RaidInfoState) -> Element {
    // RaidInfoFrameSocialView.xml: ScrollBox292x157 TOPLEFT(16,-48), bar RIGHT+10, top-2/bottom+4.
    let height = state.rows.len() as f32 * ROW_HEIGHT;
    let geometry = pixel_geometry(LIST_HEIGHT, height, LIST_HEIGHT - 6.0);
    let offset = geometry.clamp(ctx.scroll_first_row(SCROLL_BOX));
    let rows: Element = state
        .rows
        .iter()
        .enumerate()
        .flat_map(|(index, row)| saved_row(index, index, row, state.selected))
        .collect();
    let config = scroll_list_attr(&geometry);
    let scrollbar = MinimalScrollBar {
        list: SCROLL_BOX,
        left: 16.0 + 302.0,
        top: 48.0 + 2.0,
        height: 151.0,
        geometry,
        offset,
        unscrollable: Unscrollable::HideThumb,
    }
    .element();
    let child_top = -(offset as f32);
    rsx! { r#frame { name: {DynName(SCROLL_BOX.into())}, width: 292.0, height: LIST_HEIGHT, left: 16.0, top: 48.0, pos_type: "absolute", mouse_enabled: true, scroll_list: config,
        r#frame { name: "RaidInfoScrollChild", width: 292.0, height, left: 0.0, top: child_top, pos_type: "absolute", {rows} }
    }
    {scrollbar}
    }
}

fn saved_row(
    slot: usize,
    index: usize,
    row: &SavedInstanceRow,
    selected: Option<(u32, u32)>,
) -> Element {
    let y = slot as f32 * ROW_HEIGHT;
    let action = format!("raid_info:select:{index}");
    let name = format!("RaidInfoRow{index}");
    let selected = selected == Some((row.lock.map_id, row.lock.difficulty_id));
    let color = if row.lock.locked || row.lock.extended {
        "1,0.82,0,1"
    } else {
        "0.5,0.5,0.5,1"
    };
    let remaining = if row.lock.locked || row.lock.extended {
        format_reset(row.lock.time_remaining_secs)
    } else {
        "Expired".into()
    };
    let mut texts = label(
        &format!("{name}Name"),
        &row.name,
        (5.0, 4.0, 150.0),
        ("GameFontNormal", color),
    );
    texts.extend(label(
        &format!("{name}Difficulty"),
        &row.difficulty,
        (5.0, 21.0, 150.0),
        ("GameFontHighlightSmall", "1,1,1,1"),
    ));
    texts.extend(label(
        &format!("{name}Reset"),
        &remaining,
        (161.0, 4.0, 126.0),
        ("GameFontHighlightSmall", "1,1,1,1"),
    ));
    if row.lock.extended {
        texts.extend(label(
            &format!("{name}Extended"),
            "Extended",
            (201.0, 21.0, 86.0),
            ("GameFontHighlightSmall", "1,1,1,1"),
        ));
    }
    let template = (&crate::selection_list::RAID_INSTANCE, selected);
    crate::selection_list::row(name, &action, true, (0.0, y, 292.0, 40.0), template, texts)
}
fn label(
    name: &str,
    text: &str,
    (x, y, width): (f32, f32, f32),
    (font_object, color): (&str, &str),
) -> Element {
    rsx! { fontstring {name:{DynName(name.into())},text:text,width:width,height:18.0,font_object,font_color:color,
    justify_h:"LEFT",pos_type:"absolute",left:x,top:y} }
}
pub fn format_reset(seconds: u32) -> String {
    let days = seconds / 86400;
    let hours = seconds % 86400 / 3600;
    let minutes = seconds % 3600 / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}
