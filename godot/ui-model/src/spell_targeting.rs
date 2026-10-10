//! Retail SpellIsTargeting / SpellTargetingCancel / click-to-place state.
use shared::protocol::SpellCastIntent;

#[derive(Default)]
pub struct GroundTarget {
    intent: Option<SpellCastIntent>,
}

impl GroundTarget {
    pub fn begin(&mut self, intent: SpellCastIntent) {
        self.intent = Some(intent);
    }
    pub fn active(&self) -> bool {
        self.intent.is_some()
    }
    pub fn cancel(&mut self) {
        self.intent = None;
    }
    pub fn place(&mut self, destination: [f32; 3]) -> Option<SpellCastIntent> {
        if destination.iter().any(|value| !value.is_finite()) {
            return None;
        }
        let mut intent = self.intent.take()?;
        intent.destination = Some(destination);
        Some(intent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn infernal_strike() -> SpellCastIntent {
        SpellCastIntent {
            spell_id: Some(189110),
            spell: "Infernal Strike".into(),
            target_entity: None,
            witness: None,
            destination: None,
        }
    }
    #[test]
    fn ground_target_intent_contains_clicked_destination_and_exits_targeting() {
        let mut ground = GroundTarget::default();
        ground.begin(infernal_strike());
        assert!(ground.active());
        let cast = ground.place([-8910.0, 93.2, 135.0]).unwrap();
        assert_eq!(cast.spell_id, Some(189110));
        assert_eq!(cast.destination, Some([-8910.0, 93.2, 135.0]));
        assert!(!ground.active());
        assert!(ground.place([0.0; 3]).is_none());
    }
    #[test]
    fn ground_cancel_never_sends_an_intent() {
        let mut ground = GroundTarget::default();
        ground.begin(infernal_strike());
        ground.cancel();
        assert!(!ground.active());
        assert!(ground.place([0.0; 3]).is_none());
    }
    #[test]
    fn nonfinite_ground_point_keeps_targeting_and_does_not_send() {
        let mut ground = GroundTarget::default();
        ground.begin(infernal_strike());
        assert!(ground.place([f32::NAN, 1.0, 2.0]).is_none());
        assert!(ground.active());
    }
}
