use super::*;

impl SpellEffects {
    /// Start and end held precast/channel kits as units' replicated casts change.
    pub fn sync_casts(&mut self, units: &Replica, world: &mut WorldUnits) -> Result<(), String> {
        let mut errors = Vec::new();
        self.voices = units
            .units()
            .filter(|unit| is_unit(*unit))
            .filter_map(|unit| Some((unit.server_id, voice_source(unit)?)))
            .collect();
        self.forget_freed_effects();
        if self.catalog()?.is_none() {
            return Ok(());
        }
        errors.extend(self.sync_auras(units, world).err());
        self.end_missing_casts(units, world);
        for unit in units.units().filter(|unit| is_unit(*unit)) {
            errors.extend(self.start_replicated_cast(unit, units, world).err());
        }
        join_errors(errors)
    }

    pub(super) fn end_missing_casts(&mut self, units: &Replica, world: &mut WorldUnits) {
        let ended: Vec<u64> = self
            .held
            .iter()
            .filter(|(id, held)| {
                units
                    .unit(**id)
                    .and_then(|unit| unit.get::<CastState>())
                    .is_none_or(|cast| cast.spell_id != held.spell_id)
            })
            .map(|(&id, _)| id)
            .collect();
        for id in ended {
            self.end_held(id, world);
        }
    }

    pub(super) fn start_replicated_cast(
        &mut self,
        unit: Unit,
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let id = unit.server_id;
        let Some(cast) = unit.get::<CastState>() else {
            return Ok(());
        };
        if self.held.contains_key(&id) {
            return Ok(());
        }
        self.see_cast(cast.spell_id, id, false, cast.elapsed, cast.duration);
        let event = match cast.cast_type {
            CastType::Normal => VisualEvent::PrecastStart,
            CastType::Channel => VisualEvent::ChannelStart,
        };
        let started = self.start_cast_event(id, cast, event, units, world);
        let anims = started.as_ref().cloned().unwrap_or_default();
        self.held.insert(
            id,
            HeldCast {
                spell_id: cast.spell_id,
                cast_type: cast.cast_type,
                anims,
            },
        );
        started.map(drop)
    }

    pub(super) fn start_cast_event(
        &mut self,
        id: u64,
        cast: &CastState,
        event: VisualEvent,
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<Vec<u16>, String> {
        let target = (cast.target != 0).then_some(cast.target);
        let hits: Vec<u64> = target.into_iter().collect();
        let cast_units = CastUnits {
            caster: id,
            target,
            hits: &hits,
        };
        self.start_event(cast.spell_id, event, cast_units, units, world)
    }

    pub(super) fn end_held(&mut self, id: u64, world: &mut WorldUnits) {
        let Some(held) = self.held.remove(&id) else {
            return;
        };
        for anim in held.anims {
            world.stop_unit_action(id, anim);
        }
        self.pending
            .retain(|pending| !(pending.unit == id && pending.lifetime == Lifetime::UntilCastEnds));
        self.sounds.end(
            SoundHold::Cast {
                unit: id,
                spell_id: held.spell_id,
            },
            self.clock,
        );
        for effect in &mut self.active {
            if effect.owner == id
                && effect.spell_id == held.spell_id
                && effect.lifetime == Lifetime::UntilCastEnds
                && matches!(effect.phase, Phase::Start(_) | Phase::Hold)
            {
                effect.finish();
            }
        }
    }

    /// `SpellGo`: the caster's Cast kits, then its missile or Impact kits.
    pub fn spell_go(
        &mut self,
        go: &SpellGo,
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let mut errors = Vec::new();
        self.see_cast(go.spell_id, go.caster, true, 0.0, 0.0);
        // The cast resolved: its precast loop and hand effects end.
        if self
            .held
            .get(&go.caster)
            .is_some_and(|held| held.spell_id == go.spell_id && held.cast_type == CastType::Normal)
        {
            self.end_held(go.caster, world);
        }
        let cast_units = CastUnits {
            caster: go.caster,
            target: go.target,
            hits: &go.hit_targets,
        };
        if let Err(error) =
            self.start_event(go.spell_id, VisualEvent::Cast, cast_units, units, world)
        {
            errors.push(error);
        }
        errors.extend(self.resolve_go(go, cast_units, units, world).err());
        join_errors(errors)
    }

    pub(super) fn resolve_go(
        &mut self,
        go: &SpellGo,
        cast_units: CastUnits,
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        match self.ready_missile(go, units, world)? {
            Some(launch) if world.unit_awaits_missile_release(go.caster) => {
                self.ready.push(launch);
                Ok(())
            }
            Some(launch) => self.launch(launch, world),
            None => self
                .start_event(go.spell_id, VisualEvent::Impact, cast_units, units, world)
                .map(drop),
        }
    }

    pub(super) fn caster_context(
        units: &Replica,
        world: &WorldUnits,
        caster: u64,
    ) -> CasterContext {
        let unit = units.unit(caster);
        let player = unit.and_then(|unit| unit.get::<Player>());
        CasterContext {
            race: player.map_or(0, |player| player.race),
            class: player.map_or(0, |player| player.class),
            gender: player.map_or(0, |player| player.appearance.sex),
            level: unit
                .and_then(|unit| unit.get::<UnitLevel>())
                .map_or(0, |level| u32::from(level.0)),
            spec_order_index: None,
            main_hand_subclass: world.unit_main_hand_subclass(caster),
        }
    }

    /// The visual `caster` shows for `spell_id`, if any.
    pub(super) fn visual(
        &mut self,
        spell_id: u32,
        caster: u64,
        units: &Replica,
        world: &WorldUnits,
    ) -> Result<Option<u32>, String> {
        let context = Self::caster_context(units, world, caster);
        let Some(catalog) = self.catalog()? else {
            return Ok(None);
        };
        Ok(catalog.visual_for_spell(spell_id, &context))
    }

    /// Start loading, in the background, the kit models and sound files of `caster`'s
    /// `spells` not prefetched yet, once the catalog and the caster are there.
    pub fn prefetch(
        &mut self,
        caster: u64,
        spells: &[u32],
        units: &Replica,
        world: &WorldUnits,
    ) -> Result<(), String> {
        // Its race, class and weapon pick the visuals (`caster_context`).
        let replicated = units.unit(caster).is_some_and(|unit| unit.has::<Player>());
        if !replicated || spells.iter().all(|spell| self.prefetched.contains(spell)) {
            return Ok(());
        }
        let context = Self::caster_context(units, world, caster);
        let Some(catalog) = poll_catalog(&mut self.catalog)? else {
            return Ok(());
        };
        for &spell in spells {
            if !self.prefetched.insert(spell) {
                continue;
            }
            let Some(visual) = catalog.visual_for_spell(spell, &context) else {
                continue;
            };
            for asset in kit_assets(catalog, visual) {
                self.assets.request(asset, Priority::Later);
            }
        }
        Ok(())
    }
}
