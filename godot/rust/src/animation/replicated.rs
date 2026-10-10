//! Project authoritative aura312 set IDs onto every loaded unit's animation controller.
use game_engine_core::animation_replacements::AnimationReplacementCatalog;
use game_engine_network::replica::Replica;
use shared::components::{AuraOverride, UnitAuras};

use super::WorldUnits;
use crate::animation::WowAnimationPlayer;

fn active_sets(auras: Option<&UnitAuras>) -> Vec<u32> {
    auras
        .into_iter()
        .flat_map(|auras| &auras.auras)
        .flat_map(|aura| &aura.overrides)
        .filter_map(|value| match value {
            AuraOverride::Animation(id) => Some(*id),
            AuraOverride::ActionBar { .. } => None,
        })
        .collect()
}

impl WorldUnits {
    /// Also runs after late visual loads: an aura may arrive before the model does.
    pub(crate) fn sync_animation_replacements(&mut self, replica: &Replica) -> Result<(), String> {
        for (&id, unit) in &self.units {
            let sets = active_sets(replica.unit(id).and_then(|unit| unit.get::<UnitAuras>()));
            let Some(visual) = &unit.visual else { continue };
            let path = if unit.is_player {
                "M2Animation"
            } else {
                "NpcModel/M2Animation"
            };
            let Some(mut animation) = visual.try_get_node_as::<WowAnimationPlayer>(path) else {
                continue;
            };
            let map = if sets.is_empty() {
                Default::default()
            } else {
                if self.animation_replacements.is_none() {
                    self.animation_replacements =
                        Some(AnimationReplacementCatalog::load(&self.data_root)?);
                }
                self.animation_replacements
                    .as_ref()
                    .expect("loaded catalog")
                    .replacements(&sets)?
            };
            animation
                .bind_mut()
                .set_replacements(map)
                .map_err(|error| format!("Unit {id} aura312: {error}"))?;
        }
        Ok(())
    }
}
