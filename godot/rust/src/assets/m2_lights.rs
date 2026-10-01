//! A model's M2 point lights as `OmniLight3D` children following their bones and tracks
//! (`game_engine_core::m2_lights`).
//!
//! Retail applies them in a deferred pass (WebWowViewerCpp
//! `shaders/slang/deferred/lights/pointLight.frag.slang:43-58`): full colour inside the
//! attenuation start, ramping linearly to none at its end, squared. Godot linearises the
//! light's sRGB colour, which stands in for squaring the colour; the opaque shaders
//! recover the distance from Godot's windowed falloff (range = end, exponent 0) and
//! apply retail's squared ramp (`shaders/m2_point_light.gdshaderinc`), reading start /
//! end from the light's specular parameter.
use game_engine_core::{
    asset::m2_format::m2_light::{M2_LIGHT_TYPE_POINT, M2Light},
    m2, m2_billboard,
    m2_lights::{self, LightTime},
};
use godot::{
    classes::{INode3D, Node3D, OmniLight3D, Skeleton3D, light_3d},
    prelude::*,
};

use super::uv_animation::WowMaterialClock;
use crate::animation::WowAnimationPlayer;

struct Light {
    authored: M2Light,
    /// Bone index and pivot in Godot model axes, when the light follows a bone.
    bone: Option<(i32, Vector3)>,
    node: Gd<OmniLight3D>,
}

impl Light {
    fn new(index: usize, authored: &M2Light, model: &m2::Model) -> Self {
        let mut node = OmniLight3D::new_alloc();
        node.set_name(&format!("M2Light{index}"));
        node.set_param(light_3d::Param::ATTENUATION, 0.0);
        node.set_shadow(false);
        let bone = usize::try_from(authored.bone_index)
            .ok()
            .and_then(|bone| Some((bone as i32, godot_vector(model.bones.get(bone)?.pivot))));
        Self {
            authored: authored.clone(),
            bone,
            node,
        }
    }

    /// Places, colours and sizes the light for `point`, its bone at `bone`.
    fn apply(&mut self, point: m2_lights::PointLight, bone: Transform3D, model_scale: f32) {
        let [r, g, b] = point.color;
        self.node
            .set_position(bone * godot_vector(self.authored.position));
        self.node.set_visible(point.visible);
        self.node.set_color(Color::from_rgb(r, g, b));
        self.node
            .set_param(light_3d::Param::RANGE, point.attenuation_end * model_scale);
        self.node
            .set_param(light_3d::Param::SPECULAR, point.start_fraction());
    }
}

#[derive(GodotClass)]
#[class(base = Node3D, no_init)]
pub struct WowM2Lights {
    base: Base<Node3D>,
    lights: Vec<Light>,
    model_flags: u32,
    global_sequences: Vec<u32>,
    skeleton: Option<Gd<Skeleton3D>>,
    player: Option<Gd<WowAnimationPlayer>>,
    clock: Option<Gd<WowMaterialClock>>,
    /// A track, bone or billboard moves some light; otherwise the first update holds.
    animates: bool,
}

#[godot_api]
impl INode3D for WowM2Lights {
    fn ready(&mut self) {
        // Model animation nodes pose bones at priority 0 before lights read them.
        self.base_mut().set_process_priority(1);
        let parent = self.base().get_parent();
        self.skeleton = parent
            .as_ref()
            .and_then(|parent| parent.try_get_node_as::<Skeleton3D>("Skeleton3D"));
        self.player = parent
            .as_ref()
            .and_then(|parent| parent.try_get_node_as::<WowAnimationPlayer>("M2Animation"));
        self.clock = self
            .base()
            .try_get_node_as::<WowMaterialClock>("/root/M2MaterialClock");
        if self.clock.is_none() {
            godot_error!("M2 lights require /root/M2MaterialClock");
        }
        self.update();
        if !self.animates {
            self.base_mut().set_process(false);
        }
    }

    fn process(&mut self, _delta: f64) {
        self.update();
    }
}

impl WowM2Lights {
    /// `None` when the model has no point light.
    pub(super) fn from_model(model: &m2::Model) -> Option<Gd<Self>> {
        let lights: Vec<_> = model
            .lights
            .iter()
            .filter(|light| light.light_type == M2_LIGHT_TYPE_POINT)
            .enumerate()
            .map(|(index, light)| Light::new(index, light, model))
            .collect();
        if lights.is_empty() {
            return None;
        }
        let animates = lights
            .iter()
            .any(|light| m2_lights::light_animates(&light.authored))
            || !m2::bones_are_static(model)
            || model
                .bones
                .iter()
                .any(|bone| m2_billboard::is_billboard(bone.flags));
        let nodes: Vec<_> = lights.iter().map(|light| light.node.clone()).collect();
        let mut node = Gd::from_init_fn(|base| Self {
            base,
            lights,
            model_flags: model.flags,
            global_sequences: model.global_sequences.clone(),
            skeleton: None,
            player: None,
            clock: None,
            animates,
        });
        node.set_name("M2Lights");
        for light in &nodes {
            node.add_child(light);
        }
        Some(node)
    }

    fn update(&mut self) {
        // Bound equipment frees its own skeleton and animation for the wearer's.
        let (sequence, time_ms) = self
            .player
            .as_ref()
            .filter(|player| player.is_instance_valid())
            .and_then(|player| player.bind().playback())
            .unwrap_or((0, 0));
        let global_ms = self
            .clock
            .as_ref()
            .map_or(0, |clock| clock.bind().elapsed_time_ms() as u64);
        let time = LightTime {
            sequence,
            time_ms,
            global_ms,
            global_sequences: &self.global_sequences,
        };
        let model_scale = self.base().get_global_transform().basis.get_scale().x;
        let skeleton = self
            .skeleton
            .as_ref()
            .filter(|skeleton| skeleton.is_instance_valid());
        for light in &mut self.lights {
            let Some(point) = m2_lights::point_light(&light.authored, self.model_flags, &time)
            else {
                continue;
            };
            let bone = light.bone.zip(skeleton).map_or(
                Transform3D::IDENTITY,
                |((index, pivot), skeleton)| {
                    skeleton.get_bone_global_pose(index) * Transform3D::IDENTITY.translated(-pivot)
                },
            );
            light.apply(point, bone, model_scale);
        }
    }
}

/// WoW axes (x, y, z) in Godot axes (x, z, -y).
fn godot_vector([x, y, z]: [f32; 3]) -> Vector3 {
    Vector3::new(x, z, -y)
}
