//! Explicitly resolved geometry resources. No filename/FDID guessing lives here.
use crate::assets::material::{SCENERY_FADE_SHADER_META, shared_shader};
use crate::particles::ModelParticles;
use game_engine_core::{
    m2::ParticleEmitter,
    m2_particles::{EmitterSim, ModelParticle},
};
use godot::{
    classes::{
        Mesh, MeshInstance3D, MultiMesh, MultiMeshInstance3D, Node3D, ShaderMaterial,
        multi_mesh::TransformFormat,
    },
    prelude::*,
};
use std::rc::Rc;

pub(crate) struct GeometryModel {
    batches: Vec<(Gd<Mesh>, Gd<ShaderMaterial>)>,
}

impl GeometryModel {
    /// Consume resources from a separately loaded model; caller retains asset identity.
    pub fn from_node(node: &Gd<Node3D>) -> Result<Rc<Self>, String> {
        let mut batches = Vec::new();
        for child in node.get_children().iter_shared() {
            let Ok(mesh) = child.try_cast::<MeshInstance3D>() else {
                continue;
            };
            let geometry = mesh
                .get_mesh()
                .ok_or("particle geometry batch lacks mesh")?;
            let material = mesh
                .get_material_override()
                .ok_or("particle geometry batch lacks material")?
                .try_cast::<ShaderMaterial>()
                .map_err(|_| "particle geometry requires authored ShaderMaterial")?;
            batches.push((geometry, geometry_material(&material)?));
        }
        if batches.is_empty() {
            return Err("particle geometry model has no mesh batches".into());
        }
        Ok(Rc::new(Self { batches }))
    }
}

/// Multiply lifetime RGBA without changing the model's own combiner or lighting.
fn geometry_material(authored: &Gd<ShaderMaterial>) -> Result<Gd<ShaderMaterial>, String> {
    let shader = if authored.has_meta(SCENERY_FADE_SHADER_META) {
        authored
            .get_meta(SCENERY_FADE_SHADER_META)
            .to::<Gd<godot::classes::Shader>>()
    } else {
        authored
            .get_shader()
            .ok_or("particle geometry material has no shader")?
    };
    let source = shader.get_code().to_string();
    let source = if source.contains("vec4 mesh = vec4(mesh_color, transparency)") {
        source
            .replace(
                "void vertex() {",
                "varying vec4 particle_rgba;\nvoid vertex() {\nparticle_rgba = COLOR;",
            )
            .replace(
                "vec4(mesh_color, transparency)",
                "vec4(mesh_color, transparency) * particle_rgba",
            )
            .replace(
                "ALPHA = scenery_opacity;",
                "ALPHA = particle_rgba.a * scenery_opacity;",
            )
    } else {
        return Err("particle geometry material is not an M2 model shader".into());
    };
    let mut material = authored
        .duplicate()
        .ok_or("cannot duplicate particle geometry material")?
        .try_cast::<ShaderMaterial>()
        .map_err(|_| "particle geometry duplicate has wrong type")?;
    material.set_shader(&shared_shader(&source));
    Ok(material)
}

/// The mapping is supplied explicitly by a product-qualified caller (or synthetic fixture).
#[derive(Default, Clone)]
pub(crate) struct ResolvedParticleModels {
    pub geometry: Option<Rc<GeometryModel>>,
    pub recursive: Vec<(Rc<ModelParticles>, bool)>,
}

pub(crate) struct GeometryParticles {
    nodes: Vec<Gd<MultiMeshInstance3D>>,
}

impl GeometryParticles {
    pub fn new(asset: &GeometryModel, parent: &mut Gd<Node3D>) -> Self {
        let nodes = asset
            .batches
            .iter()
            .map(|(mesh, material)| {
                let mut multimesh = MultiMesh::new_gd();
                multimesh.set_transform_format(TransformFormat::TRANSFORM_3D);
                multimesh.set_use_colors(true);
                multimesh.set_mesh(mesh);
                let mut node = MultiMeshInstance3D::new_alloc();
                node.set_name("GeometryParticles");
                node.set_as_top_level(true);
                node.set_multimesh(&multimesh);
                node.set_material_override(material);
                parent.add_child(&node);
                node
            })
            .collect();
        Self { nodes }
    }

    pub fn draw(&mut self, sim: &EmitterSim, emitter: &ParticleEmitter, fade: f32) -> usize {
        let mut particles = Vec::new();
        sim.model_particles(emitter, &mut particles);
        for node in &mut self.nodes {
            let mut mesh = node.get_multimesh().expect("geometry particle multimesh");
            mesh.set_instance_count(particles.len() as i32);
            for (index, particle) in particles.iter().enumerate() {
                mesh.set_instance_transform(index as i32, &transform(particle));
                let c = particle.color;
                mesh.set_instance_color(index as i32, Color::from_rgba(c.x, c.y, c.z, c.w * fade));
            }
            mesh.set_visible_instance_count(particles.len() as i32);
        }
        particles.len()
    }
}

fn transform(particle: &ModelParticle) -> Transform3D {
    let m = particle.transform;
    let vector = |v: glam::Vec4| Vector3::new(v.x, v.y, v.z);
    Transform3D::new(
        Basis::from_cols(vector(m.x_axis), vector(m.y_axis), vector(m.z_axis)),
        vector(m.w_axis),
    )
}

impl Drop for GeometryParticles {
    fn drop(&mut self) {
        for node in self.nodes.drain(..) {
            if node.is_instance_valid() {
                node.free();
            }
        }
    }
}
