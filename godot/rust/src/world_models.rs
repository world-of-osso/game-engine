//! Imported creature display catalog and native NPC visual children.
use std::{f32::consts::FRAC_PI_2, path::PathBuf};

use game_engine_core::creature_display_data::{CreatureDisplay, query_display};
use godot::{
    classes::{MeshInstance3D, Node3D, ShaderMaterial},
    prelude::*,
};
use rusqlite::{Connection, OpenFlags};

use crate::{
    assets::{appearance::NpcAppearances, creature::load_creature_model},
    lighting::TerrainLight,
};

pub(crate) fn bind_visual_light(visual: &Gd<Node3D>, light: Option<&TerrainLight>) {
    let meshes = visual
        .find_children_ex("*")
        .type_("MeshInstance3D")
        .owned(false)
        .done();
    for node in meshes.iter_shared() {
        let mesh = node.cast::<MeshInstance3D>();
        // The native M2 loader creates one surface and one ShaderMaterial per batch.
        let mut material = mesh
            .get_surface_override_material(0)
            .expect("M2 batch has an authored material")
            .cast::<ShaderMaterial>();
        match light {
            Some(light) => light.bind_model(&mut material),
            None => TerrainLight::clear_model(&mut material),
        }
    }
}

pub(crate) struct CreatureModels {
    data_root: PathBuf,
    cache_root: PathBuf,
    catalog: Option<Connection>,
    appearances: NpcAppearances,
}

impl CreatureModels {
    pub fn new(data_root: PathBuf, cache_root: PathBuf) -> Self {
        Self {
            data_root,
            cache_root,
            catalog: None,
            appearances: NpcAppearances::default(),
        }
    }

    fn query_display(&mut self, display_id: u32) -> Result<CreatureDisplay, String> {
        let path = self.data_root.join("cache/creature_display.sqlite");
        if self.catalog.is_none() {
            let connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|error| format!("Cannot open {}: {error}", path.display()))?;
            self.catalog = Some(connection);
        }
        let connection = self.catalog.as_ref().expect("Catalog opened above");
        query_display(connection, display_id)
            .map_err(|error| {
                format!(
                    "Cannot query display {display_id} in {}: {error}",
                    path.display()
                )
            })?
            .ok_or_else(|| {
                format!(
                    "Creature display {display_id} absent from {}",
                    path.display()
                )
            })
    }

    pub fn load_visual(&mut self, display_id: u32) -> Result<Gd<Node3D>, String> {
        let display = self.query_display(display_id)?;
        let appearance = self
            .appearances
            .prepare(&self.data_root, &self.cache_root, display_id)?;
        let (mut model, missing) = load_creature_model(
            &self.data_root,
            &self.cache_root,
            &display,
            appearance.as_ref(),
        )?;
        if !missing.is_empty() {
            godot_warn!("Creature display {display_id} missing texture FDIDs: {missing:?}");
        }
        model.set_name("NpcModel");
        let scale = if display.scale_milli == 0 {
            1.0
        } else {
            display.scale_milli as f32 / 1000.0
        };
        let mut visual = Node3D::new_alloc();
        visual.set_name("NpcVisualRoot");
        visual.set_scale(Vector3::ONE * scale.max(0.01));
        visual.set_rotation(Vector3::new(0.0, -FRAC_PI_2, 0.0));
        visual.add_child(&model);
        Ok(visual)
    }
}
