use bevy::prelude::*;

/// Equipment slot identifiers matching DressUpFrame order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EquipSlot {
    Head,
    Shoulder,
    Chest,
    Waist,
    Legs,
    Feet,
    Wrist,
    Hands,
    Back,
    MainHand,
    OffHand,
    Ranged,
}

impl EquipSlot {
    pub const ALL: [EquipSlot; 12] = [
        Self::Head,
        Self::Shoulder,
        Self::Chest,
        Self::Waist,
        Self::Legs,
        Self::Feet,
        Self::Wrist,
        Self::Hands,
        Self::Back,
        Self::MainHand,
        Self::OffHand,
        Self::Ranged,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Head => "Head",
            Self::Shoulder => "Shoulder",
            Self::Chest => "Chest",
            Self::Waist => "Waist",
            Self::Legs => "Legs",
            Self::Feet => "Feet",
            Self::Wrist => "Wrist",
            Self::Hands => "Hands",
            Self::Back => "Back",
            Self::MainHand => "Main Hand",
            Self::OffHand => "Off Hand",
            Self::Ranged => "Ranged",
        }
    }
}

/// An item that can occupy an equipment slot.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PreviewItem {
    pub item_id: u32,
    pub name: String,
    pub icon_fdid: u32,
}

/// Runtime dress-up / transmog preview state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct DressUpState {
    /// Currently equipped items (from character data).
    pub equipped: [Option<PreviewItem>; 12],
    /// Preview overrides (user-selected items to try on).
    pub overrides: [Option<PreviewItem>; 12],
}

impl DressUpState {
    /// Returns the displayed item for a slot — override if set, else equipped.
    pub fn displayed_item(&self, slot: EquipSlot) -> Option<&PreviewItem> {
        let idx = Self::slot_index(slot);
        self.overrides[idx].as_ref().or(self.equipped[idx].as_ref())
    }

    /// Set a preview override for a slot.
    pub fn set_override(&mut self, slot: EquipSlot, item: PreviewItem) {
        self.overrides[Self::slot_index(slot)] = Some(item);
    }

    /// Clear a single slot's override.
    pub fn clear_override(&mut self, slot: EquipSlot) {
        self.overrides[Self::slot_index(slot)] = None;
    }

    /// Reset all overrides back to equipped.
    pub fn reset_all(&mut self) {
        self.overrides = Default::default();
    }

    /// Check if any slot has an override.
    pub fn has_overrides(&self) -> bool {
        self.overrides.iter().any(|o| o.is_some())
    }

    fn slot_index(slot: EquipSlot) -> usize {
        EquipSlot::ALL.iter().position(|&s| s == slot).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sword() -> PreviewItem {
        PreviewItem {
            item_id: 100,
            name: "Ashkandi".into(),
            icon_fdid: 135300,
        }
    }

    fn helm() -> PreviewItem {
        PreviewItem {
            item_id: 200,
            name: "Lionheart Helm".into(),
            icon_fdid: 135400,
        }
    }

    #[test]
    fn displayed_item_returns_equipped_when_no_override() {
        let mut state = DressUpState::default();
        state.equipped[0] = Some(helm());
        assert_eq!(
            state.displayed_item(EquipSlot::Head).unwrap().name,
            "Lionheart Helm"
        );
    }

    #[test]
    fn override_takes_priority_over_equipped() {
        let mut state = DressUpState::default();
        state.equipped[0] = Some(helm());
        state.set_override(
            EquipSlot::Head,
            PreviewItem {
                item_id: 300,
                name: "Crown of Woe".into(),
                icon_fdid: 0,
            },
        );
        assert_eq!(
            state.displayed_item(EquipSlot::Head).unwrap().name,
            "Crown of Woe"
        );
    }

    #[test]
    fn clear_override_reverts_to_equipped() {
        let mut state = DressUpState::default();
        state.equipped[0] = Some(helm());
        state.set_override(EquipSlot::Head, sword());
        state.clear_override(EquipSlot::Head);
        assert_eq!(
            state.displayed_item(EquipSlot::Head).unwrap().name,
            "Lionheart Helm"
        );
    }

    #[test]
    fn reset_all_clears_every_override() {
        let mut state = DressUpState::default();
        state.set_override(EquipSlot::Head, helm());
        state.set_override(EquipSlot::MainHand, sword());
        assert!(state.has_overrides());
        state.reset_all();
        assert!(!state.has_overrides());
    }

    #[test]
    fn empty_slot_returns_none() {
        let state = DressUpState::default();
        assert!(state.displayed_item(EquipSlot::Chest).is_none());
    }

    #[test]
    fn slot_labels_match_count() {
        assert_eq!(EquipSlot::ALL.len(), 12);
        for slot in EquipSlot::ALL {
            assert!(!slot.label().is_empty());
        }
    }
}
