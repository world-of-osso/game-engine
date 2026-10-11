//! Server-authoritative Transmogrify pending state.
use shared::protocol::{
    ApplyTransmog, EquipmentSlot, TransmogChange, TransmogChoice, TransmogSlot, TransmogSource,
    TransmogState,
};
use std::collections::BTreeMap;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct TransmogBook {
    pub state: Option<TransmogState>,
    pub money: u64,
    pub pending: BTreeMap<EquipmentSlot, TransmogChoice>,
    pub selected: Option<EquipmentSlot>,
    pub applying: bool,
    pub error: String,
    pub page: usize,
}

impl TransmogBook {
    pub fn receive(&mut self, state: TransmogState) {
        self.selected = state.slots.first().map(|slot| slot.slot);
        self.state = Some(state);
    }
    pub fn stage(&mut self, _choice: TransmogChoice) -> bool {
        false
    }
    pub fn pending_cost(&self) -> Option<u64> {
        Some(0)
    }
    pub fn apply(&mut self) -> Option<ApplyTransmog> {
        None
    }
}

#[cfg(test)]
#[path = "transmog_tests.rs"]
mod tests;
