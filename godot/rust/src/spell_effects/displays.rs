//! CDI state kits belong to a displayed visual, not a spell cast or aura instance.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
struct DisplayVisual {
    display: u32,
    visual: u64,
}

pub(super) struct HeldDisplay {
    owner: DisplayVisual,
    anims: Vec<u16>,
}

impl SpellEffects {
    pub(super) fn sync_display_kits(
        &mut self,
        units: &Replica,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let Some(catalog) = self.catalog()? else {
            return Ok(());
        };
        let present: HashMap<u64, DisplayVisual> = units
            .units()
            .filter_map(|unit| {
                let display = unit.get::<ModelDisplay>()?.display_id;
                catalog.display_state_kit_id(display)?;
                let visual = world.unit_model_node(unit.server_id)?;
                Some((
                    unit.server_id,
                    DisplayVisual {
                        display,
                        visual: visual.instance_id().to_i64() as u64,
                    },
                ))
            })
            .collect();
        let ended: Vec<u64> = self
            .displays
            .iter()
            .filter(|(unit, held)| present.get(unit) != Some(&held.owner))
            .map(|(&unit, _)| unit)
            .collect();
        for unit in ended {
            self.end_display_kit(unit, world);
        }
        let mut errors = Vec::new();
        for (unit, owner) in present {
            if !self.displays.contains_key(&unit) {
                errors.extend(self.begin_display_kit(unit, owner, world).err());
            }
        }
        join_errors(errors)
    }

    fn begin_display_kit(
        &mut self,
        unit: u64,
        owner: DisplayVisual,
        world: &mut WorldUnits,
    ) -> Result<(), String> {
        let kit = self
            .catalog()?
            .and_then(|catalog| catalog.display_state_kit(owner.display))
            .ok_or_else(|| format!("display {} lost its state kit", owner.display))?;
        let started = self.start_kit_on(
            KitOnUnit {
                kit: &kit,
                unit,
                caster: unit,
                spell_id: 0,
                event: VisualEvent::DisplayStart,
                hold: Some(KitHold::Display {
                    unit,
                    visual: owner.visual,
                }),
            },
            world,
        );
        let anims = started
            .as_ref()
            .ok()
            .copied()
            .flatten()
            .into_iter()
            .collect();
        self.displays.insert(unit, HeldDisplay { owner, anims });
        started.map(drop)
    }

    fn end_display_kit(&mut self, unit: u64, world: &mut WorldUnits) {
        let Some(held) = self.displays.remove(&unit) else {
            return;
        };
        for anim in held.anims {
            world.stop_unit_action(unit, anim);
        }
        let visual = held.owner.visual;
        let lifetime = Lifetime::UntilDisplayEnds { unit, visual };
        self.pending.retain(|pending| pending.lifetime != lifetime);
        self.sounds
            .end(SoundHold::Display { unit, visual }, self.clock);
        for effect in &mut self.active {
            if effect.lifetime == lifetime && matches!(effect.phase, Phase::Start(_) | Phase::Hold)
            {
                effect.finish();
            }
        }
    }
}
