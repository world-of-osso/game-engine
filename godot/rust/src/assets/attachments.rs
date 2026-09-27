//! Authored M2 attachment lookup nodes driven by the native skeleton pose.
use game_engine_core::{asset::m2_format::m2_attach::M2Attachment, m2};
use godot::{
    classes::{BoneAttachment3D, Skeleton3D},
    prelude::*,
};

use super::wow_vec3;

pub(super) fn add_attachment_nodes(
    skeleton: &mut Gd<Skeleton3D>,
    model: &m2::Model,
) -> Result<(), String> {
    if model.attachment_lookup.is_empty() {
        for attachment in &model.attachments {
            add_attachment_node(skeleton, &model.bones, attachment.id, attachment)?;
        }
    } else {
        for (id, &index) in model.attachment_lookup.iter().enumerate() {
            let Ok(index) = usize::try_from(index) else {
                continue;
            };
            let Some(attachment) = model.attachments.get(index) else {
                continue;
            };
            add_attachment_node(skeleton, &model.bones, id as u32, attachment)?;
        }
    }
    Ok(())
}

fn add_attachment_node(
    skeleton: &mut Gd<Skeleton3D>,
    bones: &[m2::Bone],
    id: u32,
    attachment: &M2Attachment,
) -> Result<(), String> {
    let bone = bones
        .get(attachment.bone as usize)
        .ok_or_else(|| format!("Attachment {id} references absent bone {}", attachment.bone))?;
    let mut node = BoneAttachment3D::new_alloc();
    node.set_name(&format!("Attachment{id}"));
    node.set_bone_idx(attachment.bone as i32);
    // The original joint origin is zero at rest; native bones rest at their M2 pivots.
    node.set_position(wow_vec3(attachment.position) - wow_vec3(bone.pivot));
    skeleton.add_child(&node);
    Ok(())
}
