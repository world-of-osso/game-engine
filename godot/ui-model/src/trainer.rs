//! Server-authoritative ClassTrainerFrame decisions (Retail TrainerUI/Mainline).
use shared::protocol::{
    TrainerBuyFailed, TrainerBuySpell, TrainerList, TrainerService, TrainerServiceState,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainerBook {
    pub list: Option<TrainerList>,
    pub selected: Option<u32>,
    pub filters: [bool; 3],
    pub filter_menu: bool,
    pub money: u64,
    pub primary_professions: usize,
    pub confirmation: Option<u32>,
    pub pending: bool,
    pub error: String,
}
impl Default for TrainerBook {
    fn default() -> Self {
        Self {
            list: None,
            selected: None,
            filters: [true; 3],
            filter_menu: false,
            money: 0,
            primary_professions: 0,
            confirmation: None,
            pending: false,
            error: String::new(),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrainerDisplay {
    pub names: BTreeMap<u32, String>,
    pub skills: BTreeMap<u32, String>,
    pub icons: BTreeMap<u32, u32>,
}
impl TrainerDisplay {
    pub fn spell_name(&self, id: u32) -> String {
        self.names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| "Unknown".into())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainerRow {
    pub spell_id: u32,
    pub name: String,
    pub icon: u32,
    pub state: TrainerServiceState,
    pub cost: u64,
    pub requirements: String,
}
pub const fn state_index(state: TrainerServiceState) -> usize {
    match state {
        TrainerServiceState::Available => 0,
        TrainerServiceState::Unavailable => 1,
        TrainerServiceState::Known => 2,
    }
}
impl TrainerBook {
    pub fn receive_list(&mut self, list: TrainerList) {
        if self.list.as_ref().is_none_or(|old| old.npc != list.npc) {
            self.selected = None;
        }
        self.list = Some(list);
        self.pending = false;
        self.confirmation = None;
        self.error.clear();
        self.select_visible();
    }
    pub fn close(&mut self) {
        self.list = None;
        self.filter_menu = false;
        self.selected = None;
        self.confirmation = None;
        self.pending = false;
        self.error.clear();
    }
    pub fn visible_services(&self) -> impl Iterator<Item = &TrainerService> {
        self.list
            .iter()
            .flat_map(|list| &list.services)
            .filter(|s| self.filters[state_index(s.state)])
    }
    pub fn toggle_filter(&mut self, state: TrainerServiceState) {
        self.filters[state_index(state)] = !self.filters[state_index(state)];
        self.confirmation = None;
        self.select_visible();
    }
    fn select_visible(&mut self) {
        if self
            .visible_services()
            .any(|s| Some(s.spell_id) == self.selected)
        {
            return;
        }
        let selected = self
            .visible_services()
            .find(|s| s.state == TrainerServiceState::Available)
            .or_else(|| self.visible_services().next())
            .map(|s| s.spell_id);
        self.selected = selected;
    }
    pub fn select(&mut self, spell: u32) {
        if self.visible_services().any(|s| s.spell_id == spell) {
            self.selected = Some(spell);
            self.confirmation = None;
            self.error.clear();
        }
    }
    pub fn selected_service(&self) -> Option<&TrainerService> {
        self.visible_services()
            .find(|s| Some(s.spell_id) == self.selected)
    }
    pub fn can_train(&self) -> bool {
        !self.pending
            && self.selected_service().is_some_and(|s| {
                s.state == TrainerServiceState::Available
                    && self.money >= s.cost
                    && (!s.profession || self.primary_professions < 2)
            })
    }
    pub fn train(&mut self) -> Option<TrainerBuySpell> {
        if !self.can_train() {
            return None;
        }
        if self.selected_service()?.profession {
            self.confirmation = self.selected;
            return None;
        }
        self.request()
    }
    pub fn confirm(&mut self, accepted: bool) -> Option<TrainerBuySpell> {
        let spell = self.confirmation.take()?;
        if accepted && self.selected == Some(spell) && self.can_train() {
            self.request()
        } else {
            None
        }
    }
    fn request(&mut self) -> Option<TrainerBuySpell> {
        let request = TrainerBuySpell {
            npc: self.list.as_ref()?.npc,
            spell_id: self.selected?,
        };
        self.pending = true;
        self.error.clear();
        Some(request)
    }
    pub fn receive_failure(&mut self, failed: TrainerBuyFailed) {
        if self.list.as_ref().is_none_or(|list| list.npc != failed.npc) {
            return;
        }
        self.pending = false;
        self.confirmation = None;
        self.error = failed
            .reason
            .message()
            .unwrap_or("This service is unavailable.")
            .into();
    }
    pub fn rows(&self, display: &TrainerDisplay) -> Vec<TrainerRow> {
        self.visible_services()
            .map(|s| TrainerRow {
                spell_id: s.spell_id,
                name: display.spell_name(s.spell_id),
                icon: display.icons.get(&s.spell_id).copied().unwrap_or(0),
                state: s.state,
                cost: s.cost,
                requirements: requirement_text(s, display),
            })
            .collect()
    }
}
fn requirement_text(service: &TrainerService, display: &TrainerDisplay) -> String {
    if service.state == TrainerServiceState::Known {
        return "Already known".into();
    }
    let mut lines = Vec::new();
    if service.req_level > 1 {
        lines.push(format!("Level {}", service.req_level));
    }
    if service.req_skill_line != 0 {
        let skill = display
            .skills
            .get(&service.req_skill_line)
            .cloned()
            .unwrap_or_else(|| format!("Skill {}", service.req_skill_line));
        lines.push(format!("{skill} ({})", service.req_skill_rank));
    }
    lines.extend(
        service
            .req_abilities
            .iter()
            .map(|id| display.spell_name(*id)),
    );
    if lines.is_empty() {
        String::new()
    } else {
        format!("Requires {}", lines.join(", "))
    }
}
