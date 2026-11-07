//! A world unit draws one batch per skin section, and most sections of a character or
//! creature bind the same material (the body texture with one blend and render flags).
//! Godot cannot instance skinned surfaces, so each section was its own draw in the color
//! pass and in every shadow cascade. Once a unit's model is assembled and its geosets are
//! settled, the drawn opaque batches that bind identical materials draw as one surface.
//!
//! Only batches that draw in the opaque pass with depth test and depth write merge: their
//! pixels do not depend on the order their triangles are drawn in. Models whose geosets
//! change after assembly (the character creation preview) are not merged.

use std::{cell::RefCell, collections::BTreeMap, collections::HashMap};

use game_engine_core::{
    m2,
    m2_batch_data::{ResolvedBatch, TextureOverlay},
    m2_material::BatchBinding,
};
use godot::{
    classes::{ArrayMesh, MeshInstance3D, Node3D},
    prelude::*,
};

use super::{M2_MERGE_GROUP_META, M2_SUBMESH_META};

/// Everything `material::load_material` derives a batch's material from besides the
/// model-wide replacement textures: batches of one model with equal keys bind identical,
/// identically animated materials.
#[derive(Clone, PartialEq)]
pub(crate) struct MergeKey {
    binding: BatchBinding,
    render_flags: u16,
    blend_mode: u16,
    overlays: Vec<TextureOverlay>,
}

/// M2 blend modes drawn in the opaque pass: opaque and alpha key (GX blend 0 and 1).
const OPAQUE_BLEND_MODES: [u16; 2] = [0, 1];
/// Render flags 0x8 (no depth test) and 0x10 (no depth write) make the draw order visible.
const DEPTH_ORDER_FLAGS: u16 = 0x8 | 0x10;

impl MergeKey {
    /// The key of a batch that may merge; `None` for one whose draw order shows.
    pub(crate) fn of(batch: &ResolvedBatch, binding: &BatchBinding) -> Option<Self> {
        (OPAQUE_BLEND_MODES.contains(&batch.blend_mode)
            && batch.render_flags & DEPTH_ORDER_FLAGS == 0)
            .then(|| Self {
                binding: binding.clone(),
                render_flags: batch.render_flags,
                blend_mode: batch.blend_mode,
                overlays: batch.overlays.clone(),
            })
    }
}

thread_local! {
    /// Merged surfaces by model file and drawn submeshes: units of one model with the same
    /// geosets share them, as `MESHES` shares single batches.
    static MERGED: RefCell<HashMap<(String, Vec<usize>), Gd<ArrayMesh>>> =
        RefCell::new(HashMap::new());
}

pub(crate) fn clear_merged_meshes() {
    MERGED.with_borrow_mut(HashMap::clear);
}

/// Draws the shown batches of `root` (built from `model` at `path`) that share a merge group
/// as their first batch: it takes one surface of all their submeshes and keeps its own
/// name, material and instance settings; the others are freed. `root` must not be in the
/// scene tree yet.
pub(crate) fn merge_shown_batches(
    root: &Gd<Node3D>,
    model: &m2::Model,
    path: &str,
) -> Result<(), String> {
    for members in shown_merge_groups(root) {
        let submeshes: Vec<usize> = members.iter().map(|(submesh, _)| *submesh).collect();
        let mesh = merged_mesh(model, path, &submeshes)?;
        let mut members = members.into_iter().map(|(_, batch)| batch);
        let mut first = members.next().expect("a merge group has members");
        first.set_mesh(&mesh);
        for other in members {
            other.free();
        }
    }
    Ok(())
}

/// The shown batches of each merge group with at least two members, in batch order.
fn shown_merge_groups(root: &Gd<Node3D>) -> Vec<Vec<(usize, Gd<MeshInstance3D>)>> {
    let mut groups: BTreeMap<i64, Vec<(usize, Gd<MeshInstance3D>)>> = BTreeMap::new();
    for child in root.get_children().iter_shared() {
        let Ok(batch) = child.try_cast::<MeshInstance3D>() else {
            continue;
        };
        if !batch.is_visible() || !batch.has_meta(M2_MERGE_GROUP_META) {
            continue;
        }
        let group = batch.get_meta(M2_MERGE_GROUP_META).to::<i64>();
        let submesh = batch.get_meta(M2_SUBMESH_META).to::<i64>();
        let submesh = usize::try_from(submesh).expect("M2 batch submesh is an index");
        groups.entry(group).or_default().push((submesh, batch));
    }
    groups
        .into_values()
        .filter(|members| members.len() > 1)
        .collect()
}

fn merged_mesh(
    model: &m2::Model,
    path: &str,
    submeshes: &[usize],
) -> Result<Gd<ArrayMesh>, String> {
    let key = (path.to_owned(), submeshes.to_vec());
    if let Some(mesh) = MERGED.with_borrow(|merged| merged.get(&key).cloned()) {
        return Ok(mesh);
    }
    let subs = submeshes
        .iter()
        .map(|&index| {
            model
                .submeshes
                .get(index)
                .ok_or_else(|| format!("{path}: merged batch references absent submesh {index}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mesh = super::streams_mesh(&m2::merged_submesh_arrays(model, &subs)?);
    MERGED.with_borrow_mut(|merged| merged.insert(key, mesh.clone()));
    Ok(mesh)
}
