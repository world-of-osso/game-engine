//! A world creature visual kept per display and gear: a later unit with both instantiates
//! it instead of building it again. A Stormwind guard's build (four models, 107 batches, their
//! skeletons, players, composited textures and equipment) took 10-15 ms of one main-thread
//! frame each time a guard came into view; its instance shares the meshes, skins and
//! static materials and makes no material. The template is a `PackedScene`: instantiating
//! the guard's 205 nodes took 3-5 ms where `Node::duplicate`, which copies each property
//! through the property list, took 16-31 ms (loaded Linux host, debug library).
//!
//! Nor does the scene hold the skeletons' bones: packing saves each bone's properties by
//! name, which took 70-90% of `PackedScene::pack` (a guard's 237 bones: 1.69 of the pack's
//! 2.48 ms, Linux debug library) and was a first-sight frame's largest step (4.3 ms median
//! on the Windows release build). The template keeps them as `SkeletonBones` and each
//! instance adds them again.
//!
//! A packed scene cannot hold a Rust node's state (and `WowM2Lights` has no default
//! constructor), so the template holds none: each instance gets its own animation player, material animation and model lights,
//! built from the parsed model as `build_model` builds them. A material that animates on
//! its model's sequence timeline follows its own instance's animation, so each instance
//! copies it; one that animates on global sequences only samples alike in every model, so
//! the instances share it and the first one the animation LOD lets sample it in a frame
//! sets it (`WowMaterialAnimation::share_global_materials`).

use std::{path::Path, sync::Arc};

use game_engine_core::{
    creature_display_data::CreatureDisplay,
    m2_material::{BatchBinding, MaterialTracks},
};
use godot::{
    classes::{
        ClassDb, MeshInstance3D, Node3D, PackedScene, ShaderMaterial, Skeleton3D, class_db::ApiType,
    },
    prelude::*,
};

use super::{
    M2_SOURCE_META,
    creature::{CachedModel, CreatureGear, model_at},
    equipment::place_equipment,
    m2_lights::WowM2Lights,
    material::copy_material,
    uv_animation::WowMaterialAnimation,
};
use crate::animation::WowAnimationPlayer;
use shared::components::EquipmentAppearance;

/// A built creature visual without its Rust nodes, and how to make them again.
pub(crate) struct CreatureTemplate {
    source: TemplateSource,
    /// The replicated virtual items of the units it makes the visual of: each gives
    /// `source.gear`, so a unit with one of them instantiates it without a worker load.
    keys: Vec<EquipmentAppearance>,
    scene: Gd<PackedScene>,
    bones: Vec<SkeletonBones>,
    nodes: Vec<RustNode>,
}

/// The bones of the skeleton at `path` under the visual, which the template's scene does
/// not hold.
struct SkeletonBones {
    path: NodePath,
    bones: Vec<Bone>,
}

struct Bone {
    name: GString,
    parent: i32,
    rest: Transform3D,
    position: Vector3,
    rotation: Quaternion,
    scale: Vector3,
}

/// What a template's visual was built of, which a unit instantiating it attaches with.
pub(crate) struct TemplateSource {
    pub(crate) display_id: u32,
    pub(crate) display: CreatureDisplay,
    pub(crate) model: Arc<CachedModel>,
    pub(crate) gear: CreatureGear,
}

/// A Rust node of the built visual, by the path of its model root under the visual.
enum RustNode {
    Animation {
        root: NodePath,
        name: StringName,
        skeleton: NodePath,
    },
    MaterialAnimation {
        root: NodePath,
        name: StringName,
        /// Materials each instance copies, and materials the instances share.
        local: Vec<(Gd<ShaderMaterial>, BatchBinding)>,
        shared: Vec<(Gd<ShaderMaterial>, BatchBinding)>,
    },
    Lights {
        root: NodePath,
        name: StringName,
    },
}

impl CreatureTemplate {
    /// The template of `built`, a creature visual of `source` not yet in the scene tree,
    /// for units with the replicated virtual items `items`.
    pub(crate) fn of(
        source: TemplateSource,
        items: EquipmentAppearance,
        built: &Gd<Node3D>,
    ) -> Result<Self, String> {
        let span = crate::profile::span(|| "template.take_rust_nodes".to_owned());
        let (nodes, detached) = take_rust_nodes(built)?;
        drop(span);
        // Packing saves the nodes `built` owns; they own nothing again afterwards.
        let span = crate::profile::span(|| "template.pack".to_owned());
        let bones = take_bones(built);
        set_owner(built, Some(built.clone().upcast()));
        let mut scene = PackedScene::new_gd();
        let packed = scene.pack(built);
        set_owner(built, None);
        drop(span);
        let _span = crate::profile::span(|| "template.reattach".to_owned());
        for skeleton in &bones {
            skeleton.add_to(built)?;
        }
        for (mut parent, node, index) in detached {
            parent.add_child(&node);
            parent.move_child(&node, index);
        }
        if packed != godot::global::Error::OK {
            return Err(format!("Creature visual did not pack: {packed:?}"));
        }
        Ok(Self {
            source,
            keys: vec![items],
            scene,
            bones,
            nodes,
        })
    }

    pub(crate) fn source(&self) -> &TemplateSource {
        &self.source
    }

    /// Whether a unit of `display_id` with the replicated virtual items `items` gets this
    /// template's visual.
    pub(crate) fn made_for(&self, display_id: u32, items: &EquipmentAppearance) -> bool {
        self.source.display_id == display_id && self.keys.contains(items)
    }

    /// Records that the replicated virtual items `items` give this template's gear.
    pub(crate) fn add_key(&mut self, items: &EquipmentAppearance) {
        if !self.keys.contains(items) {
            self.keys.push(items.clone());
        }
    }

    /// Whether a unit of `display_id` with `gear` can instantiate this template: the same
    /// models, wherever its virtual items sit (`instance` places them).
    pub(crate) fn matches(&self, display_id: u32, gear: &CreatureGear) -> bool {
        self.source.display_id == display_id && Self::same_models(&self.source.gear, gear)
    }

    /// Whether `a` and `b` hold the same models, wherever their virtual items sit.
    pub(crate) fn same_models(a: &CreatureGear, b: &CreatureGear) -> bool {
        a.armor_models == b.armor_models
            && a.extra_geosets == b.extra_geosets
            && a.items.len() == b.items.len()
            && a.items.iter().zip(&b.items).all(|((a, _), (b, _))| a == b)
    }

    /// A new visual like the one this template was made from, its virtual items on the
    /// attachments `gear` places them (a unit's sheath state, which `matches` ignores).
    pub(crate) fn instance(&self, gear: &CreatureGear) -> Result<Gd<Node3D>, String> {
        let visual = self.instantiate()?;
        if let Err(error) = self.place_items(&visual, gear) {
            visual.free();
            return Err(error);
        }
        Ok(visual)
    }

    fn place_items(&self, visual: &Gd<Node3D>, gear: &CreatureGear) -> Result<(), String> {
        let moved = gear
            .items
            .iter()
            .zip(&self.source.gear.items)
            .filter(|((_, wanted), (_, built))| wanted != built);
        let mut model = None;
        for ((item, attachment), _) in moved {
            let model = match &model {
                Some(model) => model,
                None => model.insert(
                    visual
                        .try_get_node_as::<Node3D>("NpcModel")
                        .ok_or("Creature template instance has no NpcModel")?,
                ),
            };
            place_equipment(model, item.slot, *attachment)?;
        }
        Ok(())
    }

    fn instantiate(&self) -> Result<Gd<Node3D>, String> {
        let span = crate::profile::span(|| "template.instantiate".to_owned());
        let visual = self
            .scene
            .instantiate()
            .and_then(|node| node.try_cast::<Node3D>().ok())
            .ok_or("Creature template did not instantiate")?;
        set_owner(&visual, None);
        drop(span);
        let span = crate::profile::span(|| "template.bones".to_owned());
        if let Err(error) = self
            .bones
            .iter()
            .try_for_each(|bones| bones.add_to(&visual))
        {
            visual.free();
            return Err(error);
        }
        drop(span);
        let _span = crate::profile::span(|| "template.rust_nodes".to_owned());
        if let Err(error) = self.nodes.iter().try_for_each(|node| node.attach(&visual)) {
            visual.free();
            return Err(error);
        }
        Ok(visual)
    }
}

/// Takes the bones off every skeleton under `built`.
fn take_bones(built: &Gd<Node3D>) -> Vec<SkeletonBones> {
    built
        .find_children_ex("*")
        .type_("Skeleton3D")
        .owned(false)
        .done()
        .iter_shared()
        .filter_map(|node| node.try_cast::<Skeleton3D>().ok())
        .map(|mut skeleton| {
            let bones = (0..skeleton.get_bone_count())
                .map(|bone| Bone {
                    name: skeleton.get_bone_name(bone),
                    parent: skeleton.get_bone_parent(bone),
                    rest: skeleton.get_bone_rest(bone),
                    position: skeleton.get_bone_pose_position(bone),
                    rotation: skeleton.get_bone_pose_rotation(bone),
                    scale: skeleton.get_bone_pose_scale(bone),
                })
                .collect();
            skeleton.clear_bones();
            SkeletonBones {
                path: built.get_path_to(&skeleton),
                bones,
            }
        })
        .collect()
}

impl SkeletonBones {
    /// Adds the bones to the skeleton at `path` under `visual`, which has none.
    fn add_to(&self, visual: &Gd<Node3D>) -> Result<(), String> {
        let mut skeleton = visual
            .get_node_or_null(&self.path)
            .and_then(|node| node.try_cast::<Skeleton3D>().ok())
            .ok_or_else(|| format!("Creature template lost skeleton {}", self.path))?;
        for bone in &self.bones {
            skeleton.add_bone(&bone.name);
        }
        for (index, bone) in (0..).zip(&self.bones) {
            skeleton.set_bone_parent(index, bone.parent);
            skeleton.set_bone_rest(index, bone.rest);
            skeleton.set_bone_pose_position(index, bone.position);
            skeleton.set_bone_pose_rotation(index, bone.rotation);
            skeleton.set_bone_pose_scale(index, bone.scale);
        }
        Ok(())
    }
}

/// Sets the owner of every node under `root`.
fn set_owner(root: &Gd<Node3D>, owner: Option<Gd<Node>>) {
    for mut node in root.find_children_ex("*").owned(false).done().iter_shared() {
        match &owner {
            Some(owner) => node.set_owner(owner),
            None => node.set_owner(Gd::null_arg()),
        }
    }
}

/// Detaches the Rust nodes of `built` and returns how to make them again and each detached
/// node with its parent and index, in tree order. `built`'s materials that animate on global
/// sequences only become shared (`WowMaterialAnimation::share_global_materials`).
fn take_rust_nodes(
    built: &Gd<Node3D>,
) -> Result<(Vec<RustNode>, Vec<(Gd<Node>, Gd<Node>, i32)>), String> {
    let mut nodes = Vec::new();
    let mut found = Vec::new();
    for original in built
        .find_children_ex("*")
        .owned(false)
        .done()
        .iter_shared()
    {
        let class = original.get_class();
        if ClassDb::singleton().class_get_api_type(&StringName::from(&class)) == ApiType::EXTENSION
        {
            found.push(original);
        }
    }
    let mut detached = Vec::new();
    for original in found {
        let mut parent = original
            .get_parent()
            .ok_or("Creature visual Rust node has no parent")?;
        let root = built.get_path_to(&parent);
        model_of(built, &root)?;
        let index = original.get_index();
        if let Ok(mut animation) = original.clone().try_cast::<WowMaterialAnimation>() {
            let shared = animation.bind_mut().share_global_materials();
            nodes.push(RustNode::MaterialAnimation {
                root,
                name: original.get_name(),
                local: animation.bind().local_materials(),
                shared,
            });
        } else {
            nodes.push(rust_node(built, root, original.clone())?);
        }
        parent.remove_child(&original);
        detached.push((parent, original, index));
    }
    // Reattached in reverse, each lands at the index it had when it was detached.
    detached.reverse();
    Ok((nodes, detached))
}

fn rust_node(built: &Gd<Node3D>, root: NodePath, original: Gd<Node>) -> Result<RustNode, String> {
    let name = original.get_name();
    let class = original.get_class();
    if let Ok(player) = original.clone().try_cast::<WowAnimationPlayer>() {
        let skeleton = player
            .bind()
            .skeleton()
            .ok_or("Creature animation player has no skeleton")?;
        return Ok(RustNode::Animation {
            root,
            name,
            skeleton: built.get_path_to(&skeleton),
        });
    }
    if original.clone().try_cast::<WowM2Lights>().is_ok() {
        return Ok(RustNode::Lights { root, name });
    }
    Err(format!("Creature visual holds an unsupported {class} node"))
}

impl RustNode {
    fn root(&self) -> &NodePath {
        match self {
            Self::Animation { root, .. }
            | Self::MaterialAnimation { root, .. }
            | Self::Lights { root, .. } => root,
        }
    }

    fn attach(&self, visual: &Gd<Node3D>) -> Result<(), String> {
        let _span = crate::profile::span(|| {
            match self {
                Self::Animation { .. } => "template.animation",
                Self::MaterialAnimation { .. } => "template.material_animation",
                Self::Lights { .. } => "template.lights",
            }
            .to_owned()
        });
        let mut root = visual
            .get_node_or_null(self.root())
            .ok_or_else(|| format!("Creature template lost model root {}", self.root()))?;
        let model = model_of(visual, self.root())?;
        match self {
            Self::Animation { name, skeleton, .. } => {
                let skeleton = visual
                    .get_node_or_null(skeleton)
                    .and_then(|node| node.try_cast::<Skeleton3D>().ok())
                    .ok_or_else(|| format!("Creature template lost skeleton {skeleton}"))?;
                let mut player = WowAnimationPlayer::from_model(&model.model, skeleton)?;
                player.set_name(name);
                root.add_child(&player);
                player.bind_mut().apply_sequence_bounds();
            }
            Self::MaterialAnimation {
                name,
                local,
                shared,
                ..
            } => {
                let copies: Vec<_> = local
                    .iter()
                    .map(|(material, binding)| {
                        let copy = copy_material(material);
                        replace_material(visual, material, &copy);
                        (copy, binding.clone())
                    })
                    .collect();
                let mut animation = WowMaterialAnimation::with_shared(
                    MaterialTracks::of(&model.model),
                    copies,
                    shared,
                );
                animation.set_name(name);
                root.add_child(&animation);
            }
            Self::Lights { name, .. } => {
                let mut lights = WowM2Lights::from_model(&model.model)
                    .ok_or("Creature template's model lights have no point light")?;
                lights.set_name(name);
                root.add_child(&lights);
            }
        }
        Ok(())
    }
}

/// The parsed model of the model root at `root` under `visual` (`M2_SOURCE_META`).
fn model_of(visual: &Gd<Node3D>, root: &NodePath) -> Result<Arc<CachedModel>, String> {
    let node = visual
        .get_node_or_null(root)
        .ok_or_else(|| format!("Creature template lost model root {root}"))?;
    let source = node.get_meta(M2_SOURCE_META).to::<GString>().to_string();
    model_at(Path::new(&source)).ok_or_else(|| format!("Model {source} is not parsed"))
}

/// Binds `copy` instead of `material` on every batch under `visual`.
fn replace_material(visual: &Gd<Node3D>, material: &Gd<ShaderMaterial>, copy: &Gd<ShaderMaterial>) {
    for node in visual
        .find_children_ex("*")
        .type_("MeshInstance3D")
        .owned(false)
        .done()
        .iter_shared()
    {
        let mut mesh = node.cast::<MeshInstance3D>();
        if mesh
            .get_material_override()
            .is_some_and(|bound| bound.instance_id() == material.instance_id())
        {
            mesh.set_material_override(copy);
        }
    }
}
