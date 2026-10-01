//! Original campsite prop and waterfall selection over the shared terrain objects.

use std::{path::PathBuf, time::Duration};

use game_engine_core::{
    adt::{DoodadPlacement, WmoPlacement},
    asset::wmo_format::fog::WmoFogBlend,
    campsite_object_data::{
        is_primary_campsite_doodad, is_supplemental_campsite_doodad, wmo_within_radius,
    },
};
use godot::{classes::Node3D, prelude::*};

use crate::{
    lighting::TerrainLight,
    terrain::{
        objects::{ObjectSelection, TerrainObjects},
        streaming::StreamedTerrain,
    },
};

const CAMPSITE_PROP_RADIUS: f32 = 75.0;
const CAMPSITE_WMO_RADIUS: f32 = 120.0;
/// Main-thread time per frame for campsite object loading.
const CAMPSITE_OBJECT_BUDGET: Duration = Duration::from_millis(12);

struct CampsiteSelection {
    primary: (u32, u32),
    focus: glam::Vec3,
}

impl ObjectSelection for CampsiteSelection {
    /// Primary campsite objects before supplemental waterfall scenery.
    fn tiles(&self, terrain: &StreamedTerrain) -> Vec<(u32, u32)> {
        if !terrain.parsed_tiles.contains_key(&self.primary) {
            return Vec::new();
        }
        std::iter::once(self.primary)
            .chain(
                terrain
                    .parsed_tiles
                    .keys()
                    .copied()
                    .filter(|tile| *tile != self.primary),
            )
            .collect()
    }

    fn doodad(&self, doodad: &DoodadPlacement, model: Option<&str>, tile: (u32, u32)) -> bool {
        if tile == self.primary {
            is_primary_campsite_doodad(
                doodad,
                model,
                tile.0,
                tile.1,
                self.focus,
                CAMPSITE_PROP_RADIUS,
            )
        } else {
            is_supplemental_campsite_doodad(model)
        }
    }

    fn wmo(&self, wmo: &WmoPlacement, tile: (u32, u32)) -> bool {
        tile == self.primary
            && wmo_within_radius(wmo, tile.0, tile.1, self.focus, CAMPSITE_WMO_RADIUS)
    }
}

pub(super) struct CampsiteObjects {
    objects: TerrainObjects,
    selection: CampsiteSelection,
}

impl CampsiteObjects {
    pub fn new(data_root: PathBuf, primary: (u32, u32), focus: Vector3) -> Self {
        Self {
            objects: TerrainObjects::new("CampsiteObjects", CAMPSITE_OBJECT_BUDGET, data_root),
            selection: CampsiteSelection {
                primary,
                focus: glam::Vec3::from_array(focus.to_array()),
            },
        }
    }

    pub fn sync(&mut self, parent: &mut Gd<Node3D>, terrain: &StreamedTerrain) {
        self.objects.sync(parent, terrain, &self.selection);
    }

    pub fn update_lighting(&mut self, light: &TerrainLight) {
        self.objects.update_lighting(light);
    }

    pub fn camera_fog(&self, camera: Vector3) -> Option<WmoFogBlend> {
        self.objects.camera_fog(camera)
    }

    pub fn reset(&mut self) {
        self.objects.reset();
    }
}
