//! A world creature visual kept per display and gear: a later unit with both instantiates
//! it instead of building it again. A Stormwind guard's build (four models, 107 batches, their
//! skeletons, players, composited textures and equipment) took 10-15 ms of one main-thread
//! frame each time a guard came into view; its instance shares the meshes, skins and
//! static materials and makes no material. The template is a `PackedScene`: instantiating
//! the guard's 205 nodes took 3-5 ms where `Node::duplicate`, which copies each property
//! through the property list, took 16-31 ms (loaded Linux host, debug library).
//!
//! A packed scene cannot hold a Rust node's state (and `WowM2Lights` has no default
//! constructor), so the template holds none: each instance gets its own animation player, material animation and model lights,
//! built from the parsed model as `build_model` builds them. A material that animates on
//! its model's sequence timeline follows its own instance's animation, so each instance
//! copies it; one that animates on global sequences only samples alike in every model, so
//! the material clock animates it once for all of them (`WowMaterialClock::share`).

use std::{path::Path, sync::Arc};

use game_engine_core::m2_material::{BatchBinding, MaterialTracks};
use godot::{
    classes::{
        ClassDb, Engine, MeshInstance3D, Node3D, PackedScene, SceneTree, ShaderMaterial,
        Skeleton3D, class_db::ApiType,
    },
    prelude::*,
};

use super::{
    M2_SOURCE_META,
    creature::{CachedModel, CreatureGear, model_at},
    m2_lights::WowM2Lights,
    material::copy_material,
    uv_animation::{WowMaterialAnimation, WowMaterialClock},
};
use crate::animation::WowAnimationPlayer;

/// A built creature visual without its Rust nodes, and how to make them again.
pub(crate) struct CreatureTemplate {
    display_id: u32,
    gear: CreatureGear,
    scene: Gd<PackedScene>,
    nodes: Vec<RustNode>,
    /// The clock animating the template's materials that animate on global sequences.
    clock: Gd<WowMaterialClock>,
    shared: Vec<InstanceId>,
}

/// Materials of one model that animate on global sequences only, with the model's tracks.
type GlobalMaterials = (Arc<MaterialTracks>, Vec<(Gd<ShaderMaterial>, BatchBinding)>);

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
        materials: Vec<(Gd<ShaderMaterial>, BatchBinding)>,
    },
    Lights {
        root: NodePath,
        name: StringName,
    },
}

impl CreatureTemplate {
    /// The template of `built`, a creature visual not yet in the scene tree. The material
    /// clock animates `built`'s materials that animate on global sequences from now on.
    pub(crate) fn of(
        display_id: u32,
        gear: &CreatureGear,
        built: &Gd<Node3D>,
    ) -> Result<Self, String> {
        let mut clock = material_clock()?;
        let (nodes, global, detached) = take_rust_nodes(built)?;
        // Packing saves the nodes `built` owns; they own nothing again afterwards.
        set_owner(built, Some(built.clone().upcast()));
        let mut scene = PackedScene::new_gd();
        let packed = scene.pack(built);
        set_owner(built, None);
        for (mut parent, node, index) in detached {
            parent.add_child(&node);
            parent.move_child(&node, index);
        }
        if packed != godot::global::Error::OK {
            return Err(format!("Creature visual did not pack: {packed:?}"));
        }
        let mut shared = Vec::new();
        for (tracks, materials) in global {
            shared.extend(materials.iter().map(|(material, _)| material.instance_id()));
            clock.bind_mut().share(&tracks, materials);
        }
        Ok(Self {
            display_id,
            gear: gear.clone(),
            scene,
            nodes,
            clock,
            shared,
        })
    }

    pub(crate) fn matches(&self, display_id: u32, gear: &CreatureGear) -> bool {
        self.display_id == display_id && self.gear == *gear
    }

    /// A new visual like the one this template was made from.
    pub(crate) fn instance(&self) -> Result<Gd<Node3D>, String> {
        let span = crate::profile::span(|| "template.instantiate".to_owned());
        let visual = self
            .scene
            .instantiate()
            .and_then(|node| node.try_cast::<Node3D>().ok())
            .ok_or("Creature template did not instantiate")?;
        set_owner(&visual, None);
        drop(span);
        let _span = crate::profile::span(|| "template.rust_nodes".to_owned());
        if let Err(error) = self.nodes.iter().try_for_each(|node| node.attach(&visual)) {
            visual.free();
            return Err(error);
        }
        Ok(visual)
    }
}

impl Drop for CreatureTemplate {
    fn drop(&mut self) {
        if self.clock.is_instance_valid() {
            self.clock.bind_mut().unshare(&self.shared);
        }
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

fn material_clock() -> Result<Gd<WowMaterialClock>, String> {
    Engine::singleton()
        .get_main_loop()
        .and_then(|main| main.try_cast::<SceneTree>().ok())
        .and_then(|tree| {
            tree.get_root()
                .try_get_node_as::<WowMaterialClock>("M2MaterialClock")
        })
        .ok_or_else(|| "Creature templates require /root/M2MaterialClock".to_owned())
}

/// Detaches the Rust nodes of `built` and returns how to make them again, `built`'s
/// materials that animate on global sequences only, and each detached node with its parent
/// and index, in tree order. Those materials leave `built`'s material animation, and one
/// left with none is freed instead of detached.
fn take_rust_nodes(
    built: &Gd<Node3D>,
) -> Result<
    (
        Vec<RustNode>,
        Vec<GlobalMaterials>,
        Vec<(Gd<Node>, Gd<Node>, i32)>,
    ),
    String,
> {
    let mut nodes = Vec::new();
    let mut global = Vec::new();
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
        let model = model_of(built, &root)?;
        let index = original.get_index();
        if let Ok(mut animation) = original.clone().try_cast::<WowMaterialAnimation>() {
            let materials = animation.bind_mut().take_global_materials();
            global.push((Arc::new(MaterialTracks::of(&model.model)), materials));
            let local = animation.bind().materials();
            parent.remove_child(&animation);
            if local.is_empty() {
                animation.free();
                continue;
            }
            nodes.push(RustNode::MaterialAnimation {
                root,
                name: original.get_name(),
                materials: local,
            });
        } else {
            nodes.push(rust_node(built, root, original.clone())?);
            parent.remove_child(&original);
        }
        detached.push((parent, original, index));
    }
    // Reattached lowest index first, each lands at its own index.
    detached.reverse();
    Ok((nodes, global, detached))
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
                name, materials, ..
            } => {
                let copies: Vec<_> = materials
                    .iter()
                    .map(|(material, binding)| {
                        let copy = copy_material(material);
                        replace_material(visual, material, &copy);
                        (copy, binding.clone())
                    })
                    .collect();
                let mut animation = WowMaterialAnimation::from_batches(
                    MaterialTracks::of(&model.model),
                    copies.into_iter(),
                )
                .ok_or("Creature template's material animation has no animated material")?;
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
