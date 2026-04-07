use bevy::prelude::*;

/// Texture FDIDs for the loss-of-control frame.
pub mod textures {
    /// Stun icon (Hammer of Justice).
    pub const ICON_STUN: u32 = 135963;
    /// Root icon (Frost Nova).
    pub const ICON_ROOT: u32 = 135848;
    /// Fear icon (Psychic Scream).
    pub const ICON_FEAR: u32 = 136184;
    /// Bar fill texture (shared with casting bar).
    pub const BAR_FILL: u32 = 4505182;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CCType {
    #[default]
    None,
    Stun,
    Fear,
    Incapacitate,
    Disorient,
    Root,
    Silence,
    Polymorph,
}

impl CCType {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Stun => "Stunned",
            Self::Fear => "Feared",
            Self::Incapacitate => "Incapacitated",
            Self::Disorient => "Disoriented",
            Self::Root => "Rooted",
            Self::Silence => "Silenced",
            Self::Polymorph => "Polymorphed",
        }
    }

    pub fn prevents_movement(self) -> bool {
        matches!(
            self,
            Self::Stun
                | Self::Fear
                | Self::Incapacitate
                | Self::Disorient
                | Self::Root
                | Self::Polymorph
        )
    }

    pub fn prevents_casting(self) -> bool {
        matches!(
            self,
            Self::Stun
                | Self::Fear
                | Self::Incapacitate
                | Self::Disorient
                | Self::Silence
                | Self::Polymorph
        )
    }
}

/// Runtime loss-of-control state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct LossOfControlData {
    pub active: Option<ActiveCC>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveCC {
    pub cc_type: CCType,
    pub ability_name: String,
    pub icon_fdid: u32,
    pub duration: f32,
    pub remaining: f32,
}

impl ActiveCC {
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            return 0.0;
        }
        (self.remaining / self.duration).clamp(0.0, 1.0)
    }

    pub fn duration_text(&self) -> String {
        format!("{:.1}s", self.remaining.max(0.0))
    }

    pub fn is_expired(&self) -> bool {
        self.remaining <= 0.0
    }
}

impl LossOfControlData {
    pub fn apply(&mut self, cc: ActiveCC) {
        self.active = Some(cc);
    }

    pub fn tick(&mut self, dt: f32) {
        if let Some(cc) = &mut self.active {
            cc.remaining = (cc.remaining - dt).max(0.0);
        }
    }

    pub fn clear_expired(&mut self) {
        if self.active.as_ref().is_some_and(|cc| cc.is_expired()) {
            self.active = None;
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stun() -> ActiveCC {
        ActiveCC {
            cc_type: CCType::Stun,
            ability_name: "Hammer of Justice".into(),
            icon_fdid: 12345,
            duration: 6.0,
            remaining: 6.0,
        }
    }

    #[test]
    fn cc_type_labels() {
        assert_eq!(CCType::Stun.label(), "Stunned");
        assert_eq!(CCType::Fear.label(), "Feared");
        assert_eq!(CCType::Silence.label(), "Silenced");
        assert_eq!(CCType::None.label(), "");
    }

    #[test]
    fn cc_prevents_flags() {
        assert!(CCType::Stun.prevents_movement());
        assert!(CCType::Stun.prevents_casting());
        assert!(CCType::Root.prevents_movement());
        assert!(!CCType::Root.prevents_casting());
        assert!(!CCType::Silence.prevents_movement());
        assert!(CCType::Silence.prevents_casting());
    }

    #[test]
    fn active_cc_progress() {
        let mut cc = stun();
        assert!((cc.progress() - 1.0).abs() < 0.01);
        cc.remaining = 3.0;
        assert!((cc.progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn tick_and_expire() {
        let mut data = LossOfControlData::default();
        data.apply(stun());
        assert!(data.is_active());
        data.tick(4.0);
        assert!((data.active.as_ref().unwrap().remaining - 2.0).abs() < 0.01);
        data.tick(3.0);
        data.clear_expired();
        assert!(!data.is_active());
    }

    #[test]
    fn duration_text_format() {
        let cc = ActiveCC {
            remaining: 3.7,
            ..stun()
        };
        assert_eq!(cc.duration_text(), "3.7s");
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::ICON_STUN, 0);
        assert_ne!(textures::ICON_ROOT, 0);
        assert_ne!(textures::ICON_FEAR, 0);
        assert_ne!(textures::BAR_FILL, 0);
    }

    // --- CC type duration tracking ---

    fn make_cc(cc_type: CCType, duration: f32) -> ActiveCC {
        ActiveCC {
            cc_type,
            ability_name: cc_type.label().into(),
            icon_fdid: 0,
            duration,
            remaining: duration,
        }
    }

    #[test]
    fn different_cc_types_track_independently() {
        let mut data = LossOfControlData::default();
        data.apply(make_cc(CCType::Fear, 8.0));
        assert_eq!(data.active.as_ref().unwrap().cc_type, CCType::Fear);
        data.tick(3.0);
        assert!((data.active.as_ref().unwrap().remaining - 5.0).abs() < 0.01);

        // Replace with a different CC
        data.apply(make_cc(CCType::Root, 4.0));
        assert_eq!(data.active.as_ref().unwrap().cc_type, CCType::Root);
        assert!((data.active.as_ref().unwrap().remaining - 4.0).abs() < 0.01);
    }

    #[test]
    fn progress_zero_duration() {
        let cc = ActiveCC {
            duration: 0.0,
            remaining: 0.0,
            ..stun()
        };
        assert_eq!(cc.progress(), 0.0);
    }

    #[test]
    fn progress_expired() {
        let cc = ActiveCC {
            remaining: 0.0,
            ..stun()
        };
        assert_eq!(cc.progress(), 0.0);
        assert!(cc.is_expired());
    }

    #[test]
    fn tick_on_empty_state() {
        let mut data = LossOfControlData::default();
        data.tick(5.0); // should not panic
        assert!(!data.is_active());
    }

    #[test]
    fn clear_expired_on_empty_state() {
        let mut data = LossOfControlData::default();
        data.clear_expired(); // should not panic
    }

    #[test]
    fn duration_text_at_zero() {
        let cc = ActiveCC {
            remaining: 0.0,
            ..stun()
        };
        assert_eq!(cc.duration_text(), "0.0s");
    }

    #[test]
    fn all_cc_types_have_labels() {
        let types = [
            CCType::Stun,
            CCType::Fear,
            CCType::Incapacitate,
            CCType::Disorient,
            CCType::Root,
            CCType::Silence,
            CCType::Polymorph,
        ];
        for t in types {
            assert!(!t.label().is_empty(), "{:?} has empty label", t);
        }
    }

    #[test]
    fn polymorph_prevents_both() {
        assert!(CCType::Polymorph.prevents_movement());
        assert!(CCType::Polymorph.prevents_casting());
    }

    #[test]
    fn none_prevents_nothing() {
        assert!(!CCType::None.prevents_movement());
        assert!(!CCType::None.prevents_casting());
    }
}
