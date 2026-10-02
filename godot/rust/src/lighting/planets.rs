//! Sun and moon discs (WebWowViewerCpp map.cpp `createPlanetMesh`, `planetShader`): one
//! camera-facing quad per disc on a 12-yard sphere around the camera, drawn in the sky view
//! after the stars (`PLANETS_PRIORITY`).

use std::path::Path;

use game_engine_core::sky_bodies::{PLANET_FDIDS, PlanetDraw};
use godot::{
    classes::{MeshInstance3D, Node3D, QuadMesh, ResourceLoader, Shader, ShaderMaterial},
    prelude::*,
};

use crate::assets;

const SHADER_PATH: &str = "res://shaders/sky_planet.gdshader";
/// `updatePlanetsAndStars`: the discs sit on the sky sphere at radius 12.
const SKY_RADIUS: f32 = 12.0;

struct Disc {
    fdid: u32,
    node: Gd<MeshInstance3D>,
    material: Gd<ShaderMaterial>,
    direction: Vector3,
}

pub(super) struct Planets {
    discs: Vec<Disc>,
}

impl Planets {
    /// The three discs under `root`, hidden until `sync` shows them.
    pub fn load(root: &mut Gd<Node3D>, data_root: &Path) -> Result<Self, String> {
        let shader = ResourceLoader::singleton()
            .load(SHADER_PATH)
            .and_then(|resource| resource.try_cast::<Shader>().ok())
            .ok_or_else(|| format!("Planet shader {SHADER_PATH} failed to load"))?;
        let texture_dir = data_root.join("textures");
        let mut discs = Vec::new();
        for fdid in PLANET_FDIDS {
            let mut missing = PackedInt32Array::new();
            let texture = assets::material::shared_texture(fdid, &texture_dir, &mut missing)?
                .ok_or_else(|| format!("Planet texture {fdid} missing"))?;
            let mut material = ShaderMaterial::new_gd();
            material.set_shader(&shader);
            material.set_render_priority(super::PLANETS_PRIORITY);
            material.set_shader_parameter("planet_texture", &texture.to_variant());
            let mut quad = QuadMesh::new_gd();
            quad.set_size(Vector2::ONE);
            let mut node = MeshInstance3D::new_alloc();
            node.set_name(&format!("Planet{fdid}"));
            node.set_mesh(&quad);
            node.set_material_override(&material);
            node.set_cast_shadows_setting(
                godot::classes::geometry_instance_3d::ShadowCastingSetting::OFF,
            );
            // The billboard grows past the 1-yard quad's bounds (moon: 3.3 yards).
            node.set_extra_cull_margin(4.0);
            node.set_visible(false);
            root.add_child(&node);
            discs.push(Disc {
                fdid,
                node,
                material,
                direction: Vector3::UP,
            });
        }
        Ok(Self { discs })
    }

    /// Shows the discs of `draws` at their direction and scale, tinted by `color` (authored
    /// exterior specular colour); hides the rest.
    pub fn sync(&mut self, draws: &[PlanetDraw], color: [f32; 3]) {
        for disc in &mut self.discs {
            let Some(draw) = draws.iter().find(|draw| draw.fdid == disc.fdid) else {
                disc.node.set_visible(false);
                continue;
            };
            disc.direction = Vector3::from_array(draw.direction);
            disc.material
                .set_shader_parameter("scale", &draw.scale.to_variant());
            disc.material
                .set_shader_parameter("color", &Vector3::from_array(color).to_variant());
            disc.node.set_visible(true);
        }
    }

    /// Keeps each shown disc `SKY_RADIUS` yards from `camera` along its direction.
    pub fn place(&mut self, camera: Vector3) {
        for disc in self.discs.iter_mut().filter(|disc| disc.node.is_visible()) {
            disc.node
                .set_global_position(camera + disc.direction * SKY_RADIUS);
        }
    }
}
