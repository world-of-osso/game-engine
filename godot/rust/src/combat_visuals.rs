//! Host wiring of combat animations and spell visuals: server combat traffic to
//! `WorldUnits` clips (`world_combat`) and `SpellEffects` kits, per-frame cast holds,
//! kit lifetimes, missiles and particles, and automation state.

use godot::prelude::*;

use crate::GameClient;
use crate::account::CombatMessage;
use crate::frame_error::FrameError;

impl GameClient {
    pub(super) fn receive_combat_message(
        &mut self,
        message: CombatMessage,
    ) -> Result<(), FrameError> {
        match message {
            CombatMessage::Event(event) => Ok(self.world.apply_combat_event(&event)?),
            CombatMessage::SpellGo(go) => {
                self.auto_attack_post_cast(&go)?;
                Ok(self
                    .spell_effects
                    .spell_go(&go, &self.units, &mut self.world)?)
            }
            CombatMessage::AttackStart(start) => {
                self.receive_attack_start(&start);
                Ok(())
            }
            CombatMessage::AttackStopped(stopped) => {
                self.receive_attack_stopped(&stopped);
                Ok(())
            }
        }
    }

    pub(super) fn update_spell_visuals(&mut self, delta: f32) -> Result<(), String> {
        let held = self.spell_effects.sync_casts(&self.units, &mut self.world);
        let camera = self.world_camera.transform();
        let advanced = self.spell_effects.advance(delta, camera, &mut self.world);
        held.and(advanced)
    }

    /// Recent kit starts and missile flights, and the kit models and missiles shown now.
    pub(super) fn spell_visuals_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let mut started = VarArray::new();
        for start in self.spell_effects.started() {
            let mut entry = VarDictionary::new();
            entry.set("spell", i64::from(start.spell_id));
            entry.set("kit", i64::from(start.kit_id));
            entry.set("unit", start.unit as i64);
            entry.set("event", format!("{:?}", start.event));
            entry.set("anim", start.anim.map_or(-1, i64::from));
            let models: PackedInt64Array =
                start.models.iter().map(|&fdid| i64::from(fdid)).collect();
            entry.set("models", &models);
            started.push(&entry.to_variant());
        }
        state.set("started", &started);
        let mut active = VarArray::new();
        for (unit, spell, kit, model) in self.spell_effects.active_models() {
            let mut entry = VarDictionary::new();
            entry.set("unit", unit as i64);
            entry.set("spell", i64::from(spell));
            entry.set("kit", i64::from(kit));
            entry.set("model", i64::from(model));
            active.push(&entry.to_variant());
        }
        state.set("active", &active);
        state.set("missiles", self.spell_effects.missile_count() as i64);
        let mut flights = VarArray::new();
        for flight in self.spell_effects.flights() {
            let mut entry = VarDictionary::new();
            entry.set("spell", i64::from(flight.spell_id));
            entry.set("caster", flight.caster as i64);
            entry.set("target", flight.target as i64);
            entry.set("release_delay", flight.release_delay);
            entry.set("distance", flight.distance);
            entry.set("speed", flight.speed);
            entry.set("flight_time", flight.flight_time.map_or(-1.0, f64::from));
            flights.push(&entry.to_variant());
        }
        state.set("flights", &flights);
        state
    }
}
