//! Authored campsite terrain and lighting owned by the selection scene.

use std::path::PathBuf;

use game_engine_core::warband_scene_data::{
    WarbandSceneEntry, WarbandScenePlacement, read_authored_catalog,
    supplemental_terrain_tile_coords,
};
use godot::{classes::Node3D, prelude::*};

use game_engine_core::scene_snapshot::NodeProps;

use crate::{
    lighting::{TerrainLight, WorldLighting},
    scene_export::{SceneEntry, light_entry, m2_source},
    sky_model::SkyModel,
    terrain::objects::WMO_MODEL_META,
    terrain::{material::TerrainMaterials, streaming::StreamedTerrain},
    world_models::bind_visual_light,
};

use super::objects::CampsiteObjects;

/// The campsite's authored sky, `costalislandskybox.m2`.
const CAMPSITE_SKY_FDID: u32 = 525142;

pub(super) struct Background {
    pub scene: WarbandSceneEntry,
    pub placement: WarbandScenePlacement,
    terrain: StreamedTerrain,
    materials: TerrainMaterials,
    lighting: WorldLighting,
    light: Option<TerrainLight>,
    objects: CampsiteObjects,
    sky: Option<SkyModel>,
    data_root: PathBuf,
}

impl Background {
    /// Load `scene_id`, or the first authored scene when none was chosen.
    pub fn load(data_root: PathBuf, scene_id: Option<u32>) -> Result<Self, String> {
        let catalog = read_authored_catalog(&data_root)?;
        let scene = match scene_id {
            Some(id) => catalog
                .scenes
                .iter()
                .find(|scene| scene.id == id)
                .ok_or_else(|| format!("No authored Warband scene {id}"))?,
            None => catalog.scenes.first().ok_or("No authored Warband scenes")?,
        }
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
        let objects = CampsiteObjects::new(
            data_root.clone(),
            scene.tile_coords(),
            wow_position(placement.position),
        );
        let mut terrain = StreamedTerrain::new(data_root.clone());
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
            light: None,
            objects,
            sky: None,
            data_root,
        })
    }

    /// Light a newly shown character with the scene's current light.
    pub fn bind_light(&self, model: &Gd<Node3D>) {
        if let Some(light) = &self.light {
            bind_visual_light(model, Some(light));
        }
    }

    pub fn height_at(&self, position: Vector3) -> Option<f32> {
        self.terrain.height_at(position.x, position.z)
    }

    /// `camera` is the world position whose WMO interior fog applies.
    pub fn sync(
        &mut self,
        root: &mut Gd<Node3D>,
        model: Option<&Gd<Node3D>>,
        minutes: f32,
        camera: Vector3,
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
        self.sync_lighting(root, model, minutes, camera)?;
        self.materials.sync(root, &self.terrain)?;
        // Campsite tiles are fixed authored scenery: a tile that cannot render fails the
        // background exactly like a tile that cannot parse.
        if let Some(error) = self.materials.failures().values().next() {
            return Err(format!("Character background {error}"));
        }
        self.objects.sync(root, &self.terrain);
        self.sync_sky(root)
    }

    fn sync_lighting(
        &mut self,
        root: &mut Gd<Node3D>,
        model: Option<&Gd<Node3D>>,
        minutes: f32,
        camera: Vector3,
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
            self.objects.camera_fog(camera).as_ref(),
        )? {
            if let Some(model) = model {
                bind_visual_light(model, Some(&light));
            }
            self.materials.update_lighting(light.clone());
            self.objects.update_lighting(&light);
            self.light = Some(light);
        }
        Ok(())
    }

    fn sync_sky(&mut self, root: &mut Gd<Node3D>) -> Result<(), String> {
        if self.sky.is_none() {
            let path = self.data_root.join("models/skyboxes/costalislandskybox.m2");
            let sky = SkyModel::load_model(&self.data_root, &path, CAMPSITE_SKY_FDID, None)?;
            root.add_child(&sky.node);
            self.sky = Some(sky);
        }
        let mut clock = root
            .get_node_or_null("/root/M2MaterialClock")
            .ok_or("Authored sky requires the shared M2 material clock")?;
        let elapsed = clock
            .call("elapsed_time_ms", &[])
            .try_to::<f64>()
            .map_err(|error| format!("Cannot read sky material time: {error}"))?;
        self.sky
            .as_mut()
            .expect("sky loaded above")
            .sample(elapsed as u32);
        Ok(())
    }

    /// `Background`: `terrain:<map>_<y>_<x>` and its spawned doodad count, with its WMOs
    /// and skybox.
    pub fn scene_entry(&self) -> Result<SceneEntry, String> {
        let (tile_y, tile_x) = self.scene.tile_coords();
        let mut children = self
            .objects
            .wmo_nodes()
            .into_iter()
            .map(|node| {
                let model = node
                    .get_meta(WMO_MODEL_META)
                    .try_to::<GString>()
                    .map_err(|_| format!("scene export: {} has no WMO model", node.get_name()))?
                    .to_string();
                Ok(SceneEntry::new(
                    "Object",
                    Some(node),
                    NodeProps::Object {
                        kind: "WMO".into(),
                        model,
                    },
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        if let Some(sky) = &self.sky {
            children.push(SceneEntry::new(
                "Skybox",
                Some(sky.node.clone()),
                NodeProps::Object {
                    kind: "Skybox".into(),
                    model: m2_source(&sky.node)?,
                },
            ));
        }
        Ok(SceneEntry::new(
            "Background",
            None,
            NodeProps::Background {
                model: format!("terrain:{}_{tile_y}_{tile_x}", self.scene.map_name()),
                doodad_count: self.objects.doodad_count(),
            },
        )
        .with_children(children))
    }

    /// The scene's sun, once lighting placed it.
    pub fn sun_entry(&self) -> Option<SceneEntry> {
        let sun = self.lighting.sun()?;
        Some(light_entry("EnvironmentSun", &sun))
    }

    pub fn position_sky(&mut self, focus: Vector3) {
        if let Some(sky) = self.sky.as_mut() {
            sky.node.set_position(focus);
        }
    }

    pub fn clear_nodes(&mut self) {
        if let Some(sky) = self.sky.take() {
            sky.node.free();
        }
        self.objects.reset();
        self.materials.reset();
        self.lighting.reset();
    }
}

pub(super) fn wow_position(position: [f32; 3]) -> Vector3 {
    Vector3::new(position[0], position[2], -position[1])
}
