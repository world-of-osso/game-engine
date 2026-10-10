use super::*;

/// The local caster's spec from its own spell state, other players' from their replicated
/// `ActiveSpec` (TrinityCore `UF::PlayerData::CurrentSpecID`).
fn caster_spec_order_index(
    caster: u64,
    local_spec: Option<(u64, u32)>,
    units: &Replica,
    catalog: &SpellVisualCatalog,
) -> Option<u8> {
    let spec_id = match local_spec {
        Some((local, spec_id)) if local == caster => spec_id,
        _ => units.unit(caster)?.get::<ActiveSpec>()?.0,
    };
    catalog.specialization_order_index(spec_id)
}

#[cfg(test)]
mod specialization_tests {
    use super::*;
    use std::path::Path;
    use std::sync::OnceLock;

    fn catalog() -> &'static SpellVisualCatalog {
        static CATALOG: OnceLock<SpellVisualCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let db2 = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
            SpellVisualCatalog::build(&db2).expect("local spell visual exports")
        })
    }

    fn slam_cast_kit(caster: u64, local_spec: Option<(u64, u32)>, units: &Replica) -> u32 {
        let context = CasterContext {
            race: 1,
            class: 1,
            gender: 0,
            level: 80,
            spec_order_index: caster_spec_order_index(caster, local_spec, units, catalog()),
            main_hand_subclass: Some(8), // Item.SubclassID: two-handed sword.
            auras: Vec::new(),
        };
        let visual = catalog().visual_for_spell(1464, &context).unwrap();
        catalog().kits(visual, VisualEvent::Cast)[0].kit_id
    }

    #[test]
    fn specialization_local_slam_changes_from_arms_to_fury_with_the_same_weapon() {
        // ChrSpecialization 71 (Arms): OrderIndex 0; 72 (Fury): OrderIndex 1.
        let mut spells = crate::player_spells::PlayerSpells::default();
        assert_eq!(
            slam_cast_kit(
                42,
                spells.spec().map(|spec| (42, spec)),
                &Replica::default()
            ),
            62428
        );
        spells.set_spec(71);
        assert_eq!(
            slam_cast_kit(
                42,
                spells.spec().map(|spec| (42, spec)),
                &Replica::default()
            ),
            62428
        );
        spells.set_spec(72);
        assert_eq!(
            slam_cast_kit(
                42,
                spells.spec().map(|spec| (42, spec)),
                &Replica::default()
            ),
            128672
        );
    }

    #[test]
    fn specialization_before_the_snapshot_and_unknown_remote_specs_keep_no_spec_behavior() {
        // No primary specialization: TrinityCore skips the spec comparison.
        let units = Replica::for_tests();
        assert_eq!(slam_cast_kit(42, None, &units), 62428);
        assert_eq!(slam_cast_kit(99, Some((42, 72)), &units), 62428);
        let none = caster_spec_order_index(99, Some((42, 72)), &units, catalog());
        assert_eq!(none, None);
    }

    #[test]
    fn specialization_remote_slam_follows_the_casters_replicated_spec() {
        // The local player is Arms (71); remote caster 99 replicates Fury (72), then Arms.
        let mut units = Replica::for_tests();
        units.insert(99, shared::components::ActiveSpec(72));
        assert_eq!(slam_cast_kit(99, Some((42, 71)), &units), 128672);
        assert_eq!(slam_cast_kit(99, None, &units), 128672);
        units.insert(99, shared::components::ActiveSpec(71));
        assert_eq!(slam_cast_kit(99, Some((42, 72)), &units), 62428);
    }
}

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
        spec_order_index: Option<u8>,
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
            spec_order_index,
            main_hand_subclass: world.unit_main_hand_subclass(caster),
            auras: unit
                .and_then(|unit| unit.get::<shared::components::UnitAuras>())
                .map(|auras| {
                    auras
                        .auras
                        .iter()
                        .map(|aura| (aura.spell_id, u32::from(aura.stacks)))
                        .collect()
                })
                .unwrap_or_default(),
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
        let local_spec = self.local_specialization;
        let Some(catalog) = self.catalog()? else {
            return Ok(None);
        };
        let spec_order_index = caster_spec_order_index(caster, local_spec, units, catalog);
        let context = Self::caster_context(units, world, caster, spec_order_index);
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
        // Race, class, specialization and weapon pick the same visuals as casts.
        let replicated = units.unit(caster).is_some_and(|unit| unit.has::<Player>());
        if !replicated || spells.iter().all(|spell| self.prefetched.contains(spell)) {
            return Ok(());
        }
        let Some(catalog) = poll_catalog(&mut self.catalog)? else {
            return Ok(());
        };
        let spec_order_index =
            caster_spec_order_index(caster, self.local_specialization, units, catalog);
        let context = Self::caster_context(units, world, caster, spec_order_index);
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
