//! WMOs placed by several ADT tiles. Every tile a WMO overlaps lists it in its MODF,
//! under the same uniqueId; the WMO is spawned once and lives while any of those
//! tiles is loaded.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::asset::adt_format::adt_obj::WmoPlacement;

type TileKey = (u32, u32);

#[derive(Default)]
pub(crate) struct SharedWmos {
    by_unique_id: HashMap<u32, SharedWmo>,
}

struct SharedWmo {
    entity: Entity,
    tiles: HashSet<TileKey>,
}

impl SharedWmos {
    /// MODF uniqueIds already spawned by another loaded tile.
    pub(crate) fn spawned_unique_ids(&self) -> HashSet<u32> {
        self.by_unique_id.keys().copied().collect()
    }

    /// Records `tile`'s MODF placements: `spawned` are the WMOs it just spawned,
    /// the rest of `placements` may refer to WMOs spawned by other tiles.
    pub(crate) fn record_tile(
        &mut self,
        tile: TileKey,
        placements: &[WmoPlacement],
        spawned: impl IntoIterator<Item = (u32, Entity)>,
    ) {
        for (unique_id, entity) in spawned {
            self.by_unique_id.insert(
                unique_id,
                SharedWmo {
                    entity,
                    tiles: HashSet::new(),
                },
            );
        }
        for placement in placements {
            if let Some(wmo) = self.by_unique_id.get_mut(&placement.unique_id) {
                wmo.tiles.insert(tile);
            }
        }
    }

    /// Drops `tile`'s references; returns the WMOs no loaded tile places anymore.
    pub(crate) fn release_tile(&mut self, tile: TileKey) -> Vec<Entity> {
        let mut unplaced = Vec::new();
        self.by_unique_id.retain(|_, wmo| {
            wmo.tiles.remove(&tile);
            let placed = !wmo.tiles.is_empty();
            if !placed {
                unplaced.push(wmo.entity);
            }
            placed
        });
        unplaced
    }

    pub(crate) fn clear(&mut self) {
        self.by_unique_id.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement(unique_id: u32) -> WmoPlacement {
        WmoPlacement {
            name_id: 0,
            unique_id,
            position: [0.0; 3],
            rotation: [0.0; 3],
            extents_min: [0.0; 3],
            extents_max: [0.0; 3],
            flags: 0,
            doodad_set: 0,
            name_set: 0,
            scale: 1.0,
            fdid: None,
            path: None,
        }
    }

    /// sw_harbordistrict is listed by four Stormwind tiles: the first tile spawns it,
    /// the others reuse it, and it despawns only when the last of them unloads.
    #[test]
    fn wmo_listed_by_several_tiles_is_spawned_once_and_kept_until_the_last_unloads() {
        let mut shared = SharedWmos::default();
        let harbor = Entity::from_raw_u32(7).unwrap();
        let tiles = [(30, 48), (30, 49), (31, 48), (31, 49)];

        shared.record_tile(tiles[0], &[placement(42)], [(42, harbor)]);
        for tile in &tiles[1..] {
            assert!(shared.spawned_unique_ids().contains(&42));
            shared.record_tile(*tile, &[placement(42)], []);
        }

        for tile in &tiles[..3] {
            assert!(shared.release_tile(*tile).is_empty());
            assert!(shared.spawned_unique_ids().contains(&42));
        }
        assert_eq!(shared.release_tile(tiles[3]), vec![harbor]);
        assert!(shared.spawned_unique_ids().is_empty());
    }
}
