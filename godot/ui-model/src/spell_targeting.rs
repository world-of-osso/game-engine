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

/// Retail item-target spell cursor. Bag clicks carry GUIDs, never unit IDs.
#[derive(Default)]
pub struct ItemTarget {
    pending: Option<(SpellCastIntent, u32)>,
}

impl ItemTarget {
    pub fn begin(&mut self, intent: SpellCastIntent, icon_fdid: u32) {
        self.pending = Some((intent, icon_fdid));
    }
    pub fn active(&self) -> bool {
        self.pending.is_some()
    }
    pub fn icon_fdid(&self) -> Option<u32> {
        self.pending.as_ref().map(|(_, icon)| *icon)
    }
    pub fn cancel(&mut self) {
        self.pending = None;
    }
    pub fn choose(&mut self, item_guid: u64) -> Option<SpellCastIntent> {
        if item_guid == 0 {
            return None;
        }
        let (mut intent, _) = self.pending.take()?;
        intent.target_item_guid = Some(item_guid);
        Some(intent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn infernal_strike() -> SpellCastIntent {
        SpellCastIntent {
            target_item_guid: None,
            spell_id: Some(189110),
            spell: "Infernal Strike".into(),
            target_entity: None,
            witness: None,
            destination: None,
        }
    }
    #[test]
    fn disenchant_item_cursor_waits_for_bag_guid_then_exits_targeting() {
        let mut target = ItemTarget::default();
        target.begin(
            SpellCastIntent {
                spell_id: Some(13262),
                spell: "Disenchant".into(),
                target_entity: None,
                target_item_guid: None,
                destination: None,
                witness: None,
            },
            136244,
        );
        assert!(target.active());
        assert_eq!(target.icon_fdid(), Some(136244));
        assert!(target.choose(0).is_none());
        assert!(target.active());
        let cast = target.choose(0x1234_5678_9abc_def0).unwrap();
        assert_eq!(cast.spell_id, Some(13262));
        assert_eq!(cast.target_item_guid, Some(0x1234_5678_9abc_def0));
        assert_eq!(cast.target_entity, None);
        assert!(!target.active());
        assert_eq!(target.icon_fdid(), None);
        assert!(target.choose(17).is_none());
    }

    #[test]
    fn disenchant_item_cursor_cancel_never_sends_a_cast() {
        let mut target = ItemTarget::default();
        target.begin(
            SpellCastIntent {
                spell_id: Some(13262),
                spell: "Disenchant".into(),
                target_entity: None,
                target_item_guid: None,
                destination: None,
                witness: None,
            },
            136244,
        );
        target.cancel();
        assert!(!target.active());
        assert!(target.choose(27).is_none());
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
