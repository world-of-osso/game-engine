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
const CAMERA_META: &str = "unit_camera_admitted";

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
    if !point.is_inside_tree() {
        return None;
    }
    let mut tree = point.get_tree();
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
                refresh_unit_shadows(&point.node.clone().upcast());
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
    !has_ancestor_meta_false(node, ACTIVE_META)
}

fn has_ancestor_meta_false(node: &Gd<Node>, meta: &str) -> bool {
    let mut ancestor = Some(node.clone());
    while let Some(node) = ancestor {
        if node.has_meta(meta) && !node.get_meta(meta).booleanize() {
            return true;
        }
        ancestor = node.get_parent();
    }
    false
}

/// Marks a unit subtree admitted or rejected by the camera; `refresh_unit_shadows`
/// applies it.
pub(crate) fn set_camera_admitted(node: &mut Gd<Node>, admitted: bool) {
    node.set_meta(CAMERA_META, &admitted.to_variant());
}

/// The cast-shadow mode of a geometry authored `original`: an inactive attachment
/// casts nothing, and a camera-rejected caster keeps only its shadow. Godot cannot
/// combine shadows-only with double-sided casting, so those stay drawn.
pub(crate) fn shadow_mode(
    original: ShadowCastingSetting,
    active: bool,
    camera_admitted: bool,
) -> ShadowCastingSetting {
    if !active {
        ShadowCastingSetting::OFF
    } else if !camera_admitted && original == ShadowCastingSetting::ON {
        ShadowCastingSetting::SHADOWS_ONLY
    } else {
        original
    }
}

/// Applies attachment participation and camera admission to every geometry under
/// `node`, keeping each geometry's authored mode while either changes it.
pub(crate) fn refresh_unit_shadows(node: &Gd<Node>) {
    if let Ok(mut geometry) = node.clone().try_cast::<GeometryInstance3D>() {
        let stored = geometry.has_meta(SHADOW_META);
        let original = if stored {
            ShadowCastingSetting::from_ord(geometry.get_meta(SHADOW_META).to::<i32>())
        } else {
            geometry.get_cast_shadows_setting()
        };
        let mode = shadow_mode(
            original,
            participates(node),
            !has_ancestor_meta_false(node, CAMERA_META),
        );
        if mode != geometry.get_cast_shadows_setting() {
            geometry.set_cast_shadows_setting(mode);
        }
        if mode == original && stored {
            geometry.remove_meta(SHADOW_META);
        } else if mode != original && !stored {
            geometry.set_meta(SHADOW_META, &original.ord().to_variant());
        }
    }
    for child in node.get_children().iter_shared() {
        refresh_unit_shadows(&child);
    }
}
