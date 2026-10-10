//! Cast requests, failures and replicated HUD cast bars.

use crate::frame_error::{FrameError, SessionError, report_once};
use crate::nameplate_casts::{PlateCasts, casting_bar_state, player_casting_bar_state};
use crate::{GameClient, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::cast_failed_text::cast_failed_text;
use game_engine_ui_model::casting_bar_frame_component::CastingBarState;
use godot::prelude::*;
use shared::casting::CastState;
use shared::components::PowerType;
use shared::protocol::CastFailed;

impl GameClient {
    pub(crate) fn cast_spell(&mut self, spell_id: u32) -> Result<(), SessionError> {
        let spell_id = self.effective_spell(spell_id);
        if self.open_profession_spell(spell_id) {
            return Ok(());
        }
        if self.riding_mount_of(spell_id) {
            return self.account.send_cancel_mount_aura();
        }
        let name = self
            .spells
            .catalog()
            .and_then(|data| data.get(spell_id))
            .map(|spell| spell.name.to_string())
            .unwrap_or_default();
        if self
            .spells
            .catalog()
            .and_then(|data| data.get(spell_id))
            .is_some_and(|spell| spell.ground_targeted)
        {
            self.spells.ground.begin(shared::protocol::SpellCastIntent {
                spell_id: Some(spell_id),
                spell: name,
                target_entity: None,
                witness: None,
                destination: None,
            });
            return Ok(());
        }
        self.spells.cancel_ground_target();
        let target = self.targeting_target();
        if self.auto_attack_on_cast(spell_id, target)? {
            return Ok(());
        }
        let witness = self.cast_witness(target).unwrap_or_else(|error| {
            report_once(&error);
            None
        });
        self.account.send_cast(spell_id, &name, target, witness)?;
        self.spells.sent.push(spell_id);
        Ok(())
    }

    /// Whether the local player rides the mount `spell_id` summons (it is mounted and has
    /// that spell's aura): using the mount again dismounts, as Retail's mount buttons do.
    pub(super) fn riding_mount_of(&self, spell_id: u32) -> bool {
        let Some(unit) = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
        else {
            return false;
        };
        unit.has::<shared::components::Mounted>()
            && unit
                .get::<shared::components::UnitAuras>()
                .is_some_and(|auras| auras.auras.iter().any(|aura| aura.spell_id == spell_id))
    }

    /// `CastFailed`: Retail GlobalStrings text in UIErrorsFrame.
    pub(crate) fn show_cast_failed(&mut self, failed: CastFailed) -> Result<(), String> {
        let power = self
            .spells
            .catalog()
            .and_then(|data| data.get(failed.spell_id))
            .and_then(|spell| spell.powers.first())
            .and_then(|cost| PowerType::from_db(i32::from(cost.power_type)));
        let text = cast_failed_text(failed.reason, failed.detail.as_deref(), power);
        if self.professions.book.selected == Some(failed.spell_id) {
            self.professions.error = text.clone();
        }
        self.spells.errors.push(text.clone());
        self.add_world_error(&text)
    }

    pub(super) fn spell_triggers_gcd(&self, spell_id: u32) -> bool {
        self.spells
            .catalog()
            .and_then(|data| data.get(spell_id))
            .is_none_or(|spell| spell.cooldown.gcd_ms > 0)
    }

    /// The HUD cast bars' clocks this frame (`PlayerCastingBarFrame`,
    /// `TargetFrameSpellBar`): the local player's and the target's replicated casts;
    /// bars of other units are dropped.
    pub(crate) fn update_cast_bars(&mut self, delta: f32) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.cast_bars = PlateCasts::default();
            return Ok(());
        }
        let units = [self.world.local_player_id(), self.targeting_target()];
        for id in units.into_iter().flatten() {
            let cast = self
                .replica
                .unit(id)
                .and_then(|unit| unit.get::<CastState>());
            self.cast_bars.observe(id, cast);
        }
        self.cast_bars
            .advance(delta, |id| units.contains(&Some(id)));
        Ok(())
    }

    /// `unit`'s HUD cast bar with its spell art, while it has one.
    pub(crate) fn hud_cast_bar(&mut self, unit: u64) -> Option<CastingBarState> {
        let spell = self.cast_bars.get(unit)?.spell_id;
        let art = self
            .spells
            .catalog()
            .and_then(|catalog| catalog.get(spell))
            .map(|spell| spell.icon_fdid);
        let icon = art.map(|fdid| self.drawable_fdid(fdid));
        Some(casting_bar_state(self.cast_bars.get(unit)?, icon))
    }

    pub(super) fn sync_cast_bar(&mut self) -> Result<(), String> {
        let state = self
            .world
            .local_player_id()
            .and_then(|player| {
                let state = self.hud_cast_bar(player)?;
                self.cast_bars
                    .get(player)
                    .map(|bar| player_casting_bar_state(bar, state.icon_fdid))
            })
            .unwrap_or_default();
        if let Some(ui) = self.spells.cast_ui.as_mut() {
            return ui.bind_mut().set_state(state);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("CastingBarUI");
        ui.set_layer(2);
        self.base_mut().add_child(&ui);
        let shown = ui.bind_mut().show_casting_bar(state);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.spells.cast_ui = Some(ui);
        Ok(())
    }
}
