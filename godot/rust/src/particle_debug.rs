//! `--screen particledebug`: one M2 model's particle emitters under an orbit camera,
//! with the emitter overlay of the original Bevy screen
//! (`src/scenes/particle_debug/mod.rs`). Needs no server.
//!
//! Controls: left drag orbits, the wheel zooms, Tab / Shift+Tab cycles [`MODELS`],
//! 1-9 toggle emitters #0-#8, 0 enables every emitter, R restarts the emitters.

use std::{path::PathBuf, rc::Rc};

use game_engine_core::{
    char_select_camera_data::scaled_orbit_delta,
    m2::{self, ParticleEmitter},
    m2_lights::{self, LightTime, PointLight},
    outfit_data::OutfitData,
};
use glam::{Vec2, Vec3};
use godot::{
    builtin::Side,
    classes::{
        Camera3D, CanvasLayer, DirectionalLight3D, Environment, INode3D, InputEvent, InputEventKey,
        InputEventMouseButton, InputEventMouseMotion, Label, MeshInstance3D, PanelContainer,
        PlaneMesh, StandardMaterial3D, StyleBoxFlat, WorldEnvironment,
        control::{LayoutPreset, MouseFilter},
        environment,
        text_server::AutowrapMode,
    },
    global::{Key, MouseButton},
    prelude::*,
};
use osso_asset_resolver::CascListfileResolver;

use crate::{
    GameClient,
    assets::{
        build_model,
        creature::{cache_model_files, cache_model_textures, local_resolver},
        read_model,
    },
    particles::{ModelParticles, ParticlePools, PlacedParticles, view_basis},
};

/// A model the screen shows and the orbit that frames its particles.
pub(crate) struct DebugModel {
    pub fdid: u32,
    pub name: &'static str,
    /// Orbit focus above the model origin, in yards.
    pub focus_height: f32,
    /// Orbit focus along +X, where missile trails stream.
    pub focus_x: f32,
    pub distance: f32,
    /// Initial orbit yaw; the portal's sheet faces +X.
    pub yaw: f32,
}

/// Models the screen cycles through; the first is the original screen's torch at its
/// original 3-yard orbit about the model origin.
pub(crate) const MODELS: [DebugModel; 3] = [
    DebugModel {
        fdid: 145304,
        name: "club_1h_torch_a_01",
        focus_height: 0.0,
        focus_x: 0.0,
        distance: 3.0,
        yaw: 0.0,
    },
    DebugModel {
        fdid: 197007,
        name: "instanceportal",
        focus_height: 3.0,
        focus_x: 0.0,
        distance: 16.0,
        yaw: std::f32::consts::FRAC_PI_2,
    },
    DebugModel {
        fdid: 1598570,
        name: "cfx_mage_frostbolt_missile",
        focus_height: 0.0,
        focus_x: 3.0,
        distance: 7.0,
        yaw: 0.0,
    },
];
/// The original screen places its model half a yard above the ground.
const MODEL_ORIGIN: Vector3 = Vector3::new(0.0, 0.5, 0.0);
const ZOOM_STEP: f32 = 0.4;
const ZOOM_LERP: f32 = 0.25;
const BASE_PITCH: f32 = 0.15;
/// Keeps the eye off the vertical so `look_at` has a defined up axis.
const PITCH_LIMIT: f32 = 1.4;
const PARTICLE_SEED: u32 = 7;

/// The original `OrbitCamera`: yaw/pitch about `focus`, zoom eased toward its target.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Orbit {
    pub yaw: f32,
    pub pitch: f32,
    pub focus: Vec3,
    pub distance: f32,
    pub target_distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl Orbit {
    pub fn new(focus: Vec3, distance: f32) -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            focus,
            distance,
            target_distance: distance,
            min_distance: 0.5,
            max_distance: (distance * 4.0).max(20.0),
        }
    }

    pub fn eye(&self) -> Vec3 {
        let pitch = BASE_PITCH + self.pitch;
        self.focus
            + Vec3::new(
                self.yaw.sin() * pitch.cos(),
                pitch.sin(),
                self.yaw.cos() * pitch.cos(),
            ) * self.distance
    }

    /// Mouse-drag motion in pixels.
    pub fn drag(&mut self, motion: Vec2, sensitivity: f32) {
        let delta = scaled_orbit_delta(motion, sensitivity);
        self.yaw += delta.x;
        self.pitch =
            (self.pitch + delta.y).clamp(-PITCH_LIMIT - BASE_PITCH, PITCH_LIMIT - BASE_PITCH);
    }

    /// Wheel notches, positive toward the model.
    pub fn zoom(&mut self, notches: f32) {
        self.target_distance = (self.target_distance - notches * ZOOM_STEP)
            .clamp(self.min_distance, self.max_distance);
    }

    pub fn ease(&mut self) {
        self.distance += (self.target_distance - self.distance) * ZOOM_LERP;
    }
}

/// How the overlay reports an authored emitter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EmitterState {
    On,
    Off,
    /// An emitter type the simulation has no shape for.
    NotSimulated,
}

impl EmitterState {
    fn label(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Off => "off",
            Self::NotSimulated => "not simulated",
        }
    }
}

/// The original overlay: every authored emitter's key fields, with its state here.
pub(crate) fn format_overlay(
    model: &str,
    emitters: &[ParticleEmitter],
    states: &[EmitterState],
    drawn: usize,
) -> String {
    let mut lines = vec![
        "Particle Debug".to_string(),
        format!("Model: {model}"),
        "Drag: orbit  Wheel: zoom  Tab: model  1-9: emitter  0: all  R: restart".to_string(),
    ];
    if emitters.is_empty() {
        lines.push("No particle emitters".to_string());
        return lines.join("\n");
    }
    lines.push(format!(
        "Emitters: {}  Particles drawn: {drawn}",
        emitters.len()
    ));
    for (index, (emitter, state)) in emitters.iter().zip(states).enumerate() {
        lines.push(String::new());
        lines.push(format!("Emitter #{index} [{}]", state.label()));
        lines.extend(format_emitter_lines(emitter));
    }
    lines.join("\n")
}

/// Each point light's bone, colour and attenuation as retail evaluates them.
pub(crate) fn format_lights(lights: &[(i16, PointLight)]) -> Vec<String> {
    lights
        .iter()
        .enumerate()
        .flat_map(|(index, (bone, light))| {
            let [r, g, b] = light.color;
            [
                String::new(),
                format!(
                    "Light #{index} point bone={bone} color=({r:.3}, {g:.3}, {b:.3}) attenuation={:.3}-{:.3}",
                    light.attenuation_start, light.attenuation_end
                ),
            ]
        })
        .collect()
}

/// The model's point lights at the first frame of its first sequence.
fn model_lights(model: &m2::Model) -> Vec<(i16, PointLight)> {
    let time = LightTime {
        sequence: 0,
        time_ms: 0,
        global_ms: 0,
        global_sequences: &model.global_sequences,
    };
    model
        .lights
        .iter()
        .filter_map(|light| {
            m2_lights::point_light(light, model.flags, &time).map(|point| (light.bone_index, point))
        })
        .collect()
}

fn format_emitter_lines(emitter: &ParticleEmitter) -> [String; 5] {
    [
        format_identity_line(emitter),
        format_motion_line(emitter),
        format_area_line(emitter),
        format_key_counts_line(emitter),
        format_twinkle_line(emitter),
    ]
}

fn format_identity_line(emitter: &ParticleEmitter) -> String {
    format!(
        "blend={} type={} particle={} head_tail={} bone={} tex={:?} flags={:#x}",
        emitter.blend_type,
        emitter.emitter_type,
        emitter.particle_type,
        emitter.head_or_tail,
        emitter.bone_index,
        emitter.texture_fdid,
        emitter.flags
    )
}

fn format_motion_line(emitter: &ParticleEmitter) -> String {
    format!(
        "life={:.3} +/- {:.3} rate={:.3} speed={:.3} +/- {:.3}",
        emitter.lifespan,
        emitter.lifespan_variation,
        emitter.emission_rate,
        emitter.emission_speed,
        emitter.speed_variation
    )
}

fn format_area_line(emitter: &ParticleEmitter) -> String {
    format!(
        "gravity={:.3} drag={:.3} area=({:.3}, {:.3}) tiles={}x{}",
        emitter.gravity,
        emitter.drag,
        emitter.area_length,
        emitter.area_width,
        emitter.tile_rows,
        emitter.tile_cols
    )
}

fn format_key_counts_line(emitter: &ParticleEmitter) -> String {
    format!(
        "opacity={:?} color_keys={} opacity_keys={} scale_keys={}",
        emitter.opacity,
        emitter.color_keys.len(),
        emitter.opacity_keys.len(),
        emitter.scale_keys.len()
    )
}

fn format_twinkle_line(emitter: &ParticleEmitter) -> String {
    format!(
        "burst={:.3} mid={:.3} twinkle=({:.3}, {:.3}, {:.3}, {:.3})",
        emitter.burst_multiplier,
        emitter.mid_point,
        emitter.twinkle_speed,
        emitter.twinkle_percent,
        emitter.twinkle_scale_min,
        emitter.twinkle_scale_max
    )
}

/// The loaded model and its placed emitters.
struct Shown {
    index: usize,
    node: Gd<Node3D>,
    emitters: Vec<ParticleEmitter>,
    lights: Vec<(i16, PointLight)>,
    particles: Option<Rc<ModelParticles>>,
    placed: Option<PlacedParticles>,
    missing_textures: PackedInt32Array,
}

/// The particle debug scene: model, emitters, orbit camera and overlay.
#[derive(GodotClass)]
#[class(base = Node3D, no_init)]
pub struct WowParticleDebug {
    base: Base<Node3D>,
    data_root: PathBuf,
    resolver: Option<CascListfileResolver>,
    outfit: Option<OutfitData>,
    sensitivity: f32,
    orbit: Orbit,
    dragging: bool,
    camera: Option<Gd<Camera3D>>,
    overlay: Option<Gd<Label>>,
    pools: ParticlePools,
    shown: Option<Shown>,
    /// Authored emitter indices switched off.
    disabled: Vec<usize>,
}

#[godot_api]
impl INode3D for WowParticleDebug {
    fn ready(&mut self) {
        // Model animation nodes pose bones at priority 0 before emitters read them.
        self.base_mut().set_process_priority(1);
        self.attach_environment();
        let result = self.show_model(0);
        self.report(result);
    }

    fn process(&mut self, delta: f64) {
        self.orbit.ease();
        let Some(mut camera) = self.camera.clone() else {
            return;
        };
        let eye = self.orbit.eye();
        let focus = self.orbit.focus;
        camera.look_at_from_position(
            Vector3::new(eye.x, eye.y, eye.z),
            Vector3::new(focus.x, focus.y, focus.z),
        );
        let view = view_basis(camera.get_global_transform());
        self.pools.begin_frame();
        if let Some(shown) = self.shown.as_mut()
            && let Some(placed) = shown.placed.as_mut()
        {
            placed.update_and_draw(&shown.node, delta as f32, 1.0, &view, &mut self.pools);
        }
        self.pools.end_frame();
        self.sync_overlay();
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        let handled = if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            if self.dragging {
                let relative = motion.get_relative();
                self.orbit
                    .drag(Vec2::new(relative.x, relative.y), self.sensitivity);
            }
            self.dragging
        } else if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() {
            self.mouse_button(&button)
        } else if let Ok(key) = event.try_cast::<InputEventKey>() {
            let handled = key.is_pressed() && !key.is_echo();
            if handled {
                let result = self.key(key.get_keycode(), key.is_shift_pressed());
                self.report(result);
            }
            handled
        } else {
            false
        };
        if handled && let Some(mut viewport) = self.base().get_viewport() {
            viewport.set_input_as_handled();
        }
    }

    fn exit_tree(&mut self) {
        self.pools.reset();
    }
}

#[godot_api]
impl WowParticleDebug {
    /// The shown model, its emitters and this frame's drawn particles, for fixtures.
    #[func]
    fn debug_state(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set("drawn", self.pools.drawn() as i64);
        state.set("pools", self.pools.pool_count() as i64);
        let Some(shown) = &self.shown else {
            return state;
        };
        let DebugModel { fdid, name, .. } = MODELS[shown.index];
        state.set("model_index", shown.index as i64);
        state.set("model_fdid", i64::from(fdid));
        state.set("model_name", name);
        state.set("emitters", shown.emitters.len() as i64);
        let enabled: Vec<i32> = self
            .states(shown)
            .iter()
            .enumerate()
            .filter(|(_, state)| **state == EmitterState::On)
            .map(|(index, _)| index as i32)
            .collect();
        state.set("enabled", &PackedInt32Array::from(enabled.as_slice()));
        state.set("missing_textures", &shown.missing_textures);
        if let Some(overlay) = &self.overlay {
            state.set("overlay", &overlay.get_text());
        }
        state
    }
}

impl WowParticleDebug {
    fn new(data_root: PathBuf, sensitivity: f32) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            data_root,
            resolver: None,
            outfit: None,
            sensitivity,
            orbit: framing_orbit(&MODELS[0]),
            dragging: false,
            camera: None,
            overlay: None,
            pools: ParticlePools::new(1.0),
            shown: None,
            disabled: Vec::new(),
        })
    }

    fn report(&self, result: Result<(), String>) {
        if let Err(error) = result {
            godot_error!("Particle debug: {error}");
        }
    }

    fn mouse_button(&mut self, button: &Gd<InputEventMouseButton>) -> bool {
        match button.get_button_index() {
            MouseButton::LEFT => {
                self.dragging = button.is_pressed();
                true
            }
            MouseButton::WHEEL_UP if button.is_pressed() => {
                self.orbit.zoom(button.get_factor().max(1.0));
                true
            }
            MouseButton::WHEEL_DOWN if button.is_pressed() => {
                self.orbit.zoom(-button.get_factor().max(1.0));
                true
            }
            _ => false,
        }
    }

    fn key(&mut self, key: Key, shift: bool) -> Result<(), String> {
        let current = self.shown.as_ref().map_or(0, |shown| shown.index);
        match key {
            Key::TAB => {
                let step = if shift { MODELS.len() - 1 } else { 1 };
                self.show_model((current + step) % MODELS.len())
            }
            Key::KEY_0 => {
                self.disabled.clear();
                self.place_particles()
            }
            Key::R => self.place_particles(),
            _ => {
                let Some(index) =
                    (Key::KEY_1.ord()..=Key::KEY_9.ord()).position(|ord| ord == key.ord())
                else {
                    return Ok(());
                };
                if let Some(slot) = self.disabled.iter().position(|&off| off == index) {
                    self.disabled.remove(slot);
                } else {
                    self.disabled.push(index);
                }
                self.place_particles()
            }
        }
    }

    fn attach_environment(&mut self) {
        for node in [environment_node(), light_node(), ground_node()] {
            self.base_mut().add_child(&node);
        }
        let mut camera = Camera3D::new_alloc();
        camera.set_name("Camera");
        camera.set_near(0.01);
        self.base_mut().add_child(&camera);
        camera.make_current();
        self.camera = Some(camera);
        self.attach_overlay();
        let mut root = self.base().clone();
        self.pools.attach(&mut root);
    }

    fn attach_overlay(&mut self) {
        let mut layer = CanvasLayer::new_alloc();
        layer.set_name("Overlay");
        let mut style = StyleBoxFlat::new_gd();
        style.set_bg_color(Color::from_rgba(0.02, 0.02, 0.02, 0.82));
        style.set_content_margin_all(10.0);
        let mut panel = PanelContainer::new_alloc();
        panel.set_name("EmitterOverlay");
        panel.add_theme_stylebox_override("panel", &style);
        panel.set_anchors_preset(LayoutPreset::TOP_RIGHT);
        panel.set_offset(Side::LEFT, -532.0);
        panel.set_offset(Side::RIGHT, -12.0);
        panel.set_offset(Side::TOP, 12.0);
        panel.set_mouse_filter(MouseFilter::IGNORE);
        let mut label = Label::new_alloc();
        label.add_theme_font_size_override("font_size", 15);
        label.add_theme_color_override("font_color", Color::WHITE);
        label.set_autowrap_mode(AutowrapMode::WORD_SMART);
        label.set_mouse_filter(MouseFilter::IGNORE);
        panel.add_child(&label);
        layer.add_child(&panel);
        self.base_mut().add_child(&layer);
        self.overlay = Some(label);
    }

    fn show_model(&mut self, index: usize) -> Result<(), String> {
        if let Some(previous) = self.shown.take() {
            previous.node.free();
        }
        self.disabled.clear();
        let DebugModel { fdid, name, .. } = MODELS[index];
        let (mut node, model, missing_textures) = self
            .load_model(fdid)
            .map_err(|error| format!("model {name} ({fdid}): {error}"))?;
        node.set_name(&format!("ParticleDebugModel{fdid}"));
        node.set_position(MODEL_ORIGIN);
        self.base_mut().add_child(&node);
        if !missing_textures.is_empty() {
            godot_error!("Particle debug model {fdid}: missing textures {missing_textures:?}");
        }
        self.orbit = framing_orbit(&MODELS[index]);
        self.shown = Some(Shown {
            index,
            node,
            particles: ModelParticles::from_model(fdid, &model),
            lights: model_lights(&model),
            emitters: model.particle_emitters,
            placed: None,
            missing_textures,
        });
        self.place_particles()
    }

    fn load_model(
        &mut self,
        fdid: u32,
    ) -> Result<(Gd<Node3D>, m2::Model, PackedInt32Array), String> {
        let data_root = self.data_root.clone();
        let resolver = self
            .resolver
            .get_or_insert_with(|| local_resolver(&data_root));
        let path = cache_model_files(resolver, &data_root, fdid)?;
        let skins = self
            .outfit
            .get_or_insert_with(|| OutfitData::load(&data_root))
            .resolve_item_model_skin_fdids_for_model_path(&path)
            .unwrap_or([0; 3]);
        let path = GString::from(path.to_string_lossy().as_ref());
        let model = read_model(&path)?;
        cache_model_textures(resolver, &data_root, &skins, &model)?;
        let (node, missing) = build_model(&model, &path, &skins, None)?;
        Ok((node, model, missing))
    }

    /// Places the enabled emitters afresh: toggling restarts every emitter.
    fn place_particles(&mut self) -> Result<(), String> {
        self.pools.reset();
        let mut root = self.base().clone();
        self.pools.attach(&mut root);
        let Some(shown) = self.shown.as_mut() else {
            return Ok(());
        };
        let disabled = &self.disabled;
        shown.placed = None;
        let Some(particles) = shown
            .particles
            .as_ref()
            .and_then(|particles| particles.filtered(|index| !disabled.contains(&index)))
        else {
            return Ok(());
        };
        let texture_dir = self.data_root.join("textures");
        let (placed, errors) =
            self.pools
                .place(&particles, &shown.node, PARTICLE_SEED, &texture_dir);
        shown.placed = Some(placed);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    fn states(&self, shown: &Shown) -> Vec<EmitterState> {
        let simulated: Vec<usize> = shown
            .particles
            .as_ref()
            .map(|particles| particles.authored_indices().collect())
            .unwrap_or_default();
        (0..shown.emitters.len())
            .map(|index| {
                if !simulated.contains(&index) {
                    EmitterState::NotSimulated
                } else if self.disabled.contains(&index) {
                    EmitterState::Off
                } else {
                    EmitterState::On
                }
            })
            .collect()
    }

    fn sync_overlay(&mut self) {
        let Some(shown) = &self.shown else {
            return;
        };
        let DebugModel { fdid, name, .. } = MODELS[shown.index];
        let mut text = format_overlay(
            &format!("{name} ({fdid}) [{}/{}]", shown.index + 1, MODELS.len()),
            &shown.emitters,
            &self.states(shown),
            self.pools.drawn(),
        );
        for line in format_lights(&shown.lights) {
            text.push('\n');
            text.push_str(&line);
        }
        if let Some(overlay) = self.overlay.as_mut()
            && overlay.get_text().to_string() != text
        {
            overlay.set_text(&text);
        }
    }
}

/// The original screen's dark clear colour and cool ambient light.
fn environment_node() -> Gd<Node> {
    let mut environment = Environment::new_gd();
    environment.set_background(environment::BgMode::COLOR);
    environment.set_bg_color(Color::from_rgb(0.03, 0.04, 0.06));
    environment.set_ambient_source(environment::AmbientSource::COLOR);
    environment.set_ambient_light_color(Color::from_rgb(0.92, 0.94, 0.98));
    environment.set_ambient_light_energy(0.4);
    environment.set_tonemapper(environment::ToneMapper::LINEAR);
    let mut world = WorldEnvironment::new_alloc();
    world.set_name("Environment");
    world.set_environment(&environment);
    world.upcast()
}

fn light_node() -> Gd<Node> {
    let mut light = DirectionalLight3D::new_alloc();
    light.set_name("Light");
    light.set_color(Color::from_rgb(1.0, 0.96, 0.9));
    light.set_shadow(true);
    light.set_rotation(Vector3::new(-45f32.to_radians(), 30f32.to_radians(), 0.0));
    light.upcast()
}

/// The original 18-yard dark ground plane.
fn ground_node() -> Gd<Node> {
    let mut plane = PlaneMesh::new_gd();
    plane.set_size(Vector2::new(18.0, 18.0));
    let mut material = StandardMaterial3D::new_gd();
    material.set_albedo(Color::from_rgb(0.08, 0.09, 0.11));
    material.set_roughness(0.96);
    material.set_metallic(0.02);
    plane.set_material(&material);
    let mut ground = MeshInstance3D::new_alloc();
    ground.set_name("Ground");
    ground.set_mesh(&plane);
    ground.upcast()
}

fn framing_orbit(model: &DebugModel) -> Orbit {
    let mut orbit = Orbit::new(
        Vec3::new(
            MODEL_ORIGIN.x + model.focus_x,
            MODEL_ORIGIN.y + model.focus_height,
            MODEL_ORIGIN.z,
        ),
        model.distance,
    );
    orbit.yaw = model.yaw;
    orbit
}

impl GameClient {
    /// Shows the particle debug scene in place of the login screen.
    pub(super) fn open_particle_debug(&mut self) -> Result<(), String> {
        if let Some(login) = self.login_ui.as_mut() {
            login.set_visible(false);
        }
        let mut scene = WowParticleDebug::new(
            self.data_root.clone(),
            self.client_options.camera.mouse_sensitivity,
        );
        scene.set_name("ParticleDebug");
        self.base_mut().add_child(&scene);
        Ok(())
    }
}

#[cfg(test)]
#[path = "particle_debug_tests.rs"]
mod tests;
