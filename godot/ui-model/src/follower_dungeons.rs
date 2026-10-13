//! Retail Mainline PVEFrame / LFDQueueFrame follower mode.
//! Source: Blizzard_GroupFinder/Mainline/{PVEFrame,LFDFrame,LFGFrame}.xml/.lua.
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

#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub visible: bool,
    pub viewport: [f32; 2],
    pub catalog: Option<FollowerDungeons>,
    pub role: GroupRoleSnapshot,
    pub selected: Option<u32>,
    pub pending: bool,
    pub state: Option<LfgStateUpdate>,
    pub type_open: bool,
    /// Binding follower-mode policy: solo guide remains visible but disabled.
    pub group_leader: bool,
    pub volunteer_leader: bool,
}

impl Default for View {
    fn default() -> Self {
        Self {
            visible: false,
            viewport: [1280.0, 720.0],
            catalog: None,
            role: GroupRoleSnapshot::Damage,
            selected: None,
            pending: false,
            state: None,
            type_open: false,
            group_leader: false,
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
        if action == "follower:leader" && self.group_leader {
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
        if let Some(id) = selected_id.filter(|id| self.contains(*id)) {
            self.selected = (self.selected != Some(id)).then_some(id);
        }
    }
    pub fn receive(&mut self, update: LfgStateUpdate) {
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
        if self
            .selected
            .is_some_and(|id| !catalog.entries.iter().any(|entry| entry.id == id))
        {
            self.selected = None;
        }
        self.catalog = Some(catalog);
    }
    pub fn can_queue(&self) -> bool {
        let idle = !self.pending && !self.matched();
        idle && self.selected.is_some_and(|id| self.contains(id))
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
