//! Authored attachment participation, independent of color visibility and child clocks.
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

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

struct AttachmentPoint {
    index: usize,
    node: Gd<Node3D>,
    active: Option<bool>,
    tree_generation: u64,
}

pub(super) struct AttachmentGates {
    attachments: Vec<M2Attachment>,
    points: Vec<AttachmentPoint>,
    tree_generation: Option<Rc<Cell<u64>>>,
}

thread_local! {
    static TREE_GENERATION: RefCell<Option<(InstanceId, Rc<Cell<u64>>)>> = const { RefCell::new(None) };
    static GATE_TIME: Cell<Duration> = const { Cell::new(Duration::ZERO) };
}

/// Take the gate time accumulated since the previous GameClient frame.
pub(crate) fn take_gate_time_ms() -> f64 {
    GATE_TIME.with(|time| time.replace(Duration::ZERO).as_secs_f64() * 1000.0)
}

/// A single scene-tree signal tracks descendant additions, removals and replacements.
fn watch_tree(point: &Gd<Node3D>) -> Option<Rc<Cell<u64>>> {
    let mut tree = point.get_tree()?;
    TREE_GENERATION.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some((id, generation)) = slot.as_ref()
            && *id == tree.instance_id()
        {
            return Some(generation.clone());
        }
        let generation = Rc::new(Cell::new(0u64));
        let changed = generation.clone();
        let callback = Callable::from_fn("attachment-tree-changed", move |_| {
            changed.set(changed.get().wrapping_add(1));
        });
        let result = tree.connect("tree_changed", &callback);
        assert_eq!(
            result,
            godot::global::Error::OK,
            "Cannot watch attachment tree changes"
        );
        *slot = Some((tree.instance_id(), generation.clone()));
        Some(generation)
    })
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
                Ok(AttachmentPoint {
                    index,
                    node: point,
                    active: None,
                    tree_generation: 0,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Some(Self {
            attachments: model.attachments.clone(),
            points,
            tree_generation: None,
        }))
    }

    pub fn apply(&mut self, animation: &AnimationState) {
        let started = Instant::now();
        if self.tree_generation.is_none() {
            self.tree_generation = self
                .points
                .first()
                .and_then(|point| watch_tree(&point.node));
        }
        let generation = self.tree_generation.as_ref().map(|value| value.get());
        for point in &mut self.points {
            let active = self.attachments[point.index].active_at(
                animation.current,
                animation.time_ms as u32,
                &animation.global_sequences,
                animation.global_ms,
            );
            let changed = point.active != Some(active);
            if changed {
                point.node.set_meta(ACTIVE_META, &active.to_variant());
                point.active = Some(active);
            }
            let tree_changed = generation.is_none_or(|value| value != point.tree_generation);
            if changed || tree_changed || !point.node.is_inside_tree() {
                refresh_attachment_shadows(&point.node.clone().upcast());
            }
            if let Some(value) = generation {
                point.tree_generation = value;
            }
        }
        GATE_TIME.with(|time| time.set(time.get() + started.elapsed()));
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
