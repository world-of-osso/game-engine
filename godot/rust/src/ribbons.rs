//! Native source M2 ribbon strips; model bones supply world-space emitter frames.
use crate::{animation::WowAnimationPlayer, assets::material};
use game_engine_core::{
    asset::m2_format::m2_ribbon::M2RibbonEmitter,
    m2,
    m2_material::{self, MaterialTime, MaterialTracks, TextureMatrix},
    m2_ribbons::{self, RibbonFrame, RibbonTrail, RibbonVertex},
};
use godot::{
    classes::{
        ArrayMesh, MeshInstance3D, Node3D, ShaderMaterial, Skeleton3D,
        geometry_instance_3d::ShadowCastingSetting, mesh,
    },
    prelude::*,
};
use std::path::Path;

struct PlacedRibbon {
    source: M2RibbonEmitter,
    trail: RibbonTrail,
    meshes: Vec<Gd<MeshInstance3D>>,
    materials: Vec<(Gd<ShaderMaterial>, Option<usize>)>,
    geometry: Gd<ArrayMesh>,
}

pub(crate) struct PlacedRibbons {
    ribbons: Vec<PlacedRibbon>,
    skeleton: Gd<Skeleton3D>,
    player: Option<Gd<WowAnimationPlayer>>,
    pivots: Vec<Vector3>,
    global_sequences: Vec<u32>,
    material_tracks: MaterialTracks,
    clock_ms: f64,
}

impl PlacedRibbons {
    pub(crate) fn place(
        model: &m2::Model,
        root: &Gd<Node3D>,
        texture_dir: &Path,
    ) -> Result<Option<Self>, String> {
        if model.ribbon_emitters.is_empty() {
            return Ok(None);
        }
        let skeleton = root
            .try_get_node_as::<Skeleton3D>("Skeleton3D")
            .ok_or("Ribbon model has no skeleton")?;
        let player = root.try_get_node_as::<WowAnimationPlayer>("M2Animation");
        if !model.sequences.is_empty() && player.is_none() {
            return Err("Ribbon model is missing its animation player".into());
        }
        let ribbons = model
            .ribbon_emitters
            .iter()
            .map(|source| place_ribbon(model, source, root, texture_dir))
            .collect::<Result<_, _>>()?;
        Ok(Some(Self {
            ribbons,
            skeleton,
            player,
            pivots: model
                .bones
                .iter()
                .map(|bone| wow_vector(bone.pivot))
                .collect(),
            global_sequences: model.global_sequences.clone(),
            material_tracks: MaterialTracks::of(model),
            clock_ms: 0.0,
        }))
    }

    pub(crate) fn update_and_draw(
        &mut self,
        root: &Gd<Node3D>,
        delta: f32,
        fade: f32,
        emitting: bool,
    ) -> Result<(), String> {
        self.clock_ms += f64::from(delta) * 1000.0;
        let (sequence, time_ms, global_ms) = self.playback()?;
        let time = m2::AnimTime {
            sequence,
            time_ms,
            global_ms,
            global_sequences: &self.global_sequences,
        };
        for ribbon in &mut self.ribbons {
            let frame = read_emitter_frame(root, &self.skeleton, &self.pivots, &ribbon.source)?;
            let values = m2_ribbons::sample(&ribbon.source, &time);
            ribbon.trail.advance(delta, frame, values, emitting)?;
            update_texture_transforms(
                &mut ribbon.materials,
                &self.material_tracks,
                MaterialTime {
                    sequence,
                    time_ms,
                    global_ms: global_ms as u32,
                },
            );
            let vertices = ribbon.trail.triangles(fade);
            upload_geometry(&mut ribbon.geometry, &vertices);
            for mesh in &mut ribbon.meshes {
                mesh.set_visible(!vertices.is_empty());
            }
        }
        Ok(())
    }

    fn playback(&self) -> Result<(usize, u32, f64), String> {
        let Some(player) = &self.player else {
            return Ok((0, 0, self.clock_ms)); // Source model has no animation sequences.
        };
        if !player.is_instance_valid() {
            return Err("Ribbon animation player disappeared".into());
        }
        let player = player.bind();
        let (sequence, time_ms) = player
            .playback()
            .ok_or("Ribbon animation has no playback")?;
        let global_ms = player
            .global_ms()
            .ok_or("Ribbon animation has no global clock")?;
        Ok((sequence, time_ms, global_ms))
    }
}

fn place_ribbon(
    model: &m2::Model,
    source: &M2RibbonEmitter,
    root: &Gd<Node3D>,
    texture_dir: &Path,
) -> Result<PlacedRibbon, String> {
    validate_ribbon_references(model, source)?;
    let trail = RibbonTrail::new(source)?;
    let materials = source
        .texture_indices
        .iter()
        .zip(&source.material_indices)
        .enumerate()
        .map(|(pass, (&texture, &material))| {
            let transform = m2_ribbons::texture_transform_index(model, source, pass)?;
            let material =
                load_ribbon_material(model, source.priority_plane, texture, material, texture_dir)?;
            Ok((material, transform))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let geometry = ArrayMesh::new_gd();
    let meshes = materials
        .iter()
        .map(|(material, _)| spawn_ribbon_mesh(root, &geometry, material))
        .collect();
    Ok(PlacedRibbon {
        source: source.clone(),
        trail,
        meshes,
        materials,
        geometry,
    })
}

fn read_emitter_frame(
    root: &Gd<Node3D>,
    skeleton: &Gd<Skeleton3D>,
    pivots: &[Vector3],
    source: &M2RibbonEmitter,
) -> Result<RibbonFrame, String> {
    let bone =
        i32::try_from(source.bone_index).map_err(|_| "Ribbon bone index exceeds engine range")?;
    if bone >= skeleton.get_bone_count() {
        return Err(format!("Ribbon bone{bone} missing from skeleton"));
    }
    let pose = skeleton.get_bone_global_pose(bone)
        * Transform3D::IDENTITY.translated(-pivots[bone as usize]);
    let transform = root.get_global_transform()
        * pose
        * Transform3D::IDENTITY.translated(wow_vector(source.position));
    Ok(RibbonFrame {
        position: glam::Vec3::from_array(transform.origin.to_array()),
        direction: glam::Vec3::from_array((transform.basis * Vector3::UP).to_array()),
        vertical: glam::Vec3::from_array((transform.basis * -Vector3::BACK).to_array()),
    })
}

fn validate_ribbon_references(model: &m2::Model, source: &M2RibbonEmitter) -> Result<(), String> {
    if source.bone_index as usize >= model.bones.len() {
        return Err(format!(
            "Ribbon bone{} missing from model",
            source.bone_index
        ));
    }
    if source.texture_indices.len() != source.material_indices.len()
        || source.material_indices.is_empty()
    {
        return Err(format!(
            "Ribbon{} has unsupported texture/material reference counts",
            source.id
        ));
    }
    Ok(())
}

fn spawn_ribbon_mesh(
    root: &Gd<Node3D>,
    geometry: &Gd<ArrayMesh>,
    material: &Gd<ShaderMaterial>,
) -> Gd<MeshInstance3D> {
    let mut mesh = MeshInstance3D::new_alloc();
    mesh.set_name("M2RibbonStrip");
    mesh.set_mesh(geometry);
    mesh.set_material_override(material);
    mesh.set_cast_shadows_setting(ShadowCastingSetting::OFF);
    root.clone().add_child(&mesh);
    mesh.set_as_top_level(true);
    mesh.set_global_transform(Transform3D::IDENTITY);
    mesh.set_visible(false);
    mesh
}

fn update_texture_transforms(
    materials: &mut [(Gd<ShaderMaterial>, Option<usize>)],
    tracks: &MaterialTracks,
    time: MaterialTime,
) {
    for (material, index) in materials {
        let matrix = index.map_or(m2_material::IDENTITY_MATRIX, |index| {
            m2_material::texture_matrix(&tracks.transforms[index], tracks, time)
        });
        apply_texture_transform(material, matrix);
    }
}

fn apply_texture_transform(material: &mut Gd<ShaderMaterial>, matrix: TextureMatrix) {
    let linear = Vector4::new(matrix[0], matrix[1], matrix[2], matrix[3]);
    let offset = Vector2::new(matrix[4], matrix[5]);
    material.set_shader_parameter("uv_matrix", &linear.to_variant());
    material.set_shader_parameter("uv_offset", &offset.to_variant());
}

fn load_ribbon_material(
    model: &m2::Model,
    priority: i16,
    texture_index: u16,
    material_index: u16,
    dir: &Path,
) -> Result<Gd<ShaderMaterial>, String> {
    let texture = *model
        .texture_fdids
        .get(usize::from(texture_index))
        .ok_or_else(|| format!("Ribbon texture index{texture_index} missing"))?;
    let authored = model
        .materials
        .get(usize::from(material_index))
        .ok_or_else(|| format!("Ribbon material index{material_index} missing"))?;
    let mut missing = PackedInt32Array::new();
    let image = material::shared_texture(texture, dir, &mut missing)?
        .ok_or_else(|| format!("Ribbon texture{texture} has no verified local image"))?;
    create_ribbon_material(authored, &image, priority)
}

fn create_ribbon_material(
    authored: &m2::Material,
    image: &Gd<godot::classes::ImageTexture>,
    priority: i16,
) -> Result<Gd<ShaderMaterial>, String> {
    if !(-128..=127).contains(&priority) {
        return Err(format!(
            "Ribbon priority{priority} exceeds native render priority range"
        ));
    }
    let (shader, blend) = material::ribbon_material_shader(authored)?;
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shader);
    material.set_render_priority(i32::from(priority));
    material.set_shader_parameter("ribbon_texture", &image.to_variant());
    material.set_shader_parameter("gx_blend", &i32::from(blend).to_variant());
    Ok(material)
}

fn upload_geometry(mesh: &mut Gd<ArrayMesh>, vertices: &[RibbonVertex]) {
    mesh.clear_surfaces();
    if vertices.is_empty() {
        return;
    }
    let positions: PackedVector3Array = vertices
        .iter()
        .map(|vertex| Vector3::from_array(vertex.position))
        .collect();
    let uvs: PackedVector2Array = vertices
        .iter()
        .map(|vertex| Vector2::from_array(vertex.uv))
        .collect();
    let colors: PackedColorArray = vertices
        .iter()
        .map(|vertex| {
            Color::from_rgba(
                vertex.color[0],
                vertex.color[1],
                vertex.color[2],
                vertex.color[3],
            )
        })
        .collect();
    let mut arrays = VarArray::new();
    arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(
        mesh::ArrayType::VERTEX.ord() as usize,
        &positions.to_variant(),
    );
    arrays.set(mesh::ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
    arrays.set(mesh::ArrayType::COLOR.ord() as usize, &colors.to_variant());
    mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
}

#[cfg(debug_assertions)]
#[path = "ribbons/material_fixture.rs"]
mod material_fixture;

fn wow_vector([x, y, z]: [f32; 3]) -> Vector3 {
    Vector3::new(x, z, -y)
}
