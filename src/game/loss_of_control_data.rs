use bevy::prelude::*;

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
}
