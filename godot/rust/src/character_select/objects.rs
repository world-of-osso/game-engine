//! Original campsite prop and waterfall placement over native M2 resources.

use std::{collections::BTreeSet, path::PathBuf};

use game_engine_core::{
    adt::{DoodadPlacement, WmoPlacement},
    campsite_object_data::{
        campsite_doodad_placement, doodad_position, is_primary_campsite_doodad,
        is_supplemental_campsite_doodad, placement_position,
    },
};
use godot::{classes::Node3D, prelude::*};
use osso_asset_resolver::CascListfileResolver;

use crate::{
    assets::{
        build_model,
        creature::{cache_model_files, cache_model_textures, local_resolver},
        read_model,
    },
    lighting::TerrainLight,
    terrain::streaming::StreamedTerrain,
    world_models::bind_visual_light,
};

const CAMPSITE_PROP_RADIUS: f32 = 75.0;
const CAMPSITE_WMO_RADIUS: f32 = 120.0;

pub(super) struct CampsiteObjects {
    root: Option<Gd<Node3D>>,
    attached_tiles: BTreeSet<(u32, u32)>,
    attached_doodads: BTreeSet<u32>,
    attached_wmos: BTreeSet<u32>,
    resolver: CascListfileResolver,
    data_root: PathBuf,
    primary: (u32, u32),
    focus: glam::Vec3,
}

impl CampsiteObjects {
    pub fn new(
        data_root: PathBuf,
        cache_root: PathBuf,
        primary: (u32, u32),
        focus: Vector3,
    ) -> Self {
        Self {
            root: None,
            attached_tiles: BTreeSet::new(),
            attached_doodads: BTreeSet::new(),
            attached_wmos: BTreeSet::new(),
            resolver: local_resolver(&data_root, &cache_root),
            data_root,
            primary,
            focus: glam::Vec3::from_array(focus.to_array()),
        }
    }

    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        terrain: &StreamedTerrain,
        light: Option<&TerrainLight>,
    ) -> Result<(), String> {
        // Preserve primary campsite objects before supplemental waterfall scenery.
        if !terrain.parsed_tiles.contains_key(&self.primary) {
            return Ok(());
        }
        let tiles: Vec<_> = std::iter::once(self.primary)
            .chain(
                terrain
                    .parsed_tiles
                    .keys()
                    .copied()
                    .filter(|tile| *tile != self.primary),
            )
            .collect();
        for tile in tiles {
            if self.attached_tiles.contains(&tile) {
                continue;
            }
            let parsed = &terrain.parsed_tiles[&tile];
            let objects = parsed
                .obj
                .as_ref()
                .ok_or_else(|| format!("Campsite tile {tile:?} has no object companion"))?;
            for doodad in &objects.doodads {
                self.attach_doodad(parent, terrain, doodad, tile, light)?;
            }
            if tile == self.primary {
                for wmo in &objects.wmos {
                    self.attach_wmo(parent, wmo, light)?;
                }
            }
            self.attached_tiles.insert(tile);
        }
        Ok(())
    }

    fn attach_doodad(
        &mut self,
        parent: &mut Gd<Node3D>,
        terrain: &StreamedTerrain,
        doodad: &DoodadPlacement,
        tile: (u32, u32),
        light: Option<&TerrainLight>,
    ) -> Result<(), String> {
        if self.attached_doodads.contains(&doodad.unique_id) {
            return Ok(());
        }
        let path = doodad
            .fdid
            .and_then(|fdid| self.resolver.resolve_path(fdid))
            .or_else(|| doodad.path.clone());
        if !self.select_doodad(doodad, path.as_deref(), tile) {
            return Ok(());
        }
        let model = self.load_placed_doodad(doodad, path.as_deref(), tile, terrain)?;
        bind_visual_light(&model, light);
        self.attach_model(parent, &model);
        self.attached_doodads.insert(doodad.unique_id);
        Ok(())
    }

    fn attach_model(&mut self, parent: &mut Gd<Node3D>, model: &Gd<Node3D>) {
        let root = self.root.get_or_insert_with(|| {
            let mut root = Node3D::new_alloc();
            root.set_name("CampsiteObjects");
            parent.add_child(&root);
            root
        });
        root.add_child(model);
    }

    fn attach_wmo(
        &mut self,
        parent: &mut Gd<Node3D>,
        placement: &WmoPlacement,
        light: Option<&TerrainLight>,
    ) -> Result<(), String> {
        let position = placement_position(placement.position, self.primary.0, self.primary.1);
        if position.distance(self.focus) > CAMPSITE_WMO_RADIUS
            || self.attached_wmos.contains(&placement.unique_id)
        {
            return Ok(());
        }
        let asset = crate::wmo::assets::read_placement(&self.resolver, &self.data_root, placement)?;
        let mut model = crate::wmo::scene::build_wmo_node(
            &asset,
            &self.resolver,
            &self.data_root,
            placement.doodad_set,
            light,
        )?;
        let rotation = shared::ground::placement_rotation(placement.rotation);
        model.set_name(&format!("Wmo{}", placement.unique_id));
        model.set_position(Vector3::from_array(position.to_array()));
        model.set_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ));
        model.set_scale(Vector3::ONE * placement.scale);
        self.attach_model(parent, &model);
        self.attached_wmos.insert(placement.unique_id);
        Ok(())
    }

    fn select_doodad(
        &self,
        doodad: &DoodadPlacement,
        path: Option<&str>,
        tile: (u32, u32),
    ) -> bool {
        if tile == self.primary {
            is_primary_campsite_doodad(
                doodad,
                path,
                tile.0,
                tile.1,
                self.focus,
                CAMPSITE_PROP_RADIUS,
            )
        } else {
            is_supplemental_campsite_doodad(path)
        }
    }

    fn load_placed_doodad(
        &self,
        doodad: &DoodadPlacement,
        path: Option<&str>,
        tile: (u32, u32),
        terrain: &StreamedTerrain,
    ) -> Result<Gd<Node3D>, String> {
        let fdid = doodad
            .fdid
            .or_else(|| path.and_then(|path| self.resolver.lookup_path(path)))
            .ok_or_else(|| {
                format!(
                    "Campsite doodad {} has no resolvable model",
                    doodad.unique_id
                )
            })?;
        let mut model = self.load_model(fdid)?;
        let position = doodad_position(doodad, tile.0, tile.1);
        let height = terrain.height_at(position.x, position.z);
        let placement = campsite_doodad_placement(doodad, path, tile.0, tile.1, height);
        let rotation = placement.rotation;
        model.set_name(&format!("Doodad{}", doodad.unique_id));
        model.set_position(Vector3::from_array(placement.translation.to_array()));
        model.set_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        ));
        model.set_scale(Vector3::from_array(placement.scale.to_array()));
        Ok(model)
    }

    fn load_model(&self, fdid: u32) -> Result<Gd<Node3D>, String> {
        let path = cache_model_files(&self.resolver, &self.data_root, fdid)?;
        let path = GString::from(path.to_string_lossy().as_ref());
        let parsed = read_model(&path)?;
        cache_model_textures(&self.resolver, &self.data_root, &[0; 3], &parsed)?;
        let (model, missing) = build_model(&parsed, &path, &[0; 3], None)?;
        if !missing.is_empty() {
            model.free();
            return Err(format!(
                "Campsite model {fdid} missing textures {missing:?}"
            ));
        }
        Ok(model)
    }

    pub fn update_lighting(&self, light: &TerrainLight) {
        if let Some(root) = &self.root {
            bind_visual_light(root, Some(light));
        }
    }

    pub fn reset(&mut self) {
        if let Some(root) = self.root.take() {
            root.free();
        }
        self.attached_tiles.clear();
        self.attached_doodads.clear();
        self.attached_wmos.clear();
    }
}
