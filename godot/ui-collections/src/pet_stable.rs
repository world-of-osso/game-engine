//! Retail Stable UI session. Only server replies change pet records.
use shared::protocol::{
    PET_ACTIVE_SLOTS, PET_STABLE_SLOTS, PetStableContents, PetStableOperation, PetStableRequest,
    StablePetInfo,
};

#[derive(Default)]
pub struct StableSession {
    pub npc: Option<u64>,
    pub contents: Option<PetStableContents>,
    pub selected: Option<u16>,
    pub pending: bool,
    pub requested_selection: Option<u16>,
    pub search: String,
    pub first_row: usize,
    pub rename_slot: Option<u16>,
    pub spec_picker: bool,
}

impl StableSession {
    pub fn open(&mut self, npc: u64) -> PetStableRequest {
        *self = Self {
            npc: Some(npc),
            pending: true,
            ..Default::default()
        };
        PetStableRequest {
            npc,
            operation: PetStableOperation::List,
        }
    }

    pub fn receive(&mut self, contents: PetStableContents) {
        if self.npc != Some(contents.npc) {
            return;
        }
        self.pending = false;
        self.selected = self
            .requested_selection
            .take()
            .or(self.selected)
            .filter(|slot| contents.pets.iter().any(|pet| pet.slot == *slot))
            .or(contents.current)
            .or_else(|| contents.pets.first().map(|pet| pet.slot));
        self.contents = Some(contents);
    }

    pub fn select(&mut self, slot: u16) {
        if !self.pending
            && self
                .contents
                .as_ref()
                .is_some_and(|contents| contents.pets.iter().any(|pet| pet.slot == slot))
        {
            self.selected = Some(slot);
        }
    }

    pub fn selected_pet(&self) -> Option<&StablePetInfo> {
        let selected = self.selected?;
        self.contents
            .as_ref()?
            .pets
            .iter()
            .find(|pet| pet.slot == selected)
    }

    pub fn close(&mut self) {
        *self = Self::default();
    }

    pub fn receive_failed(&mut self) {
        self.pending = false;
        self.requested_selection = None;
    }

    pub fn request(&mut self, operation: PetStableOperation) -> Option<PetStableRequest> {
        if self.pending || self.contents.is_none() {
            return None;
        }
        let npc = self.npc?;
        self.pending = true;
        Some(PetStableRequest { npc, operation })
    }

    pub fn swap(&mut self, from: u16, to: u16) -> Option<PetStableRequest> {
        if from == to {
            return None;
        }
        let request = self.request(PetStableOperation::Swap { from, to })?;
        self.requested_selection = Some(to);
        Some(request)
    }

    pub fn toggle(&mut self) -> Option<PetStableRequest> {
        let from = self.selected_pet()?.slot;
        let contents = self.contents.as_ref()?;
        let to = if usize::from(from) < PET_ACTIVE_SLOTS {
            first_unused_stable_slot(contents)
        } else {
            first_unused_active_slot(contents)
        };
        self.swap(from, to)
    }

    pub fn begin_rename(&mut self) -> Option<String> {
        if self.pending {
            return None;
        }
        let pet = self.selected_pet()?;
        let (slot, name) = (pet.slot, pet.name.clone());
        self.rename_slot = Some(slot);
        Some(name)
    }

    pub fn rename(&mut self, name: String) -> Option<PetStableRequest> {
        let slot = self.rename_slot?;
        let request = self.request(PetStableOperation::Rename { slot, name })?;
        self.rename_slot = None;
        Some(request)
    }

    pub fn abandon(&mut self, slot: u16) -> Option<PetStableRequest> {
        self.request(PetStableOperation::Abandon { slot })
    }

    pub fn choose_spec(&mut self, specialization: u32) -> Option<PetStableRequest> {
        let slot = self.selected_pet()?.slot;
        let contents = self.contents.as_ref()?;
        if !contents
            .specializations
            .iter()
            .any(|spec| spec.id == specialization)
        {
            return None;
        }
        let request = self.request(PetStableOperation::Specialization {
            slot,
            specialization,
        })?;
        self.spec_picker = false;
        Some(request)
    }

    pub fn set_search(&mut self, value: String) {
        if self.search != value {
            self.search = value;
            self.first_row = 0;
        }
    }

    pub fn stabled_pets(&self) -> Vec<&StablePetInfo> {
        let query = self.search.to_lowercase();
        let Some(contents) = &self.contents else {
            return Vec::new();
        };
        let mut pets: Vec<_> = contents
            .pets
            .iter()
            .filter(|pet| {
                usize::from(pet.slot) >= PET_ACTIVE_SLOTS
                    && (query.is_empty()
                        || pet.name.to_lowercase().contains(&query)
                        || pet.family.to_lowercase().contains(&query))
            })
            .collect();
        pets.sort_by(|left, right| {
            left.specialization
                .cmp(&right.specialization)
                .then_with(|| left.name.cmp(&right.name))
        });
        pets
    }
}

// Retail StableUI FindFirstUnusedStableSlot checks the secondary slot last.
pub fn first_unused_stable_slot(contents: &PetStableContents) -> u16 {
    let occupied = |slot| contents.pets.iter().any(|pet| pet.slot == slot);
    let first = PET_ACTIVE_SLOTS as u16;
    (first + 1..(PET_ACTIVE_SLOTS + PET_STABLE_SLOTS) as u16)
        .find(|slot| !occupied(*slot))
        .unwrap_or_else(|| if !occupied(first) { first } else { first + 1 })
}

pub fn first_unused_active_slot(contents: &PetStableContents) -> u16 {
    (0..PET_ACTIVE_SLOTS)
        .find(|slot| {
            contents.unlocked_active[*slot]
                && !contents
                    .pets
                    .iter()
                    .any(|pet| usize::from(pet.slot) == *slot)
        })
        .map_or(0, |slot| slot as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::PetSpecializationInfo;

    fn wolf(slot: u16) -> StablePetInfo {
        StablePetInfo {
            slot,
            creature_entry: 299,
            display_id: 1413,
            name: "Ash".into(),
            level: 10,
            family: "Wolf".into(),
            family_icon: 132203,
            specialization: Some(74),
            health: 23.0,
            max_health: 378.0,
        }
    }

    fn contents() -> PetStableContents {
        PetStableContents {
            npc: 42,
            current: Some(0),
            unlocked_active: [true, true, false, false, false],
            pets: vec![wolf(0)],
            specializations: vec![PetSpecializationInfo {
                id: 74,
                name: "Ferocity".into(),
            }],
        }
    }

    #[test]
    fn stable_specialization_uses_retail_dropdown_and_radio_menu_in_both_skins() {
        use crate::pet_stable_component::{StableFrameView, pet_stable_screen};
        use ui_toolkit::{
            atlas::{ActiveSkin, set_thread_skin},
            registry::FrameRegistry,
            screen::{Screen, SharedContext},
        };
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            set_thread_skin(skin);
            let mut session = StableSession::default();
            session.open(42);
            session.receive(contents());
            session.spec_picker = true;
            let mut shared = SharedContext::new();
            shared.insert(StableFrameView::from_session(
                &session,
                [1920.0, 1080.0],
                false,
            ));
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(pet_stable_screen).sync(&shared, &mut registry);
            let dropdown = registry
                .get(registry.get_by_name("StableSpecButton").unwrap())
                .unwrap();
            assert_eq!(dropdown.width.value(), 135.0);
            assert_eq!(dropdown.onclick.as_deref(), Some("stable_spec_picker"));
            let choice = registry
                .get(registry.get_by_name("StableSpec74").unwrap())
                .unwrap();
            assert_eq!(choice.height.value(), 20.0);
            assert_eq!(choice.onclick.as_deref(), Some("stable_spec:74"));
            assert!(registry.get_by_name("StableSpec74Box").is_some());
        }
    }

    #[test]
    fn stable_toggle_targets_first_non_secondary_stabled_slot_and_waits_for_server() {
        let mut session = StableSession::default();
        assert_eq!(session.open(42).operation, PetStableOperation::List);
        session.receive(contents());
        let request = session.toggle().expect("Stable button request");
        assert_eq!(request.npc, 42);
        assert_eq!(
            request.operation,
            PetStableOperation::Swap { from: 0, to: 6 }
        );
        assert_eq!(
            session.selected_pet().unwrap().slot,
            0,
            "no optimistic pet mutation"
        );
        assert!(session.toggle().is_none(), "pending operation duplicated");
        let mut reply = contents();
        reply.current = None;
        reply.pets[0].slot = 6;
        session.receive(reply);
        assert_eq!(session.selected_pet().unwrap().slot, 6);
        assert_eq!(session.selected_pet().unwrap().health, 23.0);
    }

    #[test]
    fn stable_make_active_uses_unlocked_empty_slot_then_first_active_when_full() {
        let mut session = StableSession::default();
        session.open(42);
        let mut reply = contents();
        reply.pets.push(wolf(204));
        session.receive(reply);
        session.select(204);
        assert_eq!(
            session.toggle().unwrap().operation,
            PetStableOperation::Swap { from: 204, to: 1 }
        );
        let mut reply = contents();
        reply.pets.extend([wolf(1), wolf(204)]);
        session.receive(reply);
        session.select(204);
        assert_eq!(
            session.toggle().unwrap().operation,
            PetStableOperation::Swap { from: 204, to: 0 }
        );
    }

    #[test]
    fn stable_rename_spec_and_abandon_emit_requests_without_optimistic_mutation() {
        let mut session = StableSession::default();
        session.open(42);
        session.receive(contents());
        assert_eq!(session.begin_rename(), Some("Ash".into()));
        assert_eq!(
            session.rename("Ember".into()).unwrap().operation,
            PetStableOperation::Rename {
                slot: 0,
                name: "Ember".into()
            }
        );
        assert_eq!(session.selected_pet().unwrap().name, "Ash");
        session.receive_failed();
        assert_eq!(session.selected_pet().unwrap().name, "Ash");
        assert!(session.choose_spec(999).is_none());
        assert_eq!(
            session.choose_spec(74).unwrap().operation,
            PetStableOperation::Specialization {
                slot: 0,
                specialization: 74
            }
        );
        session.receive(contents());
        assert_eq!(
            session.abandon(0).unwrap().operation,
            PetStableOperation::Abandon { slot: 0 }
        );
        assert_eq!(session.contents.as_ref().unwrap().pets.len(), 1);
        session.close();
        session.receive(contents());
        assert!(session.contents.is_none());
        assert!(session.npc.is_none());
    }

    #[test]
    fn stable_search_has_more_than_one_page_of_real_records() {
        let mut session = StableSession::default();
        session.open(42);
        let mut reply = contents();
        for slot in 6..17 {
            let mut pet = wolf(slot);
            pet.name = "Ember".into();
            reply.pets.push(pet);
        }
        session.receive(reply);
        session.set_search("ember".into());
        assert_eq!(session.stabled_pets().len(), 11);
        assert_eq!(session.stabled_pets().iter().skip(7).count(), 4);
        session.first_row = 7;
        session.set_search("missing".into());
        assert!(session.stabled_pets().is_empty());
        assert_eq!(session.first_row, 0);
    }

    #[test]
    fn stable_reply_from_closed_npc_does_not_open_or_populate_session() {
        let mut session = StableSession::default();
        session.receive(contents());
        assert!(session.npc.is_none());
        assert!(session.contents.is_none());
        session.open(99);
        session.receive(contents());
        assert!(session.contents.is_none());
        assert!(session.pending);
    }
}
