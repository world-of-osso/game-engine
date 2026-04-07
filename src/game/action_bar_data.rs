use bevy::prelude::*;

const BAR_COUNT: usize = 5;
const SLOTS_PER_BAR: usize = 12;

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
}
