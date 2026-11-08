//! Authored M2 attachment lookup nodes driven by the native skeleton pose.
use game_engine_core::{asset::m2_format::m2_attach::M2Attachment, m2};
use godot::{
    classes::{BoneAttachment3D, Node3D, Skeleton3D},
    prelude::*,
};

use super::wow_vec3;

/// Skeleton metadata: the model's attachments as `[id, bone, x, y, z]` runs (the point's
/// offset from its bone's pivot), from which `skeleton_attachment_point` makes each point on
/// first use. A creature uses a few of its ~45 attachments; a node pair for each one cost
/// half of its template's pack and a transform update per pose for nothing.
const M2_ATTACHMENTS_META: &str = "m2_attachments";
const ATTACHMENT_STRIDE: usize = 5;

/// Records the attachments of `model` on its skeleton (`M2_ATTACHMENTS_META`).
pub(super) fn record_attachments(
    skeleton: &mut Gd<Skeleton3D>,
    model: &m2::Model,
) -> Result<(), String> {
    let mut table = Vec::new();
    for (id, attachment) in model_attachments(model) {
        let bone = model
            .bones
            .get(attachment.bone as usize)
            .ok_or_else(|| format!("Attachment {id} references absent bone {}", attachment.bone))?;
        let offset = wow_vec3(attachment.position) - wow_vec3(bone.pivot);
        table.extend([
            id as f32,
            f32::from(attachment.bone),
            offset.x,
            offset.y,
            offset.z,
        ]);
    }
    if !table.is_empty() {
        skeleton.set_meta(
            M2_ATTACHMENTS_META,
            &PackedFloat32Array::from(table).to_variant(),
        );
    }
    Ok(())
}

/// The model's attachments by ID: the lookup table's entries, or every attachment by its
/// own ID when the model has no lookup table.
fn model_attachments(model: &m2::Model) -> Vec<(u32, &M2Attachment)> {
    if model.attachment_lookup.is_empty() {
        return model
            .attachments
            .iter()
            .map(|attachment| (attachment.id, attachment))
            .collect();
    }
    model
        .attachment_lookup
        .iter()
        .enumerate()
        .filter_map(|(id, &index)| {
            let attachment = model.attachments.get(usize::try_from(index).ok()?)?;
            Some((id as u32, attachment))
        })
        .collect()
}

fn point_path(id: u32) -> String {
    format!("AttachmentBone{id}/Attachment{id}")
}

/// The attachment point `id` under the model root `model` (`Skeleton3D/AttachmentBone{id}/
/// Attachment{id}`), made on first use (`skeleton_attachment_point`); `None` when the model
/// has no such attachment. The rigs of effect anchors and stand-in actors hold plain nodes
/// at the same paths.
pub(crate) fn attachment_point(model: &Gd<Node3D>, id: u32) -> Option<Gd<Node3D>> {
    model
        .try_get_node_as::<Node3D>(&format!("Skeleton3D/{}", point_path(id)))
        .or_else(|| {
            let skeleton = model.try_get_node_as::<Skeleton3D>("Skeleton3D")?;
            skeleton_attachment_point(&skeleton, id)
        })
}

/// The attachment point `id` of the creature visual root `visual` (its `NpcModel`).
pub(crate) fn visual_attachment_point(visual: &Gd<Node3D>, id: u32) -> Option<Gd<Node3D>> {
    attachment_point(&visual.try_get_node_as::<Node3D>("NpcModel")?, id)
}

/// The attachment point `id` of `skeleton`, made from its attachment table on first use: a
/// `BoneAttachment3D` on the attachment's bone holding the point at its offset, posed at
/// once when the skeleton is in the tree. `None` when the model has no such attachment.
pub(crate) fn skeleton_attachment_point(skeleton: &Gd<Skeleton3D>, id: u32) -> Option<Gd<Node3D>> {
    let path = point_path(id);
    if let Some(point) = skeleton.try_get_node_as::<Node3D>(&path) {
        return Some(point);
    }
    if !skeleton.has_meta(M2_ATTACHMENTS_META) {
        return None;
    }
    let table = skeleton
        .get_meta(M2_ATTACHMENTS_META)
        .to::<PackedFloat32Array>();
    let entry = table
        .as_slice()
        .chunks_exact(ATTACHMENT_STRIDE)
        .find(|entry| entry[0] as u32 == id)?;
    let mut bone = BoneAttachment3D::new_alloc();
    bone.set_name(&format!("AttachmentBone{id}"));
    bone.set_bone_idx(entry[1] as i32);
    // BoneAttachment3D owns its transform. Keep the authored offset on its child.
    let mut point = Node3D::new_alloc();
    point.set_name(&format!("Attachment{id}"));
    point.set_position(Vector3::new(entry[2], entry[3], entry[4]));
    bone.add_child(&point);
    skeleton.clone().add_child(&bone);
    if bone.is_inside_tree() {
        bone.on_skeleton_update();
    }
    Some(point)
}
