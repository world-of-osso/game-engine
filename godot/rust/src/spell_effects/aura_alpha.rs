use std::collections::{HashMap, HashSet};

use godot::{classes::MeshInstance3D, prelude::*};

use crate::world::WorldUnits;

const BASE_META: &str = "spell_aura_alpha_base";
const APPLIED_META: &str = "spell_aura_alpha_applied";

#[derive(Default)]
pub(super) struct AuraAlpha {
    modifiers: HashMap<(u64, u32, u64), f32>,
    applied_units: HashSet<u64>,
}

impl AuraAlpha {
    pub(super) fn add(&mut self, aura_unit: u64, instance: u32, target: u64, alpha: f32) {
        self.modifiers
            .entry((aura_unit, instance, target))
            .and_modify(|current| *current = current.min(alpha))
            .or_insert(alpha);
    }

    pub(super) fn remove(&mut self, aura_unit: u64, instance: u32) {
        self.modifiers
            .retain(|&(unit, id, _), _| (unit, id) != (aura_unit, instance));
    }

    pub(super) fn alpha(&self, target: u64) -> Option<f32> {
        self.modifiers
            .iter()
            .filter_map(|(&(_, _, unit), &alpha)| (unit == target).then_some(alpha))
            .reduce(f32::min)
    }

    pub(super) fn sync(&mut self, world: &WorldUnits) {
        let active: HashSet<u64> = self
            .modifiers
            .keys()
            .map(|&(_, _, target)| target)
            .collect();
        for &target in active.union(&self.applied_units) {
            if let Some(visual) = world.unit_visual(target) {
                let meshes = visual
                    .find_children_ex("*")
                    .type_("MeshInstance3D")
                    .owned(false)
                    .done();
                for mesh in meshes.iter_shared() {
                    apply_mesh_alpha(mesh.cast(), self.alpha(target));
                }
            }
        }
        self.applied_units = active;
    }
}

fn apply_mesh_alpha(mut mesh: Gd<MeshInstance3D>, alpha: Option<f32>) {
    if let Some(alpha) = alpha {
        let current = mesh.get_transparency();
        let base = if mesh.has_meta(BASE_META) {
            let last = mesh.get_meta(APPLIED_META).to::<f32>();
            if (current - last).abs() > f32::EPSILON {
                current
            } else {
                mesh.get_meta(BASE_META).to::<f32>()
            }
        } else {
            current
        };
        let transparency = 1.0 - (1.0 - base) * alpha;
        mesh.set_meta(BASE_META, &base.to_variant());
        mesh.set_meta(APPLIED_META, &transparency.to_variant());
        mesh.set_transparency(transparency);
    } else if mesh.has_meta(BASE_META) {
        let base = mesh.get_meta(BASE_META).to::<f32>();
        mesh.set_transparency(base);
        mesh.remove_meta(BASE_META);
        mesh.remove_meta(APPLIED_META);
    }
}

#[cfg(test)]
mod tests {
    use super::AuraAlpha;

    #[test]
    fn overlapping_auras_restore_the_remaining_alpha_then_the_original() {
        let mut auras = AuraAlpha::default();
        auras.add(42, 1, 42, 0.5);
        auras.add(42, 2, 42, 0.3);
        assert_eq!(auras.alpha(42), Some(0.3));
        auras.remove(42, 2);
        assert_eq!(auras.alpha(42), Some(0.5));
        auras.remove(42, 1);
        assert_eq!(auras.alpha(42), None);
    }
}
