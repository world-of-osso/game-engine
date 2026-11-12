//! Retail Mainline PVEFrame / LFDQueueFrame follower mode.
//! Source: Blizzard_GroupFinder/Mainline/{PVEFrame,LFDFrame,LFGFrame}.xml/.lua.
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::csv_records::CsvTable;
pub use crate::follower_dungeons_component::{FOLLOWER_LIST, ROW_HEIGHT};
use shared::protocol::{FollowerDungeons, GroupRoleSnapshot, LfgStateUpdate, QueueForLfg};
use ui_toolkit::{screen::SharedContext, widget_def::Element};

/// PVEFrame.lua GroupFinderFrameButton_SetEnabled: disabled category icon/ring gray.
pub fn apply_follower_postsetup(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::frame::WidgetData;
    apply_follower_background_layers(registry);
    apply_follower_tab_layers(registry);
    for index in 2..=3 {
        for part in ["Icon", "Ring"] {
            let name = format!("GroupFinderFrameGroupButton{index}{part}");
            if let Some(id) = registry.get_by_name(&name)
                && let Some(frame) = registry.get_mut(id)
                && let Some(WidgetData::Texture(texture)) = &mut frame.widget_data
            {
                texture.desaturated = true;
            }
        }
    }
}
/// PanelTabButtonTemplate places caps in BACKGROUND, below every overlapping tab label.
fn apply_follower_tab_layers(registry: &mut ui_toolkit::registry::FrameRegistry) {
    for index in 1..=3 {
        for part in ["Left", "Middle", "Right"] {
            let name = format!("PVEFrameTab{index}{part}");
            if let Some(id) = registry.get_by_name(&name)
                && let Some(frame) = registry.get_mut(id)
            {
                frame.draw_layer = ui_toolkit::strata::DrawLayer::Background;
            }
        }
    }
}
/// XML useParentLevel prevents nested native-size background textures covering queue labels.
fn apply_follower_background_layers(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::strata::DrawLayer;
    let Some(parent) = registry
        .get_by_name("LFDParentFrame")
        .and_then(|id| registry.get(id))
    else {
        return;
    };
    let level = parent.frame_level;
    let layers = [
        ("LFDParentFrameInset", DrawLayer::Background),
        ("LFDParentFrameInsetBg", DrawLayer::Background),
        ("LFDQueueFrameBackground", DrawLayer::Border),
        ("LFDParentFrameRoleBackground", DrawLayer::Background),
    ];
    for (name, layer) in layers {
        if let Some(id) = registry.get_by_name(name)
            && let Some(frame) = registry.get_mut(id)
        {
            frame.frame_level = level;
            frame.draw_layer = layer;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Queue(QueueForLfg),
    Pvp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonLevelRange {
    pub min: u32,
    pub max: u32,
}

static RETAIL_LEVEL_RANGES: OnceLock<Result<HashMap<u32, DungeonLevelRange>, String>> =
    OnceLock::new();

/// Retail follower-dungeon ContentTuning ranges from installed LFGDungeons and
/// ContentTuning DB2 tables.
pub fn retail_level_ranges() -> Result<HashMap<u32, DungeonLevelRange>, String> {
    RETAIL_LEVEL_RANGES
        .get_or_init(load_retail_level_ranges)
        .clone()
}

fn load_retail_level_ranges() -> Result<HashMap<u32, DungeonLevelRange>, String> {
    let lfg_path = crate::paths::resolve_data_path("db2/active/LFGDungeons.csv");
    let tuning_path = crate::paths::resolve_data_path("db2/active/ContentTuning.csv");
    let lfg = CsvTable::read(&lfg_path)?;
    let tuning = CsvTable::read(&tuning_path)?;
    let ranges_by_tuning = parse_content_tuning_ranges(&tuning)?;
    parse_lfg_level_ranges(&lfg, &ranges_by_tuning)
}

fn parse_content_tuning_ranges(
    table: &CsvTable,
) -> Result<HashMap<u32, DungeonLevelRange>, String> {
    let id_column = table.column("ID")?;
    let lfg_min_column = table.column("LfgMinLevel")?;
    let lfg_max_column = table.column("LfgMaxLevel")?;
    let squish_min_column = table.column("MinLevelSquish")?;
    let squish_max_column = table.column("MaxLevelSquish")?;
    let mut ranges = HashMap::new();
    for row in table.records() {
        let id = parse_db2_u32(&row, id_column, table, "ID")?;
        let lfg_min = parse_db2_i32(&row, lfg_min_column, table, "LfgMinLevel")?;
        let lfg_max = parse_db2_i32(&row, lfg_max_column, table, "LfgMaxLevel")?;
        let squish_min = parse_db2_i32(&row, squish_min_column, table, "MinLevelSquish")?;
        let squish_max = parse_db2_i32(&row, squish_max_column, table, "MaxLevelSquish")?;
        if let Some(range) = select_level_range(lfg_min, lfg_max, squish_min, squish_max) {
            ranges.insert(id, range);
        }
    }
    Ok(ranges)
}

fn parse_lfg_level_ranges(
    table: &CsvTable,
    ranges_by_tuning: &HashMap<u32, DungeonLevelRange>,
) -> Result<HashMap<u32, DungeonLevelRange>, String> {
    let id_column = table.column("ID")?;
    let tuning_column = table.column("ContentTuningID")?;
    let mut ranges = HashMap::new();
    for row in table.records() {
        let id = parse_db2_u32(&row, id_column, table, "ID")?;
        let content_tuning_id = parse_db2_u32(&row, tuning_column, table, "ContentTuningID")?;
        if let Some(range) = ranges_by_tuning.get(&content_tuning_id) {
            ranges.insert(id, *range);
        }
    }
    Ok(ranges)
}

fn parse_db2_u32(
    row: &[std::borrow::Cow<'_, str>],
    column: usize,
    table: &CsvTable,
    name: &str,
) -> Result<u32, String> {
    row.get(column)
        .ok_or_else(|| format!("{} row is missing {name}", table.path().display()))?
        .parse::<u32>()
        .map_err(|error| format!("{} has invalid {name}: {error}", table.path().display()))
}

fn parse_db2_i32(
    row: &[std::borrow::Cow<'_, str>],
    column: usize,
    table: &CsvTable,
    name: &str,
) -> Result<i32, String> {
    row.get(column)
        .ok_or_else(|| format!("{} row is missing {name}", table.path().display()))?
        .parse::<i32>()
        .map_err(|error| format!("{} has invalid {name}: {error}", table.path().display()))
}

fn select_level_range(
    lfg_min: i32,
    lfg_max: i32,
    squish_min: i32,
    squish_max: i32,
) -> Option<DungeonLevelRange> {
    let (min, max) = if lfg_min > 0 && lfg_max >= lfg_min {
        (lfg_min, lfg_max)
    } else {
        (squish_min, squish_max)
    };
    if min <= 0 || max < min {
        return None;
    }
    Some(DungeonLevelRange {
        min: u32::try_from(min).ok()?,
        max: u32::try_from(max).ok()?,
    })
}

pub fn localize_error(error: &str) -> String {
    let tag = match error {
        "FOLLOWER_GEAR_SOURCE_UNAVAILABLE" => "INSTANCE_UNAVAILABLE_SELF_TEMPORARILY_DISABLED",
        "LFG_LOCKSTATUS_TOO_LOW_LEVEL" => "INSTANCE_UNAVAILABLE_SELF_LEVEL_TOO_LOW",
        "LFG_LOCKSTATUS_TOO_HIGH_LEVEL" => "INSTANCE_UNAVAILABLE_SELF_LEVEL_TOO_HIGH",
        _ => return error.to_owned(),
    };
    crate::auction_mail::text(tag).to_owned()
}

#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub visible: bool,
    pub viewport: [f32; 2],
    pub catalog: Option<FollowerDungeons>,
    pub player_level: Option<u32>,
    pub level_ranges: HashMap<u32, DungeonLevelRange>,
    pub role: GroupRoleSnapshot,
    pub selected: Option<u32>,
    pub pending: bool,
    pub state: Option<LfgStateUpdate>,
    pub type_open: bool,
    /// Retail allows a solo player or the current group leader to volunteer as guide.
    pub leader_available: bool,
    pub volunteer_leader: bool,
}

impl Default for View {
    fn default() -> Self {
        Self {
            visible: false,
            viewport: [1280.0, 720.0],
            catalog: None,
            player_level: None,
            level_ranges: HashMap::new(),
            role: GroupRoleSnapshot::Damage,
            selected: None,
            pending: false,
            state: None,
            type_open: false,
            leader_available: true,
            volunteer_leader: false,
        }
    }
}
impl View {
    pub fn action(&mut self, action: &str) -> Option<Action> {
        if action == "follower:pvp" {
            self.visible = false;
            return Some(Action::Pvp);
        }
        if self.update_window_controls(action) || self.pending || self.matched() {
            return None;
        }
        if action == "follower:leader" && self.leader_available {
            self.volunteer_leader = !self.volunteer_leader;
        }
        self.select_role_and_dungeon(action);
        if action != "follower:queue" || !self.can_queue() {
            return None;
        }
        let dungeon = self.selected?;
        self.pending = true;
        Some(Action::Queue(QueueForLfg {
            role: self.role,
            dungeon_ids: vec![dungeon],
        }))
    }
    fn update_window_controls(&mut self, action: &str) -> bool {
        match action {
            "follower:close" => self.visible = false,
            "follower:type" => self.type_open = !self.type_open,
            "follower:type:follower" => self.type_open = false,
            _ => return false,
        }
        true
    }
    fn select_role_and_dungeon(&mut self, action: &str) {
        if let Some(role) = role_for_action(action) {
            self.role = role;
        }
        let selected_id = action
            .strip_prefix("follower:select:")
            .and_then(|id| id.parse().ok());
        if let Some(id) = selected_id.filter(|id| self.is_eligible(*id)) {
            self.selected = (self.selected != Some(id)).then_some(id);
        }
    }
    pub fn receive(&mut self, mut update: LfgStateUpdate) {
        if let Some(error) = &mut update.error {
            *error = localize_error(error);
        }
        let refused = update.error.is_some();
        let matched = update
            .snapshot
            .as_ref()
            .is_some_and(|state| state.match_found.is_some());
        if refused || matched {
            self.pending = false;
        }
        self.state = Some(update);
    }
    pub fn receive_catalog(&mut self, catalog: FollowerDungeons) {
        if self.selected.is_some_and(|id| {
            !catalog.entries.iter().any(|entry| entry.id == id) || self.level_lock_tag(id).is_some()
        }) {
            self.selected = None;
        }
        self.catalog = Some(catalog);
    }
    pub fn can_select(&self, id: u32) -> bool {
        !self.pending && !self.matched() && self.is_eligible(id)
    }
    pub fn can_queue(&self) -> bool {
        !self.pending && !self.matched() && self.selected.is_some_and(|id| self.is_eligible(id))
    }
    pub fn level_lock_message(&self, id: u32) -> Option<&'static str> {
        self.level_lock_tag(id).map(crate::auction_mail::text)
    }
    fn is_eligible(&self, id: u32) -> bool {
        self.contains(id) && self.level_lock_tag(id).is_none()
    }
    fn level_lock_tag(&self, id: u32) -> Option<&'static str> {
        let level = self.player_level?;
        let Some(range) = self.level_ranges.get(&id) else {
            return Some("INSTANCE_UNAVAILABLE_SELF_TEMPORARILY_DISABLED");
        };
        if level < range.min {
            Some("INSTANCE_UNAVAILABLE_SELF_LEVEL_TOO_LOW")
        } else if level > range.max {
            Some("INSTANCE_UNAVAILABLE_SELF_LEVEL_TOO_HIGH")
        } else {
            None
        }
    }
    pub fn matched(&self) -> bool {
        self.state
            .as_ref()
            .and_then(|update| update.snapshot.as_ref())
            .is_some_and(|state| state.match_found.is_some())
    }
    fn contains(&self, id: u32) -> bool {
        self.catalog
            .as_ref()
            .is_some_and(|catalog| catalog.entries.iter().any(|entry| entry.id == id))
    }
}
fn role_for_action(action: &str) -> Option<GroupRoleSnapshot> {
    match action {
        "follower:role:tank" => Some(GroupRoleSnapshot::Tank),
        "follower:role:healer" => Some(GroupRoleSnapshot::Healer),
        "follower:role:damage" => Some(GroupRoleSnapshot::Damage),
        _ => None,
    }
}

pub fn follower_screen(shared: &SharedContext) -> Element {
    crate::follower_dungeons_component::screen(shared)
}
