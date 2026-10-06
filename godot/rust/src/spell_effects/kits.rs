use super::*;

impl SpellEffects {
    /// Start `spell_id`'s kits for `event`; returns the looping clips started on the
    /// caster.
    pub(super) fn start_event(
        &mut self,
        spell_id: u32,
        event: VisualEvent,
        cast_units: CastUnits,
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<Vec<u16>, String> {
        let Some(visual) = self.visual(spell_id, cast_units.caster, units, world)? else {
            return Ok(Vec::new());
        };
        let Some(catalog) = self.catalog()? else {
            return Ok(Vec::new());
        };
        let kits = catalog.kits(visual, event);
        let looping = self.start_kits(spell_id, event, cast_units, kits, KitHold::Cast, world)?;
        Ok(looping
            .into_iter()
            .filter(|&(unit, _)| unit == cast_units.caster)
            .map(|(_, clip)| clip)
            .collect())
    }

    pub(super) fn start_kits(
        &mut self,
        spell_id: u32,
        event: VisualEvent,
        cast_units: CastUnits,
        kits: Vec<VisualKit>,
        hold: KitHold,
        world: &mut WorldUnits,
    ) -> Result<Vec<(u64, u16)>, String> {
        let caster = cast_units.caster;
        let mut looping = Vec::new();
        let mut errors = Vec::new();
        for kit in kits {
            let units: Vec<u64> = match kit.target {
                KitTarget::Caster => vec![caster],
                KitTarget::HitUnits => cast_units.hits.to_vec(),
                KitTarget::PrimaryTarget => cast_units.target.into_iter().collect(),
                KitTarget::Other(_) => Vec::new(),
            };
            for unit in units {
                let start = KitOnUnit {
                    kit: &kit,
                    unit,
                    caster,
                    spell_id,
                    event,
                    hold: (kit.end != VisualEvent::OneShot).then_some(hold),
                };
                match self.start_kit_on(start, world) {
                    Ok(clip) => looping.extend(clip.map(|clip| (unit, clip))),
                    Err(error) => errors.push(error),
                }
            }
        }
        // Loaded models without a start delay appear this frame.
        if let Err(error) = self.spawn_due(world) {
            errors.push(error);
        }
        join_errors(errors).map(|()| looping)
    }

    /// One kit on one unit: its animation, sounds, unit voice and models. Returns the
    /// looping clip it holds, if any.
    pub(super) fn start_kit_on(
        &mut self,
        start: KitOnUnit,
        world: &mut WorldUnits,
    ) -> Result<Option<u16>, String> {
        let KitOnUnit {
            kit,
            unit,
            caster,
            spell_id,
            ..
        } = start;
        let mut errors = Vec::new();
        let played = match play_kit_animation(kit, unit, world) {
            Ok(clip) => clip,
            Err(error) => {
                errors.push(error);
                None
            }
        };
        let cue = SoundCue {
            unit,
            spell_id,
            kit_id: kit.kit_id,
            hold: start.hold.map(|hold| hold.sound(caster, spell_id)),
            source: SoundSource::Kit,
        };
        errors.extend(self.play_sounds_on_unit(&kit.sounds, cue, world).err());
        errors.extend(self.play_voices(kit, cue, world).err());
        self.queue_kit_models(start);
        self.record_kit_start(start, played);
        let held_clip = played.filter(|_| kit.animation.is_some_and(|anim| anim.looping));
        join_errors(errors).map(|()| held_clip)
    }

    pub(super) fn queue_kit_models(&mut self, start: KitOnUnit) {
        let KitOnUnit {
            kit,
            unit,
            spell_id,
            ..
        } = start;
        let lifetime = start.hold.map_or(Lifetime::OneShot, KitHold::lifetime);
        for model in &kit.models {
            self.assets
                .request(SpellAsset::Model(model.model_fdid), Priority::Now);
            self.pending.push(PendingModel {
                due_at: self.clock + model.start_delay,
                unit,
                spell_id,
                kit_id: kit.kit_id,
                model: model.clone(),
                lifetime,
            });
        }
    }

    pub(super) fn record_kit_start(&mut self, start: KitOnUnit, played: Option<u16>) {
        let KitOnUnit {
            kit,
            unit,
            spell_id,
            ..
        } = start;
        self.record(KitStart {
            spell_id,
            kit_id: kit.kit_id,
            unit,
            event: start.event,
            anim: played,
            models: kit.models.iter().map(|model| model.model_fdid).collect(),
        });
    }

    /// The kit's unit-voice sounds (`CreatureSoundData`) in `cue.unit`'s own voice.
    pub(super) fn play_voices(
        &mut self,
        kit: &VisualKit,
        cue: SoundCue,
        world: &WorldUnits,
    ) -> Result<(), String> {
        let Some(&voice) = self.voices.get(&cue.unit) else {
            return Ok(());
        };
        let sounds: Vec<KitSound> = {
            let Some(catalog) = self.catalog()? else {
                return Ok(());
            };
            kit.unit_sounds
                .iter()
                .filter_map(|&sound| catalog.unit_sound(voice, sound).cloned())
                .collect()
        };
        let cue = SoundCue {
            hold: None,
            source: SoundSource::Voice,
            ..cue
        };
        self.play_sounds_on_unit(&sounds, cue, world)
    }

    fn play_sounds_on_unit(
        &mut self,
        sounds: &[KitSound],
        cue: SoundCue,
        world: &WorldUnits,
    ) -> Result<(), String> {
        let mut errors = Vec::new();
        for sound in sounds {
            errors.extend(self.play_on_unit(sound, cue, world).err());
        }
        join_errors(errors)
    }

    pub(super) fn play_on_unit(
        &mut self,
        sound: &KitSound,
        cue: SoundCue,
        world: &WorldUnits,
    ) -> Result<(), String> {
        match world.unit_node(cue.unit) {
            Some(parent) => self.play_sound(sound, parent, cue),
            None => Ok(()),
        }
    }

    pub(super) fn play_sound(
        &mut self,
        sound: &KitSound,
        parent: Gd<Node3D>,
        cue: SoundCue,
    ) -> Result<(), String> {
        let request = SoundRequest {
            parent,
            unit: cue.unit,
            spell_id: cue.spell_id,
            kit_id: cue.kit_id,
            hold: cue.hold,
            source: cue.source,
            at: self.clock,
        };
        self.sounds.start(sound, request, &mut self.assets)
    }

    pub(super) fn record(&mut self, start: KitStart) {
        if self.started.len() == STARTED_KEEP {
            self.started.remove(0);
        }
        self.started.push(start);
    }
}

fn play_kit_animation(
    kit: &VisualKit,
    unit: u64,
    world: &mut WorldUnits,
) -> Result<Option<u16>, String> {
    match kit.animation {
        Some(animation) => world.play_unit_action(
            unit,
            animation.anim_id,
            animation.looping,
            ActionPriority::Spell,
        ),
        None => Ok(None),
    }
}
