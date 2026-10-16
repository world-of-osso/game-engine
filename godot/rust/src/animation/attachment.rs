//! Authored attachment participation, independent of color visibility and child clocks.
use game_engine_core::{asset::m2_format::m2_attach::M2Attachment, m2};
use godot::{
    classes::{
        GeometryInstance3D, Node, Node3D, Skeleton3D, geometry_instance_3d::ShadowCastingSetting,
    },
    prelude::*,
};

use super::AnimationState;

pub(super) fn take_callbacks(animation: &mut AnimationState, active: bool) -> Vec<[u8; 4]> {
    let events = animation.take_fired_events();
    if active { events } else { Vec::new() }
}

const ACTIVE_META: &str = "m2_attachment_active";
const SHADOW_META: &str = "m2_attachment_shadow_mode";

pub(super) struct AttachmentGates {
    attachments: Vec<M2Attachment>,
    points: Vec<(usize, Gd<Node3D>)>,
}

impl AttachmentGates {
    pub fn new(model: &m2::Model, skeleton: &Gd<Skeleton3D>) -> Result<Option<Self>, String> {
        let authored = model.attachments.iter().any(|attachment| {
            attachment
                .activation
                .sequences
                .iter()
                .any(|(times, _)| !times.is_empty())
        });
        if !authored {
            return Ok(None);
        }
        let points = attachment_indices(model)
            .into_iter()
            .map(|(id, index)| {
                let point = skeleton
                    .try_get_node_as::<Node3D>(&format!("AttachmentBone{id}/Attachment{id}"))
                    .ok_or_else(|| format!("Attachment {id} has no native point"))?;
                Ok((index, point))
            })
            .collect::<Result<_, String>>()?;
        Ok(Some(Self {
            attachments: model.attachments.clone(),
            points,
        }))
    }

    pub fn apply(&mut self, animation: &AnimationState) {
        for (index, point) in &mut self.points {
            let active = self.attachments[*index].active_at(
                animation.current,
                animation.time_ms as u32,
                &animation.global_sequences,
                animation.global_ms,
            );
            point.set_meta(ACTIVE_META, &active.to_variant());
            refresh_attachment_shadows(&point.clone().upcast());
        }
    }
}

fn attachment_indices(model: &m2::Model) -> Vec<(u32, usize)> {
    if model.attachment_lookup.is_empty() {
        return model
            .attachments
            .iter()
            .enumerate()
            .map(|(index, attachment)| (attachment.id, index))
            .collect();
    }
    model
        .attachment_lookup
        .iter()
        .enumerate()
        .filter_map(|(id, &index)| {
            let index = usize::try_from(index).ok()?;
            model.attachments.get(index).map(|_| (id as u32, index))
        })
        .collect()
}

/// Inactive outer attachments also exclude every recursively attached descendant.
/// Absence of an attachment ancestor leaves ordinary model playback untouched.
pub(super) fn participates(node: &Gd<Node>) -> bool {
    let mut ancestor = Some(node.clone());
    while let Some(node) = ancestor {
        if node.has_meta(ACTIVE_META) && !node.get_meta(ACTIVE_META).booleanize() {
            return false;
        }
        ancestor = node.get_parent();
    }
    true
}

pub(crate) fn refresh_attachment_shadows(node: &Gd<Node>) {
    if let Ok(mut geometry) = node.clone().try_cast::<GeometryInstance3D>() {
        let active = participates(node);
        if !active && !geometry.has_meta(SHADOW_META) {
            let original = geometry.get_cast_shadows_setting().ord();
            geometry.set_meta(SHADOW_META, &original.to_variant());
            geometry.set_cast_shadows_setting(ShadowCastingSetting::OFF);
        } else if active && geometry.has_meta(SHADOW_META) {
            let original = geometry.get_meta(SHADOW_META).to::<i32>();
            geometry.set_cast_shadows_setting(ShadowCastingSetting::from_ord(original));
            geometry.remove_meta(SHADOW_META);
        }
    }
    for child in node.get_children().iter_shared() {
        refresh_attachment_shadows(&child);
    }
}
