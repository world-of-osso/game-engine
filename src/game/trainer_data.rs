//! Client trainer state: the server's `TrainerList` for the open trainer NPC, the
//! selected service and scroll position of the Retail `ClassTrainerFrame`, and the
//! purchases the frame asks the network layer to send (`TrainerRequest`).

use bevy::prelude::*;
use shared::protocol::{TrainerList, TrainerService, TrainerServiceState};

/// Retail `CLASS_TRAINER_SKILLS_DISPLAYED` (Blizzard_TrainerUI.lua:1).
pub const TRAINER_SKILLS_DISPLAYED: usize = 7;

/// The open trainer frame (`npc` = server entity bits; `None` = closed).
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct TrainerState {
    pub npc: Option<u64>,
    pub npc_name: String,
    pub trainer_id: u32,
    pub greeting: String,
    pub services: Vec<TrainerService>,
    /// Selected service (spell id).
    pub selected: Option<u32>,
    /// First visible row.
    pub scroll: usize,
}

/// A purchase for the open trainer (`BuyTrainerService`).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainerRequest {
    pub spell_id: u32,
}

impl TrainerState {
    pub fn is_open(&self) -> bool {
        self.npc.is_some()
    }

    /// A list from the server: opens the frame, or refreshes the open frame of the
    /// same trainer after a purchase (keeping the scroll position).
    pub fn apply_list(&mut self, list: TrainerList, npc_name: String) {
        let opening = self.npc != Some(list.npc);
        if opening {
            *self = Self {
                npc: Some(list.npc),
                npc_name,
                ..Self::default()
            };
        }
        self.trainer_id = list.trainer_id;
        self.greeting = list.greeting;
        self.services = list.services;
        self.scroll = self.scroll.min(self.max_scroll());
        if self
            .selected
            .is_none_or(|spell| self.service(spell).is_none())
        {
            self.selected = self.nearest_learnable();
        }
        if opening {
            self.scroll_to_selected();
        }
    }

    pub fn close(&mut self) {
        *self = Self::default();
    }

    pub fn service(&self, spell_id: u32) -> Option<&TrainerService> {
        self.services
            .iter()
            .find(|service| service.spell_id == spell_id)
    }

    /// `ClassTrainer_SelectNearestLearnableSkill`: the first available service,
    /// otherwise the first one.
    fn nearest_learnable(&self) -> Option<u32> {
        self.services
            .iter()
            .find(|service| service.state == TrainerServiceState::Available)
            .or(self.services.first())
            .map(|service| service.spell_id)
    }

    /// `ClassTrainer_SelectNearestLearnableSkill` scrolls the selection into view.
    fn scroll_to_selected(&mut self) {
        if let Some(index) = self
            .selected
            .and_then(|spell| self.services.iter().position(|s| s.spell_id == spell))
            && !(self.scroll..self.scroll + TRAINER_SKILLS_DISPLAYED).contains(&index)
        {
            self.scroll = index.min(self.max_scroll());
        }
    }

    pub fn max_scroll(&self) -> usize {
        self.services.len().saturating_sub(TRAINER_SKILLS_DISPLAYED)
    }

    pub fn scroll_by(&mut self, rows: isize) {
        self.scroll = self
            .scroll
            .saturating_add_signed(rows)
            .min(self.max_scroll());
    }

    pub fn visible_services(&self) -> &[TrainerService] {
        let start = self.scroll.min(self.services.len());
        let end = (start + TRAINER_SKILLS_DISPLAYED).min(self.services.len());
        &self.services[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(spell_id: u32, state: TrainerServiceState) -> TrainerService {
        TrainerService {
            spell_id,
            cost: 10,
            state,
            req_level: 0,
            req_skill_line: 0,
            req_skill_rank: 0,
            req_abilities: vec![],
            profession: false,
        }
    }

    fn list(services: Vec<TrainerService>) -> TrainerList {
        TrainerList {
            npc: 42,
            trainer_id: 163,
            greeting: "Greetings!".into(),
            services,
        }
    }

    #[test]
    fn opening_selects_the_first_available_service() {
        let mut state = TrainerState::default();
        state.apply_list(
            list(vec![
                service(2393, TrainerServiceState::Unavailable),
                service(264617, TrainerServiceState::Available),
            ]),
            "Georgio Bolero".into(),
        );
        assert_eq!(state.npc, Some(42));
        assert_eq!(state.npc_name, "Georgio Bolero");
        assert_eq!(state.selected, Some(264617));
    }

    #[test]
    fn opening_scrolls_the_selected_service_into_view() {
        let mut services: Vec<_> = (0..9)
            .map(|i| service(1000 + i, TrainerServiceState::Unavailable))
            .collect();
        services.push(service(264617, TrainerServiceState::Available));
        let mut state = TrainerState::default();
        state.apply_list(list(services), "Georgio Bolero".into());
        assert_eq!(state.selected, Some(264617));
        assert_eq!(state.scroll, 3);
        assert_eq!(state.visible_services().last().unwrap().spell_id, 264617);
    }

    #[test]
    fn a_refreshed_list_keeps_selection_and_scroll() {
        let mut state = TrainerState::default();
        let services: Vec<_> = (0..10)
            .map(|i| service(1000 + i, TrainerServiceState::Available))
            .collect();
        state.apply_list(list(services.clone()), "Georgio Bolero".into());
        state.selected = Some(1005);
        state.scroll_by(2);
        state.apply_list(list(services), String::new());
        assert_eq!((state.selected, state.scroll), (Some(1005), 2));
        assert_eq!(state.npc_name, "Georgio Bolero");
        assert_eq!(state.visible_services().len(), TRAINER_SKILLS_DISPLAYED);
    }

    #[test]
    fn scrolling_stops_at_the_last_page() {
        let mut state = TrainerState::default();
        let services: Vec<_> = (0..9)
            .map(|i| service(1000 + i, TrainerServiceState::Available))
            .collect();
        state.apply_list(list(services), "Georgio Bolero".into());
        state.scroll_by(10);
        assert_eq!(state.scroll, 2);
        state.scroll_by(-5);
        assert_eq!(state.scroll, 0);
    }
}
