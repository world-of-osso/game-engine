//! Candidate grid for short-range static scenery. Animated and long-range placements
//! remain in the per-frame set so their clocks and large visibility ranges are unchanged.

use std::collections::HashMap;

use glam::Vec3;

use super::scenery::{SceneryDistance, box_in_frustum, boxes_overlap};
use crate::wmo::portals::HalfSpace;

/// A class's cell is half its far radius, so a cull scans about twice the area that can
/// be drawn rather than whole 100-yard cells around a 30-yard radius.
const CELLS_PER_RADIUS: f32 = 2.0;
const WORD_BITS: usize = u64::BITS as usize;

struct Cell {
    min: Vec3,
    max: Vec3,
    indices: Vec<usize>,
}

#[derive(Default)]
pub(super) struct SceneryIndex {
    cells: HashMap<(i32, i32, usize), Cell>,
    admission: Vec<Option<bool>>,
    cached_windows: Option<Vec<Vec<HalfSpace>>>,
    cached_interiors: Vec<(Vec3, Vec3)>,
    admission_current: bool,
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
        self.admission.push(None);
        self.admission_current = false;
        if let Some((center, class)) = scenery.grid_position().filter(|_| !animated) {
            let (min, max) = scenery.world_bounds();
            let cell = self
                .cells
                .entry((cell(center.x, class), cell(center.z, class), class))
                .or_insert_with(|| Cell {
                    min,
                    max,
                    indices: Vec::new(),
                });
            cell.min = cell.min.min(min);
            cell.max = cell.max.max(max);
            cell.indices.push(index);
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
            for x in cell(camera.x - radius, class)..=cell(camera.x + radius, class) {
                for z in cell(camera.z - radius, class)..=cell(camera.z + radius, class) {
                    if let Some(cell) = self.cells.get(&(x, z, class)) {
                        for &index in &cell.indices {
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

    /// Classifies static cells against the same portal windows and interior boxes as
    /// each placement. Boundary cells keep the original per-placement test.
    pub fn update_admission(
        &mut self,
        windows: Option<&[Vec<HalfSpace>]>,
        interiors: &[(Vec3, Vec3)],
    ) {
        if self.admission_current
            && self.cached_windows.as_deref() == windows
            && self.cached_interiors.as_slice() == interiors
        {
            return;
        }
        for cell in self.cells.values() {
            let result = classify_cell(cell.min, cell.max, windows, interiors);
            for &index in &cell.indices {
                self.admission[index] = result;
            }
        }
        self.cached_windows = windows.map(<[Vec<HalfSpace>]>::to_vec);
        self.cached_interiors.clear();
        self.cached_interiors.extend_from_slice(interiors);
        self.admission_current = true;
    }

    pub fn update_with_admission(&mut self, camera: Vec3) -> (&[usize], &[Option<bool>]) {
        self.update(camera);
        (&self.work, &self.admission)
    }
}

fn classify_cell(
    min: Vec3,
    max: Vec3,
    windows: Option<&[Vec<HalfSpace>]>,
    interiors: &[(Vec3, Vec3)],
) -> Option<bool> {
    let Some(windows) = windows else {
        return Some(true);
    };
    if interiors
        .iter()
        .any(|&(a, b)| min.cmpge(a).all() && max.cmple(b).all())
        || windows.iter().any(|window| {
            window.iter().all(|plane| {
                let corner = Vec3::select(plane.normal.cmpge(Vec3::ZERO), min, max);
                plane.normal.dot(corner) + plane.d >= 0.0
            })
        })
    {
        return Some(true);
    }
    if interiors
        .iter()
        .any(|&(a, b)| boxes_overlap(min, max, a, b))
        || windows
            .iter()
            .any(|window| box_in_frustum(min, max, window))
    {
        None
    } else {
        Some(false)
    }
}

fn cell(position: f32, class: usize) -> i32 {
    let yards = SceneryDistance::grid_radii()[class] / CELLS_PER_RADIUS;
    (position / yards).floor() as i32
}

fn set_bit(words: &mut [u64], index: usize) {
    words[index / WORD_BITS] |= 1 << (index % WORD_BITS);
}

#[cfg(test)]
mod tests {
    use glam::{Affine3A, Vec3};

    use super::SceneryIndex;
    use crate::terrain::scenery::{SceneryDistance, outdoor_box_admitted};
    use crate::wmo::portals::HalfSpace;

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
            for &i in index.update(camera) {
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
    fn cell_admission_matches_individual_boxes_when_inputs_change() {
        let mut placements = vec![
            placement(Vec3::new(-250.0, 0.0, 0.0), 2.0),
            placement(Vec3::new(-10.0, 0.0, 0.0), 2.0),
            placement(Vec3::new(10.0, 0.0, 0.0), 2.0),
            placement(Vec3::new(70.0, 0.0, 0.0), 2.0),
            placement(Vec3::new(250.0, 0.0, 0.0), 2.0),
        ];
        let mut index = SceneryIndex::default();
        for scenery in &placements {
            index.push(*scenery, false);
        }
        let window = |normal, d| vec![vec![HalfSpace { normal, d }]];
        let windows = [window(Vec3::X, 0.0), window(Vec3::NEG_X, 50.0), vec![]];
        let interior = [(Vec3::new(-10.5, -5.0, -5.0), Vec3::new(-9.5, 5.0, 5.0))];
        for (window, interiors) in [
            (Some(windows[0].as_slice()), &interior[..]),
            (Some(windows[0].as_slice()), &interior[..]),
            (Some(windows[1].as_slice()), &interior[..]),
            (Some(windows[2].as_slice()), &[][..]),
            (None, &[][..]),
        ] {
            index.update_admission(window, interiors);
            let (_, admission) = index.update_with_admission(Vec3::ZERO);
            for (scenery, result) in placements.iter().zip(admission) {
                let (min, max) = scenery.world_bounds();
                let expected = outdoor_box_admitted(min, max, window, interiors);
                assert_eq!(result.unwrap_or(expected), expected);
            }
        }
        index.update_admission(Some(&windows[0]), &interior);
        let (_, admission) = index.update_with_admission(Vec3::ZERO);
        assert_eq!(admission[0], Some(false));
        assert_eq!(admission[1], None);
        assert_eq!(admission[2], Some(true));
        assert_eq!(admission[4], Some(true));

        // A new placement must invalidate a previously classified cell.
        index.update_admission(Some(&windows[0]), &[]);
        placements.push(placement(Vec3::new(1.0, 0.0, 0.0), 4.0));
        index.push(*placements.last().unwrap(), false);
        index.update_admission(Some(&windows[0]), &[]);
        let (_, admission) = index.update_with_admission(Vec3::ZERO);
        let scenery = placements.last().unwrap();
        let (min, max) = scenery.world_bounds();
        let expected = outdoor_box_admitted(min, max, Some(&windows[0]), &[]);
        assert_eq!(*admission.last().unwrap(), None);
        assert_eq!(admission.last().unwrap().unwrap_or(expected), expected);
    }

    #[test]
    fn newly_spawned_far_placements_become_hidden() {
        let placements = [
            placement(Vec3::new(1000.0, 0.0, 0.0), 0.5),
            placement(Vec3::new(-1000.0, 0.0, 0.0), 0.5),
        ];
        let mut index = SceneryIndex::default();
        let mut visible = [true, true];
        index.push(placements[0], false);
        for &i in index.update(Vec3::ZERO) {
            visible[i] = placements[i].visible_from(Vec3::ZERO);
        }
        assert!(!visible[0]);
        index.push(placements[1], false);
        for &i in index.update(Vec3::ZERO) {
            visible[i] = placements[i].visible_from(Vec3::ZERO);
        }
        assert_eq!(visible, [false, false]);
    }

    /// Small placements packed across 15-yard cell edges stay exactly the reference
    /// visible set while the camera walks over them.
    #[test]
    fn dense_small_placements_match_reference_across_cell_edges() {
        let placements: Vec<_> = (0..400)
            .map(|i| {
                let center = Vec3::new(
                    (i % 20) as f32 * 7.4 - 74.0,
                    0.0,
                    (i / 20) as f32 * 7.4 - 74.0,
                );
                placement(center, 0.5)
            })
            .collect();
        let mut index = SceneryIndex::default();
        for scenery in &placements {
            index.push(*scenery, false);
        }
        let mut visible = vec![true; placements.len()];
        for step in 0..40 {
            let camera = Vec3::new(step as f32 * 3.7 - 74.0, 1.0, 29.9 - step as f32 * 2.2);
            for &i in index.update(camera) {
                visible[i] = placements[i].visible_from(camera);
            }
            let reference: Vec<_> = placements
                .iter()
                .map(|item| item.visible_from(camera))
                .collect();
            assert_eq!(visible, reference, "camera {camera:?}");
        }
    }

    #[test]
    fn many_placements_match_reference_when_the_camera_moves() {
        let placements: Vec<_> = (0..130)
            .map(|i| {
                let center = Vec3::new(
                    (i % 13) as f32 * 65.0 - 390.0,
                    0.0,
                    (i / 13) as f32 * 65.0 - 260.0,
                );
                placement(center, [0.5, 2.0, 8.0][i % 3])
            })
            .collect();
        let mut index = SceneryIndex::default();
        for (i, scenery) in placements.iter().copied().enumerate() {
            index.push(scenery, i % 17 == 0);
        }
        let mut visible = vec![true; placements.len()];
        for camera in [
            Vec3::ZERO,
            Vec3::new(200.0, 0.0, -100.0),
            Vec3::new(-300.0, 0.0, 200.0),
        ] {
            for &i in index.update(camera) {
                visible[i] = placements[i].visible_from(camera);
            }
            let reference: Vec<_> = placements
                .iter()
                .map(|item| item.visible_from(camera))
                .collect();
            assert_eq!(visible, reference, "camera {camera:?}");
        }
    }
}
