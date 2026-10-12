//! Saved housing decor visuals. Only authoritative layouts create durable scene nodes.
use crate::assets::creature::{cache_model_files, cache_model_textures, local_resolver};
use crate::assets::{build_model, read_model};
use game_engine_core::housing::HousingCatalog;
use godot::classes::base_material_3d::{BlendMode, ShadingMode, Transparency};
use godot::classes::{Material, MeshInstance3D, StandardMaterial3D};
use godot::prelude::*;
use shared::protocol::{HouseLayout, PlacedDecor};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Default)]
pub(crate) struct HousingDecor {
    nodes: BTreeMap<u64, Gd<Node3D>>,
    pub(crate) preview: Option<Gd<Node3D>>,
    saved: Option<HouseLayout>,
    highlighted: Option<u64>,
}

pub(crate) fn load_visual(
    data: &Path,
    catalog: &HousingCatalog,
    decor_id: u32,
) -> Result<Gd<Node3D>, String> {
    let decor = catalog
        .decor
        .iter()
        .find(|decor| decor.id == decor_id)
        .ok_or_else(|| format!("Decor {decor_id} is not in the authenticated catalog."))?;
    let resolver = local_resolver(data);
    let path = cache_model_files(&resolver, data, decor.model_fdid)
        .map_err(|error| format!("Decor {decor_id} model {}: {error}", decor.model_fdid))?;
    let model_path = GString::from(path.to_string_lossy().as_ref());
    let parsed = read_model(&model_path)?;
    cache_model_textures(&resolver, data, &[0; 3], &parsed)?;
    let (mut visual, missing) = build_model(&parsed, &model_path, &[0; 3], None)?;
    if !missing.is_empty() {
        visual.free();
        return Err(format!(
            "Decor {decor_id} is missing authored textures {missing:?}"
        ));
    }
    visual.set_meta("housing_decor_id", &(decor_id as i64).to_variant());
    visual.set_meta(
        "model_file_data_id",
        &(decor.model_fdid as i64).to_variant(),
    );
    Ok(visual)
}

fn apply_transform(node: &mut Gd<Node3D>, decor: &PlacedDecor) {
    apply_precision(node, decor.transform);
}

fn apply_precision(node: &mut Gd<Node3D>, transform: shared::protocol::DecorTransform) {
    node.set_position(Vector3::from_array(transform.position));
    node.set_rotation(Vector3::new(transform.pitch, transform.yaw, transform.roll));
    node.set_scale(Vector3::ONE * transform.scale);
}

fn apply_hover_overlay(node: &Gd<Node3D>, material: Option<&Gd<Material>>) {
    let meshes = node
        .find_children_ex("*")
        .type_("MeshInstance3D")
        .owned(false)
        .done();
    for mesh in meshes.iter_shared() {
        mesh.cast::<MeshInstance3D>().set_material_overlay(material);
    }
}

fn cleanup_highlight_material() -> Gd<Material> {
    let mut material = StandardMaterial3D::new_gd();
    material.set_shading_mode(ShadingMode::UNSHADED);
    material.set_transparency(Transparency::ALPHA);
    material.set_blend_mode(BlendMode::ADD);
    material.set_albedo(Color::from_rgba(1.0, 0.82, 0.0, 0.25));
    material.upcast()
}

impl HousingDecor {
    pub(crate) fn highlight(&mut self, instance: Option<u64>) {
        if self.highlighted == instance {
            return;
        }
        if let Some(previous) = self.highlighted.take().and_then(|id| self.nodes.get(&id)) {
            apply_hover_overlay(previous, None);
        }
        if let Some(node) = instance.and_then(|id| self.nodes.get(&id)) {
            apply_hover_overlay(node, Some(&cleanup_highlight_material()));
            self.highlighted = instance;
        }
    }

    pub(crate) fn restore_saved(&mut self) {
        self.saved = None;
    }

    pub(crate) fn stage(
        &mut self,
        instance: u64,
        transform: shared::protocol::DecorTransform,
    ) -> Result<(), String> {
        let node = self
            .nodes
            .get_mut(&instance)
            .ok_or("Selected decor visual is unavailable.")?;
        apply_precision(node, transform);
        Ok(())
    }

    pub(crate) fn contains(&self, instance: u64) -> bool {
        self.nodes.contains_key(&instance)
    }

    pub(crate) fn clear_preview(&mut self) {
        if let Some(node) = self.preview.take() {
            node.free();
        }
    }

    pub(crate) fn clear(&mut self) {
        self.highlight(None);
        self.clear_preview();
        for (_, node) in std::mem::take(&mut self.nodes) {
            node.free();
        }
        self.saved = None;
    }

    pub(crate) fn preview_at(
        &mut self,
        parent: &mut Gd<Node3D>,
        data: &Path,
        catalog: &HousingCatalog,
        decor_id: u32,
        transform: shared::protocol::DecorTransform,
    ) -> Result<(), String> {
        if self.preview.is_none() {
            let mut visual = load_visual(data, catalog, decor_id)?;
            visual.set_name("HousingDecorPreview");
            parent.add_child(&visual);
            self.preview = Some(visual);
        }
        let preview = self.preview.as_mut().ok_or("Decor preview disappeared.")?;
        apply_precision(preview, transform);
        Ok(())
    }

    pub(crate) fn sync_layout(
        &mut self,
        parent: &mut Gd<Node3D>,
        data: &Path,
        catalog: &HousingCatalog,
        layout: &HouseLayout,
    ) -> Result<(), String> {
        if self.saved.as_ref() == Some(layout) {
            return Ok(());
        }
        let removed: Vec<_> = self
            .nodes
            .keys()
            .copied()
            .filter(|id| !layout.decor.iter().any(|decor| decor.instance_id == *id))
            .collect();
        if self.highlighted.is_some_and(|id| removed.contains(&id)) {
            self.highlight(None);
        }
        for id in removed {
            if let Some(node) = self.nodes.remove(&id) {
                node.free();
            }
        }
        for decor in &layout.decor {
            if !self.nodes.contains_key(&decor.instance_id) {
                let mut node = load_visual(data, catalog, decor.decor_id)?;
                crate::targeting::attach_pick_area(&node, decor.instance_id)?;
                node.set_name(&format!("HouseDecor_{}", decor.instance_id));
                node.set_meta(
                    "housing_instance_id",
                    &(decor.instance_id as i64).to_variant(),
                );
                parent.add_child(&node);
                self.nodes.insert(decor.instance_id, node);
            }
            apply_transform(
                self.nodes
                    .get_mut(&decor.instance_id)
                    .ok_or("Saved decor node disappeared.")?,
                decor,
            );
        }
        self.saved = Some(layout.clone());
        Ok(())
    }
}
