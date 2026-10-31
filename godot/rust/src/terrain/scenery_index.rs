//! Candidate grid for short-range static scenery. Animated and long-range placements
//! remain in the per-frame set so their clocks and large visibility ranges are unchanged.

use std::collections::HashMap;

use glam::Vec3;

use super::scenery::SceneryDistance;

const CELL_YARDS: f32 = 100.0;
const WORD_BITS: usize = u64::BITS as usize;

#[derive(Default)]
pub(super) struct SceneryIndex {
    cells: HashMap<(i32, i32, usize), Vec<usize>>,
    always: Vec<u64>,
    previous: Vec<u64>,
    current: Vec<u64>,
    work: Vec<usize>,
    len: usize,
}

impl SceneryIndex {
    /// Adds a placement in the same order as the owning doodad vector.
    pub fn push(&mut self, scenery: SceneryDistance, animated: bool) {
        let index = self.len;
        self.len += 1;
        if index / WORD_BITS == self.always.len() {
            self.always.push(0);
            self.previous.push(0);
            self.current.push(0);
        }
        if let Some((center, class)) = scenery.grid_position().filter(|_| !animated) {
            self.cells
                .entry((cell(center.x), cell(center.z), class))
                .or_default()
                .push(index);
        } else {
            set_bit(&mut self.always, index);
        }
        // Newly spawned nodes start visible and must be evaluated once even when far away.
        set_bit(&mut self.previous, index);
    }

    /// Includes current candidates and prior candidates that may need to be hidden.
    pub fn update(&mut self, camera: Vec3) -> &[usize] {
        self.current.copy_from_slice(&self.always);
        for (class, radius) in SceneryDistance::grid_radii().iter().enumerate() {
            for x in cell(camera.x - radius)..=cell(camera.x + radius) {
                for z in cell(camera.z - radius)..=cell(camera.z + radius) {
                    if let Some(indices) = self.cells.get(&(x, z, class)) {
                        for &index in indices {
                            set_bit(&mut self.current, index);
                        }
                    }
                }
            }
        }
        self.work.clear();
        for (word_index, (current, previous)) in self
            .current
            .iter()
            .copied()
            .zip(&mut self.previous)
            .enumerate()
        {
            let mut work = current | *previous;
            *previous = current;
            while work != 0 {
                let bit = work.trailing_zeros() as usize;
                self.work.push(word_index * WORD_BITS + bit);
                work &= work - 1;
            }
        }
        &self.work
    }
}

fn cell(position: f32) -> i32 {
    (position / CELL_YARDS).floor() as i32
}

fn set_bit(words: &mut [u64], index: usize) {
    words[index / WORD_BITS] |= 1 << (index % WORD_BITS);
}

#[cfg(test)]
mod tests {
    use glam::{Affine3A, Vec3};

    use super::SceneryIndex;
    use crate::terrain::scenery::SceneryDistance;

    fn placement(center: Vec3, size: f32) -> SceneryDistance {
        let half = Vec3::splat(size / 2.0);
        SceneryDistance::new(-half, half, Affine3A::from_translation(center))
    }

    #[test]
    fn candidate_updates_preserve_visible_sets_across_cell_and_fade_boundaries() {
        let placements = [
            placement(Vec3::new(-201.0, 0.0, -99.0), 0.5),
            placement(Vec3::new(-100.0, 3.0, 0.0), 2.0),
            placement(Vec3::new(99.0, 0.0, 199.0), 8.0),
            placement(Vec3::new(1200.0, 0.0, 0.0), 120.0),
            placement(Vec3::new(0.0, 0.0, 0.0), 0.5),
        ];
        let mut index = SceneryIndex::default();
        for (i, scenery) in placements.iter().copied().enumerate() {
            index.push(scenery, i == 4);
        }
        // Nodes start visible before the first cull.
        let mut indexed_visible = vec![true; placements.len()];
        for camera in [
            Vec3::ZERO,
            Vec3::new(-99.99, 0.0, -99.99),
            Vec3::new(-200.0, 0.0, -99.0),
            Vec3::new(100.0, 0.0, 199.0),
            Vec3::new(1500.0, 0.0, 0.0),
            Vec3::ZERO,
        ] {
            let work = index.update(camera);
            assert!(work.windows(2).all(|pair| pair[0] < pair[1]));
            for &i in work {
                indexed_visible[i] = placements[i].visible_from(camera);
            }
            let reference: Vec<_> = placements
                .iter()
                .map(|scenery| scenery.visible_from(camera))
                .collect();
            assert_eq!(indexed_visible, reference, "camera {camera:?}");
        }
    }

    #[test]
    fn new_far_placement_is_culled_once_before_it_can_be_skipped() {
        let mut index = SceneryIndex::default();
        index.push(placement(Vec3::new(1000.0, 0.0, 0.0), 0.5), false);
        assert_eq!(index.update(Vec3::ZERO), &[0]);
        assert!(index.update(Vec3::ZERO).is_empty());
        index.push(placement(Vec3::new(-1000.0, 0.0, 0.0), 0.5), false);
        assert_eq!(index.update(Vec3::ZERO), &[1]);
    }
}
