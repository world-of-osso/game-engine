use bevy::prelude::*;

const BAR_COUNT: usize = 5;
const SLOTS_PER_BAR: usize = 12;

/// Texture FDIDs for action bar chrome and cooldown overlays.
pub mod textures {
    /// Action bar atlas sheet (slot backgrounds, borders, flash, highlight).
    pub const ACTION_BAR_ATLAS: u32 = 4613342;
    /// Cooldown sweep overlay (clock-wipe dark overlay).
    pub const COOLDOWN_SWEEP: u32 = 131006;
    /// Cooldown edge glow (drawn at sweep boundary).
    pub const COOLDOWN_EDGE: u32 = 131008;
    /// Cooldown finished star burst.
    pub const COOLDOWN_STAR: u32 = 131010;
}

/// Atlas region names used by slot textures.
pub mod atlas {
    pub const SLOT_FRAME: &str = "ui-hud-actionbar-iconframe";
    pub const SLOT_FRAME_EXTRA: &str = "ui-hud-actionbar-iconframe-addrow";
    pub const SLOT_PRESSED: &str = "ui-hud-actionbar-iconframe-down";
    pub const SLOT_PRESSED_EXTRA: &str = "ui-hud-actionbar-iconframe-addrow-down";
    pub const SLOT_HIGHLIGHT: &str = "ui-hud-actionbar-iconframe-mouseover";
    pub const SLOT_BORDER: &str = "ui-hud-actionbar-iconframe-border";
    pub const SLOT_FLASH: &str = "ui-hud-actionbar-iconframe-flash";
}

/// Which bar a slot belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BarKind {
    Main,
    BottomLeft,
    BottomRight,
    Right,
    Left,
}

impl BarKind {
    pub const ALL: [BarKind; BAR_COUNT] = [
        BarKind::Main,
        BarKind::BottomLeft,
        BarKind::BottomRight,
        BarKind::Right,
        BarKind::Left,
    ];

    fn index(self) -> usize {
        match self {
            BarKind::Main => 0,
            BarKind::BottomLeft => 1,
            BarKind::BottomRight => 2,
            BarKind::Right => 3,
            BarKind::Left => 4,
        }
    }
}

/// Contents of a single action bar slot.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ActionSlot {
    /// Display name (empty = slot is empty).
    pub name: String,
    /// Icon texture FDID (0 = no icon).
    pub icon_fdid: u32,
    /// Cooldown remaining in seconds (0 = ready).
    pub cooldown_remaining: f32,
    /// Total cooldown duration for progress display.
    pub cooldown_total: f32,
    /// Whether the ability can be used right now.
    pub usable: bool,
    /// Whether the player lacks the resource (mana/energy/rage).
    pub out_of_resource: bool,
    /// Stack count (0 = hide count).
    pub count: u32,
}

impl ActionSlot {
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }

    /// Cooldown progress 0.0 (just started) to 1.0 (ready).
    pub fn cooldown_progress(&self) -> f32 {
        if self.cooldown_total <= 0.0 {
            return 1.0;
        }
        let elapsed = self.cooldown_total - self.cooldown_remaining;
        (elapsed / self.cooldown_total).clamp(0.0, 1.0)
    }

    pub fn is_on_cooldown(&self) -> bool {
        self.cooldown_remaining > 0.0
    }
}

/// Runtime state for all action bars.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ActionBarState {
    bars: [[ActionSlot; SLOTS_PER_BAR]; BAR_COUNT],
}

impl Default for ActionBarState {
    fn default() -> Self {
        Self {
            bars: std::array::from_fn(|_| std::array::from_fn(|_| ActionSlot::default())),
        }
    }
}

impl ActionBarState {
    pub fn slot(&self, bar: BarKind, index: usize) -> &ActionSlot {
        &self.bars[bar.index()][index]
    }

    pub fn slot_mut(&mut self, bar: BarKind, index: usize) -> &mut ActionSlot {
        &mut self.bars[bar.index()][index]
    }

    pub fn bar_slots(&self, bar: BarKind) -> &[ActionSlot; SLOTS_PER_BAR] {
        &self.bars[bar.index()]
    }

    /// Tick all cooldowns by `dt` seconds.
    pub fn tick_cooldowns(&mut self, dt: f32) {
        for bar in &mut self.bars {
            for slot in bar {
                if slot.cooldown_remaining > 0.0 {
                    slot.cooldown_remaining = (slot.cooldown_remaining - dt).max(0.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_has_empty_slots() {
        let state = ActionBarState::default();
        for bar in BarKind::ALL {
            for i in 0..SLOTS_PER_BAR {
                assert!(state.slot(bar, i).is_empty());
            }
        }
    }

    #[test]
    fn slot_mut_sets_ability() {
        let mut state = ActionBarState::default();
        let slot = state.slot_mut(BarKind::Main, 0);
        slot.name = "Fireball".into();
        slot.icon_fdid = 135810;
        slot.usable = true;
        assert_eq!(state.slot(BarKind::Main, 0).name, "Fireball");
        assert!(!state.slot(BarKind::Main, 0).is_empty());
    }

    #[test]
    fn cooldown_progress_when_ready() {
        let slot = ActionSlot::default();
        assert!((slot.cooldown_progress() - 1.0).abs() < 0.01);
        assert!(!slot.is_on_cooldown());
    }

    #[test]
    fn cooldown_progress_midway() {
        let slot = ActionSlot {
            cooldown_remaining: 5.0,
            cooldown_total: 10.0,
            ..Default::default()
        };
        assert!(slot.is_on_cooldown());
        assert!((slot.cooldown_progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn tick_cooldowns_decrements() {
        let mut state = ActionBarState::default();
        state.slot_mut(BarKind::Main, 0).cooldown_remaining = 3.0;
        state.slot_mut(BarKind::Main, 0).cooldown_total = 10.0;
        state.tick_cooldowns(1.0);
        assert!((state.slot(BarKind::Main, 0).cooldown_remaining - 2.0).abs() < 0.01);
    }

    #[test]
    fn tick_cooldowns_clamps_at_zero() {
        let mut state = ActionBarState::default();
        state.slot_mut(BarKind::BottomLeft, 5).cooldown_remaining = 0.5;
        state.slot_mut(BarKind::BottomLeft, 5).cooldown_total = 8.0;
        state.tick_cooldowns(2.0);
        assert_eq!(state.slot(BarKind::BottomLeft, 5).cooldown_remaining, 0.0);
    }

    #[test]
    fn out_of_resource_tracks_mana_state() {
        let mut state = ActionBarState::default();
        let slot = state.slot_mut(BarKind::Main, 2);
        slot.name = "Frostbolt".into();
        slot.usable = false;
        slot.out_of_resource = true;
        assert!(!state.slot(BarKind::Main, 2).usable);
        assert!(state.slot(BarKind::Main, 2).out_of_resource);
    }

    #[test]
    fn bar_slots_returns_full_bar() {
        let state = ActionBarState::default();
        assert_eq!(state.bar_slots(BarKind::Main).len(), 12);
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::ACTION_BAR_ATLAS, 0);
        assert_ne!(textures::COOLDOWN_SWEEP, 0);
        assert_ne!(textures::COOLDOWN_EDGE, 0);
        assert_ne!(textures::COOLDOWN_STAR, 0);
    }

    #[test]
    fn atlas_names_are_nonempty() {
        assert!(!atlas::SLOT_FRAME.is_empty());
        assert!(!atlas::SLOT_HIGHLIGHT.is_empty());
        assert!(!atlas::SLOT_BORDER.is_empty());
        assert!(!atlas::SLOT_FLASH.is_empty());
    }

    #[test]
    fn cooldown_progress_just_started() {
        let slot = ActionSlot {
            cooldown_remaining: 10.0,
            cooldown_total: 10.0,
            ..Default::default()
        };
        assert!((slot.cooldown_progress() - 0.0).abs() < 0.01);
    }

    #[test]
    fn cooldown_progress_finished() {
        let slot = ActionSlot {
            cooldown_remaining: 0.0,
            cooldown_total: 10.0,
            ..Default::default()
        };
        assert!((slot.cooldown_progress() - 1.0).abs() < 0.01);
        assert!(!slot.is_on_cooldown());
    }

    #[test]
    fn tick_cooldowns_across_multiple_bars() {
        let mut state = ActionBarState::default();
        state.slot_mut(BarKind::Main, 0).cooldown_remaining = 5.0;
        state.slot_mut(BarKind::Main, 0).cooldown_total = 10.0;
        state.slot_mut(BarKind::Right, 3).cooldown_remaining = 2.0;
        state.slot_mut(BarKind::Right, 3).cooldown_total = 8.0;
        state.tick_cooldowns(1.5);
        assert!((state.slot(BarKind::Main, 0).cooldown_remaining - 3.5).abs() < 0.01);
        assert!((state.slot(BarKind::Right, 3).cooldown_remaining - 0.5).abs() < 0.01);
    }

    #[test]
    fn tick_cooldowns_skips_ready_slots() {
        let mut state = ActionBarState::default();
        // Slot with no cooldown should stay at 0
        state.slot_mut(BarKind::Main, 1).name = "Polymorph".into();
        state.tick_cooldowns(5.0);
        assert_eq!(state.slot(BarKind::Main, 1).cooldown_remaining, 0.0);
    }

    #[test]
    fn slot_content_assignment_and_clear() {
        let mut state = ActionBarState::default();
        let slot = state.slot_mut(BarKind::BottomRight, 11);
        slot.name = "Heroic Strike".into();
        slot.icon_fdid = 132282;
        slot.usable = true;
        slot.count = 3;
        assert_eq!(state.slot(BarKind::BottomRight, 11).name, "Heroic Strike");
        assert_eq!(state.slot(BarKind::BottomRight, 11).count, 3);

        // Clear the slot
        *state.slot_mut(BarKind::BottomRight, 11) = ActionSlot::default();
        assert!(state.slot(BarKind::BottomRight, 11).is_empty());
        assert_eq!(state.slot(BarKind::BottomRight, 11).count, 0);
    }

    #[test]
    fn usable_on_cooldown_and_out_of_resource() {
        let slot = ActionSlot {
            name: "Flash Heal".into(),
            usable: false,
            out_of_resource: true,
            cooldown_remaining: 1.5,
            cooldown_total: 1.5,
            ..Default::default()
        };
        assert!(!slot.usable);
        assert!(slot.out_of_resource);
        assert!(slot.is_on_cooldown());
        assert!((slot.cooldown_progress() - 0.0).abs() < 0.01);
    }

    #[test]
    fn bar_kind_all_covers_five_bars() {
        assert_eq!(BarKind::ALL.len(), 5);
    }
}
