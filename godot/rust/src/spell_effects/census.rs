//! Debug-only playback of DB2 kits on a detached census caster.
use super::*;

impl SpellEffects {
    /// Return None while the client catalog or one of the kit's models is loading.
    pub(crate) fn census_prepare(
        &mut self,
        visual: u32,
        kit_id: u32,
    ) -> Result<Option<VisualKit>, String> {
        let Some(catalog) = self.catalog()? else {
            return Ok(None);
        };
        let kit = catalog
            .diagnostic_kit(visual, kit_id)
            .ok_or_else(|| format!("visual {visual} has no kit {kit_id}"))?;
        for model in &kit.models {
            self.assets
                .request(SpellAsset::Model(model.model_fdid), Priority::Now);
        }
        for model in &kit.models {
            match self.assets.model(model.model_fdid) {
                None => return Ok(None),
                Some(Err(error)) => return Err(error),
                Some(Ok(_)) => {}
            }
        }
        Ok(Some(kit))
    }

    pub(crate) fn census_start(&mut self, kit: &VisualKit, visual: InstanceId, spell: u32) {
        let lifetime = if kit.end == VisualEvent::OneShot {
            Lifetime::OneShot
        } else {
            Lifetime::UntilDetachedEnds { visual }
        };
        self.queue_owned_models(kit, EffectOwner::Detached(visual), lifetime, spell);
    }

    pub(crate) fn census_clear(&mut self, visual: InstanceId) {
        let owner = EffectOwner::Detached(visual);
        self.pending.retain(|pending| pending.owner != owner);
        self.active.retain_mut(|effect| {
            if effect.owner != owner {
                return true;
            }
            if effect.node.is_instance_valid() {
                effect.node.clone().free();
            }
            false
        });
    }

    pub(crate) fn census_advance_animations(
        &mut self,
        visual: InstanceId,
        delta: f64,
    ) -> Result<(), String> {
        for effect in &mut self.active {
            if effect.owner != EffectOwner::Detached(visual) {
                continue;
            }
            if let Some(mut player) = effect
                .node
                .try_get_node_as::<WowAnimationPlayer>("M2Animation")
            {
                player.set_process(false);
                if !player.bind_mut().advance_time_ms(delta * 1000.0) {
                    return Err(format!(
                        "model {} animation advance failed",
                        effect.model_fdid
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn census_active_models(&self, visual: InstanceId) -> Vec<u32> {
        self.active
            .iter()
            .filter(|effect| effect.owner == EffectOwner::Detached(visual))
            .map(|effect| effect.model_fdid)
            .collect()
    }

    pub(crate) fn census_particles_drawn(&self) -> usize {
        self.pools.drawn()
    }
}
