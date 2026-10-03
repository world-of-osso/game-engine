//! Skyriding vigor: the Retail `FillUpFrames` widget (`dragonriding_vigor`) showing the
//! Skyriding Charges (ChargeCategory 2391) while the Skyriding bar is up, i.e. while an
//! ADV_FLYING aura lets the player skyride. Charges the server has not reported are full
//! (Retail `SpellHistory` only lists consumed charges).

use game_engine_ui_model::vigor_bar_component::{VIGOR_ART_FDIDS, VigorBarState, VigorFrames};
use godot::prelude::*;
use shared::skyriding::SURGE_FORWARD;

use crate::player_spells::{ChargeTimer, SKYRIDING_BONUS_BAR};
use crate::ui::RegistryUi;

/// `SpellCategories.ChargeCategory` of Surge Forward and Skyward Ascent.
const SKYRIDING_CHARGES: u32 = 2391;

/// The widget's frames: one per charge (`max_charges` from the spell data until the server
/// reports the category), the recovering one filling.
pub(crate) fn vigor_frames(
    skyriding: bool,
    charges: Option<ChargeTimer>,
    max_charges: u8,
) -> Option<VigorFrames> {
    if !skyriding {
        return None;
    }
    Some(match charges {
        Some(charges) => VigorFrames {
            total: charges.max,
            full: charges.current,
            filling: charges.recovered(),
        },
        None => VigorFrames {
            total: max_charges,
            full: max_charges,
            filling: 0.0,
        },
    })
}

impl crate::GameClient {
    pub(super) fn vigor_bar_state(&self) -> VigorBarState {
        let max_charges = self
            .spells
            .catalog()
            .and_then(|catalog| catalog.get(SURGE_FORWARD)?.charges)
            .map_or(0, |charges| {
                charges.max_charges.min(u32::from(u8::MAX)) as u8
            });
        VigorBarState {
            shown: vigor_frames(
                self.bonus_bar_offset() == SKYRIDING_BONUS_BAR,
                self.account.spells.charges(SKYRIDING_CHARGES),
                max_charges,
            ),
        }
    }

    pub(super) fn sync_vigor_bar(&mut self) -> Result<(), String> {
        let state = self.vigor_bar_state();
        if let Some(ui) = self.spells.vigor_ui.as_mut() {
            return ui.bind_mut().set_state(state);
        }
        if state.shown.is_none() {
            return Ok(());
        }
        self.extract_art(&VIGOR_ART_FDIDS);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("VigorBarUI");
        ui.set_layer(2);
        self.base_mut().add_child(&ui);
        let shown = ui.bind_mut().show_vigor_bar(state);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.spells.vigor_ui = Some(ui);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreported_charges_show_full_and_reported_ones_fill() {
        assert_eq!(vigor_frames(false, None, 6), None);
        assert_eq!(
            vigor_frames(true, None, 6),
            Some(VigorFrames {
                total: 6,
                full: 6,
                filling: 0.0
            })
        );
        let spent = ChargeTimer {
            current: 2,
            max: 6,
            recharge: 10.0,
            remaining: 7.5,
        };
        assert_eq!(
            vigor_frames(true, Some(spent), 6),
            Some(VigorFrames {
                total: 6,
                full: 2,
                filling: 0.25
            })
        );
    }
}
