//! Authored campsite terrain and lighting owned by the selection scene.

use std::path::PathBuf;

use game_engine_core::warband_scene_data::{
    WarbandSceneEntry, WarbandScenePlacement, read_authored_catalog,
    supplemental_terrain_tile_coords,
};
use godot::{classes::Node3D, prelude::*};

use crate::{
    lighting::WorldLighting,
    terrain::{material::TerrainMaterials, streaming::StreamedTerrain},
    world_models::bind_visual_light,
};

pub(super) struct Background {
    pub scene: WarbandSceneEntry,
    pub placement: WarbandScenePlacement,
    terrain: StreamedTerrain,
    materials: TerrainMaterials,
    lighting: WorldLighting,
}

impl Background {
    pub fn load(data_root: PathBuf, cache_root: PathBuf) -> Result<Self, String> {
        let catalog = read_authored_catalog(&data_root)?;
        let scene = catalog
            .scenes
            .first()
            .ok_or("No authored Warband scenes")?
            .clone();
        let placement = catalog
            .placements
            .iter()
            .find(|slot| slot.scene_id == scene.id && slot.is_character_slot())
            .or_else(|| {
                catalog
                    .placements
                    .iter()
                    .find(|slot| slot.scene_id == scene.id)
            })
            .ok_or_else(|| format!("Warband scene {} has no placement", scene.id))?
            .clone();
        let mut terrain = StreamedTerrain::new(data_root, cache_root);
        terrain.request_map_tiles(
            scene.map_name(),
            scene.tile_coords(),
            &supplemental_terrain_tile_coords(&scene),
        )?;
        Ok(Self {
            scene,
            placement,
            terrain,
            materials: TerrainMaterials::default(),
            lighting: WorldLighting::default(),
        })
    }

    pub fn height_at(&self, position: Vector3) -> Option<f32> {
        self.terrain.height_at(position.x, position.z)
    }

    pub fn sync(
        &mut self,
        root: &mut Gd<Node3D>,
        model: &Gd<Node3D>,
        minutes: f32,
    ) -> Result<(), String> {
        self.terrain.poll()?;
        let state = self.terrain.state();
        if let Some(error) = state.map_error {
            return Err(format!("Character background map: {error}"));
        }
        if let Some(failure) = state.failures.first() {
            return Err(format!(
                "Character background tile {:?}: {}",
                failure.tile, failure.error
            ));
        }
        self.sync_lighting(root, model, minutes)?;
        self.materials.sync(root, &self.terrain)
    }

    fn sync_lighting(
        &mut self,
        root: &mut Gd<Node3D>,
        model: &Gd<Node3D>,
        minutes: f32,
    ) -> Result<(), String> {
        let Some(wdt) = self.terrain.map_wdt.as_ref() else {
            return Ok(());
        };
        if let Some(light) = self.lighting.sync(
            root,
            &wdt.lighting,
            self.scene.map_id,
            wow_position(self.scene.position),
            minutes,
        )? {
            bind_visual_light(model, Some(&light));
            self.materials.update_lighting(light);
        }
        Ok(())
    }

    pub fn clear_nodes(&mut self) {
        self.materials.reset();
        self.lighting.reset();
    }
}

pub(super) fn wow_position(position: [f32; 3]) -> Vector3 {
    Vector3::new(position[0], position[2], -position[1])
}
