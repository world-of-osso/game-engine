//! Opt-in model annotations and independent spring state for each placed tree.

use std::{cell::RefCell, collections::HashMap, fs, path::Path, sync::Arc};

use game_engine_core::elastic_tree::{
    BranchState, Capsule, Contact, TreeAnnotation, parse_annotation, sweep_capsule,
};
use glam::Vec3;
use godot::{
    classes::{
        CapsuleShape3D, CollisionShape3D, INode3D, MeshInstance3D, Node3D, ProjectSettings,
        StaticBody3D,
    },
    prelude::*,
};

use super::{doodad_collision::DOODAD_LAYER, tree_render};

/// Separate from ordinary doodad camera collision; queried only by airborne movement.
pub(crate) const TREE_LAYER: u32 = 1 << 4;
const TREE_NODE: &str = "ElasticTree";
const RESISTANCE_STIFFNESS: f32 = 100.0;
/// Prototype mount momentum relative to normalized branch inertia.
const MOUNT_IMPULSE: f32 = 60.0;

thread_local! {
    static ANNOTATIONS: RefCell<HashMap<u32, Result<Option<Arc<TreeAnnotation>>, String>>> = RefCell::new(HashMap::new());
}

/// Numeric cached M2 paths identify the model, never its world placement.
pub(crate) fn load_annotation(model_path: &GString) -> Result<Option<Arc<TreeAnnotation>>, String> {
    let path = model_path.to_string();
    let model = Path::new(&path)
        .file_stem()
        .and_then(|name| name.to_str())
        .and_then(|name| name.parse::<u32>().ok());
    let Some(fdid) = model else {
        return Ok(None);
    };
    if let Some(cached) = ANNOTATIONS.with_borrow(|cache| cache.get(&fdid).cloned()) {
        return cached;
    }
    let loaded = read_annotation(fdid);
    ANNOTATIONS.with_borrow_mut(|cache| cache.insert(fdid, loaded.clone()));
    loaded
}

fn read_annotation(fdid: u32) -> Result<Option<Arc<TreeAnnotation>>, String> {
    let path = ProjectSettings::singleton()
        .globalize_path(&format!("res://trees/{fdid}.json"))
        .to_string();
    let json = match fs::read_to_string(&path) {
        Ok(json) => json,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot read tree annotation {path}: {error}")),
    };
    let annotation =
        parse_annotation(&json).map_err(|error| format!("Tree annotation {path}: {error}"))?;
    if annotation.model_fdid != fdid {
        return Err(format!(
            "Tree annotation {path} names model {} instead of {fdid}",
            annotation.model_fdid
        ));
    }
    Ok(Some(Arc::new(annotation)))
}

#[derive(GodotClass)]
#[class(base = Node3D, no_init)]
pub struct WowElasticTree {
    annotation: Arc<TreeAnnotation>,
    branches: Vec<BranchState>,
    bodies: Vec<Gd<StaticBody3D>>,
    batches: Vec<Gd<MeshInstance3D>>,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WowElasticTree {
    fn physics_process(&mut self, delta: f64) {
        for (state, branch) in self.branches.iter_mut().zip(&self.annotation.branches) {
            state.advance(branch, delta as f32);
        }
        self.update_bends();
    }
}

#[godot_api]
impl WowElasticTree {
    /// Authoring/debug scene entry point; uses exactly the production airborne contact path.
    #[func]
    fn move_airborne(
        node: Gd<Node3D>,
        from: Vector3,
        to: Vector3,
        radius: f32,
        delta: f32,
    ) -> Vector3 {
        let space = node
            .get_world_3d()
            .and_then(|world| world.get_direct_space_state());
        match space {
            Some(space) => Vector3::from_array(
                super::tree_contact::move_airborne(
                    &space,
                    from.to_array().into(),
                    to.to_array().into(),
                    radius,
                    delta,
                )
                .to_array(),
            ),
            None => {
                godot_error!("Elastic tree has no physics space");
                from
            }
        }
    }

    /// Local rotation plus world-space branch endpoints, for annotation tuning and fixtures.
    #[func]
    fn branch_state(&self) -> VarArray {
        let mut result = VarArray::new();
        for (index, (state, branch)) in self
            .branches
            .iter()
            .zip(&self.annotation.branches)
            .enumerate()
        {
            let capsule = self.branch_capsule(index);
            let mut entry = VarDictionary::new();
            entry.set("name", branch.name.as_str());
            entry.set("rotation", Vector3::from_array(state.rotation.to_array()));
            entry.set(
                "pivot",
                self.base().to_global(Vector3::from_array(capsule.start)),
            );
            entry.set(
                "tip",
                self.base().to_global(Vector3::from_array(capsule.end)),
            );
            entry.set("radius", branch.radius * self.uniform_scale());
            entry.set("stiffness", branch.stiffness);
            result.push(&entry.to_variant());
        }
        result
    }
}

pub(crate) struct TreeHit {
    pub contact: Contact,
    /// None denotes the fixed trunk; otherwise the authored branch index.
    pub branch: Option<usize>,
}

impl WowElasticTree {
    pub(crate) fn attach(
        root: &mut Gd<Node3D>,
        annotation: Arc<TreeAnnotation>,
    ) -> Result<(), String> {
        let batches = tree_render::prepare_batches(root, &annotation)?;
        let bodies = build_bodies(&annotation);
        let branches = annotation
            .branches
            .iter()
            .map(|_| BranchState::default())
            .collect();
        let mut node = Gd::from_init_fn(|base| Self {
            annotation,
            branches,
            bodies,
            batches,
            base,
        });
        node.set_name(TREE_NODE);
        let bodies = node.bind().bodies.clone();
        for body in bodies {
            node.add_child(&body);
        }
        root.add_child(&node);
        Ok(())
    }

    pub(crate) fn sweep(
        &self,
        from: Vec3,
        to: Vec3,
        radius: f32,
        excluded: &[usize],
    ) -> Option<TreeHit> {
        let from = self.local_point(from);
        let to = self.local_point(to);
        let radius = radius / self.uniform_scale();
        self.sweep_local(from, to, radius, excluded)
            .map(|hit| self.world_hit(hit))
    }

    fn local_point(&self, point: Vec3) -> Vec3 {
        Vec3::from(
            self.base()
                .to_local(Vector3::from_array(point.to_array()))
                .to_array(),
        )
    }

    fn sweep_local(
        &self,
        from: Vec3,
        to: Vec3,
        radius: f32,
        excluded: &[usize],
    ) -> Option<TreeHit> {
        let trunk =
            sweep_capsule(from, to, radius, &self.annotation.trunk).map(|contact| TreeHit {
                contact,
                branch: None,
            });
        let branches = (0..self.branches.len())
            .filter(|index| !excluded.contains(index))
            .filter_map(|index| {
                sweep_capsule(from, to, radius, &self.branch_capsule(index)).map(|contact| {
                    TreeHit {
                        contact,
                        branch: Some(index),
                    }
                })
            });
        trunk
            .into_iter()
            .chain(branches)
            .min_by(|a, b| a.contact.fraction.total_cmp(&b.contact.fraction))
    }

    fn world_hit(&self, mut hit: TreeHit) -> TreeHit {
        let point = self
            .base()
            .to_global(Vector3::from_array(hit.contact.point.to_array()));
        let normal =
            self.base().get_global_basis() * Vector3::from_array(hit.contact.normal.to_array());
        hit.contact.point = Vec3::from(point.to_array());
        hit.contact.normal = Vec3::from(normal.to_array()).normalize_or_zero();
        hit
    }

    pub(crate) fn apply_hit(&mut self, hit: &TreeHit, motion: Vec3) -> f32 {
        let Some(index) = hit.branch else {
            return 1.0;
        };
        let point = self
            .base()
            .to_local(Vector3::from_array(hit.contact.point.to_array()));
        let local_motion =
            self.base().get_global_basis().inverse() * Vector3::from_array(motion.to_array());
        let branch = &self.annotation.branches[index];
        self.branches[index].apply_contact(
            branch,
            Vec3::from(point.to_array()),
            Vec3::from(local_motion.to_array()) * MOUNT_IMPULSE,
        );
        // Resistance is shared with spring stiffness: thin limbs yield rather than act as walls.
        let resistance = branch.stiffness / (branch.stiffness + RESISTANCE_STIFFNESS);
        self.update_bends();
        resistance
    }

    fn uniform_scale(&self) -> f32 {
        self.base().get_global_basis().col_a().length()
    }

    fn branch_capsule(&self, index: usize) -> Capsule {
        self.branches[index].collider(&self.annotation.branches[index])
    }

    fn update_bends(&mut self) {
        let rotations: Vec<_> = self.branches.iter().map(|state| state.rotation).collect();
        tree_render::update_bends(&mut self.batches, &rotations);
        for index in 0..self.branches.len() {
            let capsule = self.branch_capsule(index);
            self.bodies[index + 1].set_transform(capsule_transform(&capsule));
        }
    }
}

fn build_bodies(annotation: &TreeAnnotation) -> Vec<Gd<StaticBody3D>> {
    std::iter::once(build_body(&annotation.trunk, "Trunk"))
        .chain(
            annotation
                .branches
                .iter()
                .enumerate()
                .map(|(index, branch)| {
                    build_body(
                        &Capsule {
                            start: branch.pivot,
                            end: branch.tip,
                            radius: branch.radius,
                        },
                        &format!("Branch{index}"),
                    )
                }),
        )
        .collect()
}

fn build_body(capsule: &Capsule, name: &str) -> Gd<StaticBody3D> {
    let mut shape = CapsuleShape3D::new_gd();
    shape.set_radius(capsule.radius);
    shape.set_height(
        Vec3::from(capsule.start).distance(Vec3::from(capsule.end)) + 2.0 * capsule.radius,
    );
    let mut collision = CollisionShape3D::new_alloc();
    collision.set_shape(&shape);
    let mut body = StaticBody3D::new_alloc();
    body.set_name(name);
    body.set_collision_layer(TREE_LAYER | DOODAD_LAYER);
    body.set_collision_mask(0);
    body.set_transform(capsule_transform(capsule));
    body.add_child(&collision);
    body
}

fn capsule_transform(capsule: &Capsule) -> Transform3D {
    let start = Vec3::from(capsule.start);
    let end = Vec3::from(capsule.end);
    let rotation = glam::Quat::from_rotation_arc(Vec3::Y, (end - start).normalize());
    Transform3D::new(
        Basis::from_quaternion(Quaternion::new(
            rotation.x, rotation.y, rotation.z, rotation.w,
        )),
        Vector3::from_array(((start + end) * 0.5).to_array()),
    )
}
