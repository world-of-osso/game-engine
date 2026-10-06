//! Spatial loading progress. Distant entries remain tracked and spawn normally.

use std::collections::HashMap;

use glam::Vec3;

use super::objects::Pending;

/// Local entry bubble: camera (15 yards by default) plus room to move before
/// background scenery arrives. This is client policy, not a reverse-engineered
/// Retail constant.
pub(crate) const ENTRY_RADIUS: f32 = 100.0;

struct Placement {
    min: Vec3,
    max: Vec3,
    done: bool,
}

#[derive(Default)]
pub(super) struct NearbyProgress(HashMap<Pending, Placement>);

impl NearbyProgress {
    pub fn add(&mut self, key: Pending, min: Vec3, max: Vec3) {
        self.0.entry(key).or_insert(Placement {
            min: min.min(max),
            max: min.max(max),
            done: false,
        });
    }

    pub fn finish(&mut self, key: Pending) {
        if let Some(placement) = self.0.get_mut(&key) {
            placement.done = true;
        }
    }

    pub fn is_nearby(&self, key: Pending, player: Vec3) -> bool {
        self.0
            .get(&key)
            .is_some_and(|placement| placement.nearby(player))
    }

    pub fn get(&self, player: Vec3) -> (usize, usize) {
        self.0
            .values()
            .filter(|placement| placement.nearby(player))
            .fold((0, 0), |(done, total), placement| {
                (done + usize::from(placement.done), total + 1)
            })
    }
}

impl Placement {
    fn nearby(&self, player: Vec3) -> bool {
        let nearest = player.clamp(self.min, self.max);
        player.distance_squared(nearest) <= ENTRY_RADIUS * ENTRY_RADIUS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearby_gate_waits_for_discovered_wmo_children_not_distant_scenery() {
        let player = Vec3::new(-8928.0, 99.295, -606.0);
        let tree = Pending::Doodad((30, 48), 0);
        let distant = Pending::Doodad((30, 48), 1);
        let city = Pending::Wmo((30, 48), 0);
        let nearby_chair = Pending::WmoDoodad(5457395, 0);
        let distant_chair = Pending::WmoDoodad(5457395, 1);
        let mut progress = NearbyProgress::default();
        progress.add(tree, player + Vec3::X * 50.0, player + Vec3::X * 50.0);
        progress.add(distant, player + Vec3::X * 500.0, player + Vec3::X * 500.0);
        // A city root is required by its extent, not its distant origin.
        progress.add(
            city,
            player - Vec3::splat(300.0),
            player + Vec3::splat(300.0),
        );
        assert_eq!(progress.get(player), (0, 2));
        assert!(progress.is_nearby(city, player));
        assert!(!progress.is_nearby(distant, player));
        progress.finish(tree);
        assert_eq!(progress.get(player), (1, 2));
        // Production ordering: register children before the completed root can
        // release the gate, so discovery cannot create a false-ready frame.
        progress.add(nearby_chair, player, player);
        progress.add(
            distant_chair,
            player + Vec3::Z * 250.0,
            player + Vec3::Z * 250.0,
        );
        progress.finish(city);
        assert_eq!(progress.get(player), (2, 3));
        progress.finish(nearby_chair);
        assert_eq!(progress.get(player), (3, 3));
        // A distant object is still pending; it becomes required after movement.
        assert_eq!(progress.get(player + Vec3::X * 500.0), (0, 1));
        progress.finish(distant);
        assert_eq!(progress.get(player + Vec3::X * 500.0), (1, 1));
    }

    #[test]
    fn nearby_boundary_is_inclusive_and_duplicate_registration_preserves_completion() {
        let key = Pending::Doodad((30, 48), 0);
        let mut progress = NearbyProgress::default();
        progress.add(key, Vec3::X * ENTRY_RADIUS, Vec3::X * ENTRY_RADIUS);
        assert_eq!(progress.get(Vec3::ZERO), (0, 1));
        assert_eq!(progress.get(-Vec3::X * 0.01), (0, 0));
        progress.finish(key);
        progress.finish(key);
        progress.add(key, Vec3::X * ENTRY_RADIUS, Vec3::X * ENTRY_RADIUS);
        assert_eq!(progress.get(Vec3::ZERO), (1, 1));
    }
}
