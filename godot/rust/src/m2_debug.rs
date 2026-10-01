//! `--screen m2debug`: the original M2 debug scene (`src/scenes/m2_debug/mod.rs`) — the
//! wolf reference model on a grass ground plane under a directional light, framed by
//! an orbit camera 6 yd from a focus 1 yd above the origin. Needs no server.
//!
//! Controls (original `OrbitCamera`): left drag orbits, the wheel zooms.

use std::path::{Path, PathBuf};

use game_engine_core::creature_display_data::query_preferred_skins;
use glam::{EulerRot, Quat, Vec2, Vec3};
use godot::{
    classes::{
        Camera3D, DirectionalLight3D, Environment, INode3D, InputEvent, InputEventMouseButton,
        InputEventMouseMotion, MeshInstance3D, PlaneMesh, StandardMaterial3D, WorldEnvironment,
        base_material_3d::TextureParam, environment,
    },
    global::MouseButton,
    prelude::*,
};
use rusqlite::{Connection, OpenFlags};

use crate::{
    GameClient,
    assets::{
        build_model,
        creature::{cache_model_files, cache_model_textures, local_resolver},
        material::shared_texture,
        read_model,
    },
    particle_debug::Orbit,
    scene_export::{SceneEntry, child_3d, debug_stage_entries, m2_source},
};
use game_engine_core::scene_snapshot::NodeProps;

/// `creature/wolf/wolf.m2`, the original screen's reference model.
const MODEL_FDID: u32 = 126487;
/// The original ground plane's grass texture.
const GRASS_FDID: u32 = 187126;
const FOCUS: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const DISTANCE: f32 = 6.0;
/// Bevy's default perspective field of view, which the original camera keeps.
const FOV_DEGREES: f32 = 45.0;
/// The original 100-yd plane with its UVs tiled 20 times.
const GROUND_SIZE: f32 = 100.0;
const GROUND_UV_TILES: f32 = 20.0;

/// The M2 debug scene: reference model, ground, light and orbit camera.
#[derive(GodotClass)]
#[class(base = Node3D, no_init)]
pub struct WowM2Debug {
    base: Base<Node3D>,
    data_root: PathBuf,
    sensitivity: f32,
    orbit: Orbit,
    dragging: bool,
    camera: Option<Gd<Camera3D>>,
    skin_fdids: [u32; 3],
    missing_textures: PackedInt32Array,
}

#[godot_api]
impl INode3D for WowM2Debug {
    fn process(&mut self, _delta: f64) {
        self.orbit.ease();
        let Some(mut camera) = self.camera.clone() else {
            return;
        };
        let eye = self.orbit.eye();
        camera.look_at_from_position(
            Vector3::new(eye.x, eye.y, eye.z),
            Vector3::new(FOCUS.x, FOCUS.y, FOCUS.z),
        );
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        let handled = orbit_input(&mut self.orbit, &mut self.dragging, self.sensitivity, event);
        if handled && let Some(mut viewport) = self.base().get_viewport() {
            viewport.set_input_as_handled();
        }
    }
}

#[godot_api]
impl WowM2Debug {
    /// The reference model and its textures, for fixtures.
    #[func]
    fn debug_state(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set("model_fdid", i64::from(MODEL_FDID));
        let skins: Vec<i32> = self.skin_fdids.iter().map(|&fdid| fdid as i32).collect();
        state.set("skin_fdids", &PackedInt32Array::from(skins.as_slice()));
        state.set("missing_textures", &self.missing_textures);
        state
    }
}

impl WowM2Debug {
    fn new(data_root: PathBuf, sensitivity: f32) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            data_root,
            sensitivity,
            orbit: Orbit::new(FOCUS, DISTANCE),
            dragging: false,
            camera: None,
            skin_fdids: [0; 3],
            missing_textures: PackedInt32Array::new(),
        })
    }

    fn attach_scene(&mut self) -> Result<(), String> {
        let ground = ground_node(&self.data_root, GROUND_SIZE, GROUND_UV_TILES)?;
        let environment = environment_node(Color::from_rgb(0.05, 0.06, 0.08), Color::WHITE);
        let light = light_node(Color::WHITE, [-0.9, -0.6, 0.0]);
        for node in [environment, light, ground] {
            self.base_mut().add_child(&node);
        }
        let mut camera = Camera3D::new_alloc();
        camera.set_name("Camera");
        camera.set_fov(FOV_DEGREES);
        self.base_mut().add_child(&camera);
        camera.make_current();
        self.camera = Some(camera);
        let mut model = self
            .load_model()
            .map_err(|error| format!("reference model {MODEL_FDID}: {error}"))?;
        model.set_name("M2DebugReferenceModel");
        model.set_rotation(Vector3::new(0.0, -std::f32::consts::FRAC_PI_2, 0.0));
        self.base_mut().add_child(&model);
        if !self.missing_textures.is_empty() {
            return Err(format!(
                "reference model {MODEL_FDID}: missing textures {:?}",
                self.missing_textures
            ));
        }
        Ok(())
    }

    /// `M2DebugScene`: Camera, Light, Ground and the reference model.
    pub(crate) fn scene_entry(&self) -> Result<SceneEntry, String> {
        let root = self.base().clone().upcast::<Node3D>();
        let model = child_3d(&root, "M2DebugReferenceModel")?;
        let mut children = debug_stage_entries(&root)?;
        children.push(SceneEntry::new(
            "ReferenceModel",
            Some(model.clone()),
            NodeProps::Object {
                kind: "reference-model".into(),
                model: m2_source(&model)?,
            },
        ));
        Ok(SceneEntry::scene("M2DebugScene", Some(root), children))
    }

    fn load_model(&mut self) -> Result<Gd<Node3D>, String> {
        let resolver = local_resolver(&self.data_root);
        let path = cache_model_files(&resolver, &self.data_root, MODEL_FDID)?;
        self.skin_fdids = preferred_skins(&self.data_root, MODEL_FDID)?;
        let path = GString::from(path.to_string_lossy().as_ref());
        let model = read_model(&path)?;
        cache_model_textures(&resolver, &self.data_root, &self.skin_fdids, &model)?;
        let (node, missing) = build_model(&model, &path, &self.skin_fdids, None)?;
        self.missing_textures = missing;
        Ok(node)
    }
}

/// The original's `CreatureDisplayMap::resolve_skin_fdids_for_model_path`: the model's
/// preferred display skins, none when no display uses it.
fn preferred_skins(data_root: &Path, model_fdid: u32) -> Result<[u32; 3], String> {
    let path = data_root.join("cache/creature_display.sqlite");
    let connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("Cannot open {}: {error}", path.display()))?;
    query_preferred_skins(&connection, model_fdid)
        .map(|skins| skins.unwrap_or([0; 3]))
        .map_err(|error| format!("Cannot query skins of {model_fdid}: {error}"))
}

/// A debug scene's clear colour and ambient light colour.
pub(crate) fn environment_node(clear: Color, ambient: Color) -> Gd<Node> {
    let mut environment = Environment::new_gd();
    environment.set_background(environment::BgMode::COLOR);
    environment.set_bg_color(clear);
    environment.set_ambient_source(environment::AmbientSource::COLOR);
    environment.set_ambient_light_color(ambient);
    environment.set_ambient_light_energy(0.4);
    environment.set_tonemapper(environment::ToneMapper::LINEAR);
    let mut world = WorldEnvironment::new_alloc();
    world.set_name("Environment");
    world.set_environment(&environment);
    world.upcast()
}

/// A debug scene's shadowed directional light, rotated by Bevy XYZ Euler angles.
pub(crate) fn light_node(color: Color, [x, y, z]: [f32; 3]) -> Gd<Node> {
    let rotation = Quat::from_euler(EulerRot::XYZ, x, y, z);
    let mut light = DirectionalLight3D::new_alloc();
    light.set_name("Light");
    light.set_color(color);
    light.set_shadow(true);
    light.set_quaternion(Quaternion::new(
        rotation.x, rotation.y, rotation.z, rotation.w,
    ));
    light.upcast()
}

/// The original debug scenes' original orbit controls: left drag orbits, the wheel
/// zooms. `true` when `event` was theirs.
pub(crate) fn orbit_input(
    orbit: &mut Orbit,
    dragging: &mut bool,
    sensitivity: f32,
    event: Gd<InputEvent>,
) -> bool {
    if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
        if *dragging {
            let relative = motion.get_relative();
            orbit.drag(Vec2::new(relative.x, relative.y), sensitivity);
        }
        return *dragging;
    }
    let Ok(button) = event.try_cast::<InputEventMouseButton>() else {
        return false;
    };
    match button.get_button_index() {
        MouseButton::LEFT => {
            *dragging = button.is_pressed();
            true
        }
        MouseButton::WHEEL_UP if button.is_pressed() => {
            orbit.zoom(button.get_factor().max(1.0));
            true
        }
        MouseButton::WHEEL_DOWN if button.is_pressed() => {
            orbit.zoom(-button.get_factor().max(1.0));
            true
        }
        _ => false,
    }
}

/// The original debug scenes' grass plane: `size` yd, its texture tiled `tiles` times.
pub(crate) fn ground_node(data_root: &Path, size: f32, tiles: f32) -> Result<Gd<Node>, String> {
    let mut missing = PackedInt32Array::new();
    let grass = shared_texture(GRASS_FDID, &data_root.join("textures"), &mut missing)?
        .ok_or_else(|| format!("missing ground texture {GRASS_FDID}"))?;
    let mut material = StandardMaterial3D::new_gd();
    material.set_texture(TextureParam::ALBEDO, &grass);
    material.set_uv1_scale(Vector3::new(tiles, tiles, 1.0));
    material.set_roughness(0.9);
    let mut plane = PlaneMesh::new_gd();
    plane.set_size(Vector2::new(size, size));
    plane.set_material(&material);
    let mut ground = MeshInstance3D::new_alloc();
    ground.set_name("Ground");
    ground.set_mesh(&plane);
    Ok(ground.upcast())
}

impl GameClient {
    /// Shows the M2 debug scene in place of the login screen.
    pub(super) fn open_m2_debug(&mut self) -> Result<(), String> {
        if let Some(login) = self.login_ui.as_mut() {
            login.set_visible(false);
        }
        let mut scene = WowM2Debug::new(
            self.data_root.clone(),
            self.client_options.camera.mouse_sensitivity,
        );
        scene.set_name("M2Debug");
        let attached = scene.bind_mut().attach_scene();
        if let Err(error) = attached {
            scene.free();
            return Err(format!("M2 debug: {error}"));
        }
        self.base_mut().add_child(&scene);
        Ok(())
    }
}
