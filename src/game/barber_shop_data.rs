use bevy::prelude::*;

use crate::auction_house_data::Money;

/// Texture FDIDs for the barber shop frame.
pub mod textures {
    /// Left arrow button (normal state).
    pub const ARROW_LEFT: u32 = 136487;
    /// Right arrow button (normal state).
    pub const ARROW_RIGHT: u32 = 136490;
    /// Panel button (normal state) — used for Accept/Cancel.
    pub const PANEL_BUTTON_UP: u32 = 130828;
    /// Panel button (pressed state).
    pub const PANEL_BUTTON_DOWN: u32 = 130825;
    /// Panel button highlight.
    pub const PANEL_BUTTON_HIGHLIGHT: u32 = 130826;
    /// Model preview backdrop.
    pub const MODEL_BACKDROP: u32 = 131081;
}

/// A single customization category with its available choices.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomizationDef {
    pub label: &'static str,
    pub choices: &'static [&'static str],
}

/// All available barber shop customizations.
pub static CUSTOMIZATIONS: &[CustomizationDef] = &[
    CustomizationDef {
        label: "Hair Style",
        choices: &[
            "Style 1", "Style 2", "Style 3", "Style 4", "Style 5", "Style 6", "Style 7", "Style 8",
        ],
    },
    CustomizationDef {
        label: "Hair Color",
        choices: &[
            "Black", "Brown", "Blonde", "Auburn", "Red", "White", "Gray", "Blue",
        ],
    },
    CustomizationDef {
        label: "Facial Hair",
        choices: &["None", "Goatee", "Full Beard", "Mustache", "Sideburns"],
    },
    CustomizationDef {
        label: "Skin Color",
        choices: &["Light", "Medium", "Tan", "Dark"],
    },
    CustomizationDef {
        label: "Face",
        choices: &["Face 1", "Face 2", "Face 3", "Face 4", "Face 5"],
    },
];

/// Cost per changed option (copper).
const COST_PER_CHANGE: u64 = 10_000; // 1 gold

/// Runtime barber shop state.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct BarberShopState {
    /// Current selection index per customization (parallel to CUSTOMIZATIONS).
    pub selections: Vec<usize>,
    /// Original selection indices (before entering barber shop).
    pub original: Vec<usize>,
}

impl Default for BarberShopState {
    fn default() -> Self {
        let selections: Vec<usize> = CUSTOMIZATIONS.iter().map(|_| 0).collect();
        Self {
            original: selections.clone(),
            selections,
        }
    }
}

impl BarberShopState {
    /// Get the display value for a customization index.
    pub fn selected_value(&self, option_index: usize) -> &'static str {
        let sel = self.selections.get(option_index).copied().unwrap_or(0);
        CUSTOMIZATIONS
            .get(option_index)
            .and_then(|def| def.choices.get(sel))
            .unwrap_or(&"???")
    }

    /// Cycle selection forward, wrapping around.
    pub fn next_choice(&mut self, option_index: usize) {
        let Some(def) = CUSTOMIZATIONS.get(option_index) else {
            return;
        };
        let sel = self.selections.get_mut(option_index).unwrap();
        *sel = (*sel + 1) % def.choices.len();
    }

    /// Cycle selection backward, wrapping around.
    pub fn prev_choice(&mut self, option_index: usize) {
        let Some(def) = CUSTOMIZATIONS.get(option_index) else {
            return;
        };
        let sel = self.selections.get_mut(option_index).unwrap();
        *sel = sel.checked_sub(1).unwrap_or(def.choices.len() - 1);
    }

    /// Number of options that differ from original.
    pub fn changed_count(&self) -> usize {
        self.selections
            .iter()
            .zip(&self.original)
            .filter(|(a, b)| a != b)
            .count()
    }

    /// Total cost based on number of changed options.
    pub fn total_cost(&self) -> Money {
        let changes = self.changed_count() as u64;
        Money(changes * COST_PER_CHANGE)
    }

    /// Cost display string ("Free" if no changes).
    pub fn cost_display(&self) -> String {
        let cost = self.total_cost();
        if cost.0 == 0 {
            "Free".into()
        } else {
            cost.display()
        }
    }

    /// Reset selections to original values.
    pub fn reset(&mut self) {
        self.selections = self.original.clone();
    }

    /// Apply current selections as the new baseline.
    pub fn apply(&mut self) {
        self.original = self.selections.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_starts_at_first_choice() {
        let state = BarberShopState::default();
        assert_eq!(state.selections.len(), CUSTOMIZATIONS.len());
        assert_eq!(state.selected_value(0), "Style 1");
        assert_eq!(state.selected_value(1), "Black");
    }

    #[test]
    fn next_choice_cycles_forward() {
        let mut state = BarberShopState::default();
        state.next_choice(0);
        assert_eq!(state.selected_value(0), "Style 2");
        state.next_choice(0);
        assert_eq!(state.selected_value(0), "Style 3");
    }

    #[test]
    fn next_choice_wraps_around() {
        let mut state = BarberShopState::default();
        let hair_count = CUSTOMIZATIONS[0].choices.len();
        for _ in 0..hair_count {
            state.next_choice(0);
        }
        assert_eq!(state.selected_value(0), "Style 1");
    }

    #[test]
    fn prev_choice_wraps_to_last() {
        let mut state = BarberShopState::default();
        state.prev_choice(0);
        let last = *CUSTOMIZATIONS[0].choices.last().unwrap();
        assert_eq!(state.selected_value(0), last);
    }

    #[test]
    fn changed_count_and_cost() {
        let mut state = BarberShopState::default();
        assert_eq!(state.changed_count(), 0);
        assert_eq!(state.cost_display(), "Free");

        state.next_choice(0);
        assert_eq!(state.changed_count(), 1);
        assert_eq!(state.total_cost(), Money(10_000));

        state.next_choice(1);
        assert_eq!(state.changed_count(), 2);
        assert_eq!(state.total_cost(), Money(20_000));
    }

    #[test]
    fn reset_restores_original() {
        let mut state = BarberShopState::default();
        state.next_choice(0);
        state.next_choice(1);
        state.reset();
        assert_eq!(state.changed_count(), 0);
        assert_eq!(state.selected_value(0), "Style 1");
    }

    #[test]
    fn apply_updates_baseline() {
        let mut state = BarberShopState::default();
        state.next_choice(0);
        state.apply();
        assert_eq!(state.changed_count(), 0);
        assert_eq!(state.selected_value(0), "Style 2");
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::ARROW_LEFT, 0);
        assert_ne!(textures::ARROW_RIGHT, 0);
        assert_ne!(textures::PANEL_BUTTON_UP, 0);
        assert_ne!(textures::PANEL_BUTTON_DOWN, 0);
        assert_ne!(textures::MODEL_BACKDROP, 0);
    }
}
