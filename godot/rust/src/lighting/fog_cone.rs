//! The LightSkybox flag 0x4 fog cone (WebWowViewerCpp map.cpp `createSkyMesh(.., true)`,
//! `Map::updateBuffers` `skyMeshMat0x4`): drawn after the skybox models, the dome's lower
//! cone from the SkyBand2 ring (alpha 0) through the SkyFog ring to the bottom pole
//! (alpha 1) in the end fog colour, with the dome's sun-fog scattering. It veils a
//! skybox model's lower edge into the final fog.

use game_engine_core::{retail_fog::FogUniforms, sky_cubemap_data::sky_dome_profile};
use godot::{
    classes::{ArrayMesh, MeshInstance3D, Node3D, ResourceLoader, Shader, ShaderMaterial, mesh},
    prelude::*,
};

const SHADER_PATH: &str = "res://shaders/sky_fog_cone.gdshader";
/// skyConusVBO: 24 vertices per ring.
const SEGMENTS: usize = 24;
/// Index buffer slice 198..300 covers dome points 4 (SkyBand2 ring), 5 (SkyFog ring) and
/// 6 (bottom pole); `skyColor[0..4]` alpha 0, `skyColor[5]` alpha 1.
const RINGS: [(usize, f32); 2] = [(4, 0.0), (5, 1.0)];
const BOTTOM_POLE: usize = 6;

pub(super) struct FogCone {
    node: Gd<MeshInstance3D>,
    material: Gd<ShaderMaterial>,
}

impl FogCone {
    pub fn load(root: &mut Gd<Node3D>) -> Result<Self, String> {
        let shader = ResourceLoader::singleton()
            .load(SHADER_PATH)
            .and_then(|resource| resource.try_cast::<Shader>().ok())
            .ok_or_else(|| format!("Fog cone shader {SHADER_PATH} failed to load"))?;
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(&shader);
        let mut node = MeshInstance3D::new_alloc();
        node.set_name("SkyFogCone");
        node.set_mesh(&cone_mesh());
        node.set_material_override(&material);
        node.set_cast_shadows_setting(
            godot::classes::geometry_instance_3d::ShadowCastingSetting::OFF,
        );
        // Sorted after the skybox models, which also sit on the camera.
        node.set_sorting_offset(1.0);
        node.set_visible(false);
        root.add_child(&node);
        Ok(Self { node, material })
    }

    /// Shows the cone while a collected skybox has flag 0x4, in `end_fog_color` (linear;
    /// SkyFogColor `sky_fog` when the end fog colour is black) with the scene sun fog.
    pub fn sync(
        &mut self,
        shown: bool,
        end_fog_color: [f32; 3],
        sky_fog: [f32; 3],
        fog: &FogUniforms,
    ) {
        self.node.set_visible(shown);
        if !shown {
            return;
        }
        let length = end_fog_color.iter().map(|c| c * c).sum::<f32>().sqrt();
        let color = if length < 0.0001 {
            sky_fog
        } else {
            end_fog_color
        };
        let material = &mut self.material;
        for (name, value) in [
            ("fog_color", color),
            ("fog_sun_direction", fog.sun_direction),
            ("fog_sun_color", fog.sun_color),
        ] {
            material.set_shader_parameter(name, &Vector3::from_array(value).to_variant());
        }
        for (name, value) in [
            ("fog_sun_angle", fog.sun_angle),
            ("fog_sun_percentage", fog.sun_percentage),
        ] {
            material.set_shader_parameter(name, &value.to_variant());
        }
    }

    pub fn place(&mut self, camera: Vector3) {
        if self.node.is_visible() {
            self.node.set_global_position(camera);
        }
    }
}

/// Godot y-up cone: the two rings, then a fan to the bottom pole; alpha in vertex colour.
fn cone_mesh() -> Gd<ArrayMesh> {
    let mut vertices = PackedVector3Array::new();
    let mut colors = PackedColorArray::new();
    for (vertex, alpha) in cone_triangles() {
        vertices.push(vertex);
        colors.push(Color::from_rgba(1.0, 1.0, 1.0, alpha));
    }
    let mut arrays = VarArray::new();
    arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(
        mesh::ArrayType::VERTEX.ord() as usize,
        &vertices.to_variant(),
    );
    arrays.set(mesh::ArrayType::COLOR.ord() as usize, &colors.to_variant());
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
    mesh
}

/// Triangle-list corners with their alpha: per segment, the band between the rings and
/// the fan triangle down to the bottom pole.
fn cone_triangles() -> Vec<(Vector3, f32)> {
    let profile = sky_dome_profile();
    let [(upper, top), (lower, low)] = RINGS;
    let ring = |point: usize, segment: usize| {
        let angle = segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let p = profile[point];
        Vector3::new(
            p.horizontal * angle.cos(),
            p.height,
            p.horizontal * angle.sin(),
        )
    };
    let bottom = Vector3::new(0.0, profile[BOTTOM_POLE].height, 0.0);
    (0..SEGMENTS)
        .flat_map(|segment| {
            let next = (segment + 1) % SEGMENTS;
            [
                (ring(upper, segment), top),
                (ring(lower, segment), low),
                (ring(upper, next), top),
                (ring(upper, next), top),
                (ring(lower, segment), low),
                (ring(lower, next), low),
                (ring(lower, segment), low),
                (bottom, low),
                (ring(lower, next), low),
            ]
        })
        .collect()
}
