//! Host wiring of combat animations and spell visuals: server combat traffic to
//! `WorldUnits` clips (`world_combat`), `SpellEffects` kits and melee sounds, per-frame
//! cast holds, kit lifetimes, missiles and particles, and automation state.

use std::time::Instant;

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
            CombatMessage::Event(event) => {
                let voiced = self.spell_effects.observe_melee(&event, &self.world);
                self.world.apply_combat_event(&event)?;
                Ok(voiced?)
            }
            CombatMessage::SpellGo(go) => {
                self.sync_spell_visual_specialization();
                if self.world.local_player_id() == Some(go.caster) {
                    self.player_movement.skyriding_spell(go.spell_id);
                }
                self.nameplates.casts.spell_go(go.caster, go.spell_id);
                self.cast_bars.spell_go(go.caster, go.spell_id);
                self.auto_attack_post_cast(&go)?;
                let started = Instant::now();
                let shown = self
                    .spell_effects
                    .spell_go(&go, &self.replica, &mut self.world);
                self.spell_effects.add_busy(started.elapsed());
                Ok(shown?)
            }
            CombatMessage::SpellFailure(failure) => {
                let interrupter = failure
                    .failed_by
                    .and_then(|unit| self.cast_interrupter(unit));
                for casts in [&mut self.nameplates.casts, &mut self.cast_bars] {
                    casts.spell_failure(
                        failure.caster,
                        failure.spell_id,
                        failure.reason,
                        interrupter.clone(),
                    );
                }
                Ok(())
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

    fn sync_spell_visual_specialization(&mut self) {
        let local_spec = self.world.local_player_id().zip(self.account.spells.spec());
        self.spell_effects.set_local_specialization(local_spec);
    }

    pub(super) fn update_spell_visuals(&mut self, delta: f32) -> Result<(), String> {
        self.sync_spell_visual_specialization();
        let started = Instant::now();
        let prefetched = match self.world.local_player_id() {
            Some(local) => self.spell_effects.prefetch(
                local,
                self.account.spells.known(),
                &self.replica,
                &self.world,
            ),
            None => Ok(()),
        };
        let held = self
            .spell_effects
            .sync_casts(&self.replica, &mut self.world);
        let camera = self.world_camera.transform();
        let sound = &self.client_options.sound;
        let gain = if sound.muted {
            0.0
        } else {
            sound.master_volume * sound.effects_volume
        };
        let advanced = self
            .spell_effects
            .advance(delta, camera, gain, &mut self.world);
        self.spell_effects.end_frame(started.elapsed());
        prefetched.and(held).and(advanced)
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
            entry.set("released_at", flight.released_at);
            entry.set("distance", flight.distance);
            entry.set("speed", flight.speed);
            entry.set("flight_time", flight.flight_time.map_or(-1.0, f64::from));
            flights.push(&entry.to_variant());
        }
        state.set("flights", &flights);
        let mut sounds = VarArray::new();
        for start in self.spell_effects.sound_starts() {
            let mut entry = VarDictionary::new();
            entry.set("spell", i64::from(start.spell_id));
            entry.set("kit", i64::from(start.kit_id));
            entry.set("unit", start.unit as i64);
            entry.set("sound_kit", i64::from(start.sound_kit_id));
            entry.set("fdid", i64::from(start.fdid));
            entry.set("looping", start.looping);
            entry.set("source", start.source.name());
            entry.set("at", start.at);
            entry.set("late", start.late);
            entry.set("stopped_at", start.stopped_at.map_or(-1.0, f64::from));
            sounds.push(&entry.to_variant());
        }
        state.set("sounds", &sounds);
        let mut melee = VarArray::new();
        for seen in self.spell_effects.melee_seen() {
            let mut entry = VarDictionary::new();
            entry.set("attacker", seen.attacker as i64);
            entry.set("target", seen.target as i64);
            entry.set("result", format!("{:?}", seen.result));
            entry.set("at", seen.at);
            melee.push(&entry.to_variant());
        }
        state.set("melee", &melee);
        state.set("clock", self.spell_effects.clock());
        state.set("assets_pending", self.spell_effects.assets_pending() as i64);
        state.set("frame_ms", self.spell_effects.frame_ms());
        let mut casts = VarArray::new();
        for seen in self.spell_effects.casts_seen() {
            let mut entry = VarDictionary::new();
            entry.set("spell", i64::from(seen.spell_id));
            entry.set("unit", seen.unit as i64);
            entry.set("go", seen.go);
            entry.set("at", seen.at);
            entry.set("wall_ms", seen.wall_ms as i64);
            entry.set("elapsed", seen.elapsed);
            entry.set("duration", seen.duration);
            casts.push(&entry.to_variant());
        }
        state.set("casts", &casts);
        state
    }
}
