//! Offline authored-sky controller; source selection and procedural environment are separate.
use std::path::Path;

use game_engine_core::{
    char_select_camera_data::scaled_orbit_delta,
    client_options_data::CameraOptions,
    light_lookup_types::{LightParamsFlags, LightSkyboxFlags},
    startup_args_data::StartupArgs,
};
use glam::Vec2;
use godot::{
    classes::{
        Camera3D, INode3D, InputEvent, InputEventMouseButton, InputEventMouseMotion, Node3D,
    },
    global::MouseButton,
    prelude::*,
};

use crate::{assets::uv_animation::WowMaterialClock, character_select::sky::Sky};

mod environment;
mod source;

const FOCUS: Vector3 = Vector3::new(0.0, 1.0, 0.0);
const START_DISTANCE: f32 = 7.5;
const BASE_PITCH: f32 = 0.15;
const ZOOM_STEP: f32 = 0.4;
const ZOOM_LERP: f32 = 0.25;
const MIN_DISTANCE: f32 = 0.5;
const MAX_DISTANCE: f32 = 20.0;
// Clock and bone animation nodes process at priority 0 before this controller.
const PROCESS_PRIORITY: i32 = 1;
const MATERIAL_CLOCK_PATH: &str = "/root/M2MaterialClock";
const BASELINE_CLEAR_COLOR: Color = Color::from_rgb(0.05, 0.06, 0.08);

#[derive(Clone, Copy)]
pub(crate) struct Composition {
    pub clear_color: Color,
    pub procedural_baseline: bool,
    pub procedural_fog: bool,
    pub reference_objects: bool,
}

impl Composition {
    fn from_source(source: &source::ResolvedSource, verify: bool) -> Self {
        if verify {
            return Self {
                clear_color: Color::BLACK,
                procedural_baseline: false,
                procedural_fog: false,
                reference_objects: false,
            };
        }
        let procedural_baseline = procedural_baseline_enabled(source.flags, source.params_flags);
        Self {
            clear_color: if procedural_baseline {
                BASELINE_CLEAR_COLOR
            } else {
                Color::BLACK
            },
            procedural_baseline,
            procedural_fog: procedural_fog_enabled(source.flags, source.params_flags),
            reference_objects: true,
        }
    }
}

fn procedural_baseline_enabled(
    skybox: Option<LightSkyboxFlags>,
    params: Option<LightParamsFlags>,
) -> bool {
    let combines_sky = skybox.map_or(true, |flags| {
        flags.contains(LightSkyboxFlags::COMBINE_PROCEDURAL_AND_SKYBOX)
    });
    let suppresses_celestial = params.is_some_and(suppresses_celestial_visibility);
    combines_sky && !suppresses_celestial
}

fn suppresses_celestial_visibility(flags: LightParamsFlags) -> bool {
    [
        LightParamsFlags::DONT_INHERIT_SKYBOX,
        LightParamsFlags::HIDE_SUN,
        LightParamsFlags::HIDE_MOON,
        LightParamsFlags::HIDE_STARS,
        LightParamsFlags::OVERRIDE_CELESTIAL_SPHERE,
        LightParamsFlags::HIDE_CELESTIAL_OBJECT,
    ]
    .into_iter()
    .any(|suppression| flags.contains(suppression))
}

fn procedural_fog_enabled(
    skybox: Option<LightSkyboxFlags>,
    params: Option<LightParamsFlags>,
) -> bool {
    let blends_fog = skybox.map_or(true, |flags| {
        flags.contains(LightSkyboxFlags::PROCEDURAL_FOG_COLOR_BLEND)
    });
    let height_fog =
        params.is_some_and(|flags| flags.contains(LightParamsFlags::HEIGHT_FOG_ABOVE_PLANE));
    blends_fog || height_fog
}

/// Original debug orbit, including its intentionally unbounded pitch.
struct Orbit {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target_distance: f32,
}

impl Orbit {
    fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            distance: START_DISTANCE,
            target_distance: START_DISTANCE,
        }
    }

    fn eye(&self) -> Vector3 {
        let pitch = BASE_PITCH + self.pitch;
        FOCUS
            + Vector3::new(
                self.yaw.sin() * pitch.cos(),
                pitch.sin(),
                self.yaw.cos() * pitch.cos(),
            ) * self.distance
    }

    fn drag(&mut self, relative: Vector2, sensitivity: f32) {
        let delta = scaled_orbit_delta(Vec2::new(relative.x, relative.y), sensitivity);
        self.yaw += delta.x;
        self.pitch += delta.y;
    }

    fn zoom(&mut self, notches: f32) {
        self.target_distance =
            (self.target_distance - notches * ZOOM_STEP).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    fn ease(&mut self) {
        self.distance += (self.target_distance - self.distance) * ZOOM_LERP;
    }
}

#[derive(GodotClass)]
#[class(base = Node3D, no_init)]
pub struct WowSkyboxDebug {
    base: Base<Node3D>,
    options: CameraOptions,
    orbit: Orbit,
    dragging: bool,
    camera: Gd<Camera3D>,
    sky: Sky,
    fixed_time_ms: Option<u32>,
    // Only pending inside load; a failed environment construction frees the entire root.
    environment: Option<environment::DebugEnvironment>,
}

#[godot_api]
impl INode3D for WowSkyboxDebug {
    fn ready(&mut self) {
        self.camera.make_current();
        self.update_camera_pose();
    }

    fn process(&mut self, _delta: f64) {
        if let Err(error) = self.tick() {
            godot_error!("SkyboxDebug: {error}");
            self.base_mut().set_process(false);
            self.base_mut().set_process_input(false);
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        let consumed = self.consume_orbit_input(event);
        if consumed {
            if let Some(mut viewport) = self.base().get_viewport() {
                viewport.set_input_as_handled();
            }
        }
    }
}

impl WowSkyboxDebug {
    pub(crate) fn load(
        data_root: &Path,
        args: &StartupArgs,
        camera_options: CameraOptions,
    ) -> Result<Gd<Self>, String> {
        let source = source::resolve_source(data_root, args)?;
        let composition = Composition::from_source(&source, args.skybox_verify);
        let sky = Sky::load_model(data_root, &source.path, source.fdid, args.skybox_time_ms)?;
        let mut root = Self::from_sky(sky, camera_options, args.skybox_time_ms);
        let result = root.bind_mut().attach_environment(data_root, composition);
        if let Err(error) = result {
            root.free();
            return Err(error);
        }
        Ok(root)
    }

    fn from_sky(mut sky: Sky, options: CameraOptions, fixed_time_ms: Option<u32>) -> Gd<Self> {
        sky.node.set_position(FOCUS);
        // The constructor is outside the tree: retain the loader's initial zero/fixed sample.
        sky.sample(fixed_time_ms.unwrap_or(0));
        let mut camera = Camera3D::new_alloc();
        camera.set_name("Camera");
        camera.set_fov(options.fov_degrees);
        camera.set_current(true);
        let mut root = Gd::from_init_fn(|base| Self {
            base,
            options,
            orbit: Orbit::new(),
            dragging: false,
            camera,
            sky,
            fixed_time_ms,
            environment: None,
        });
        root.bind_mut().attach_scene_nodes();
        root
    }

    fn attach_scene_nodes(&mut self) {
        let camera = self.camera.clone();
        let sky = self.sky.node.clone();
        self.base_mut().set_name("SkyboxDebug");
        self.base_mut().set_process_priority(PROCESS_PRIORITY);
        self.base_mut().add_child(&camera);
        self.base_mut().add_child(&sky);
    }

    fn attach_environment(
        &mut self,
        data_root: &Path,
        composition: Composition,
    ) -> Result<(), String> {
        let mut root = self.base().clone();
        let environment = environment::create(&mut root, &mut self.camera, data_root, composition)?;
        self.environment = Some(environment);
        Ok(())
    }

    pub(crate) fn update_options(&mut self, options: CameraOptions) {
        self.camera.set_fov(options.fov_degrees);
        self.options = options;
    }

    fn tick(&mut self) -> Result<(), String> {
        let live_time_ms = self.read_material_clock_ms()?;
        self.environment
            .as_mut()
            .ok_or("SkyboxDebug environment was not initialized")?
            .update_time_ms(live_time_ms)?;
        self.sky.sample(self.fixed_time_ms.unwrap_or(live_time_ms));
        self.orbit.ease();
        self.update_camera_pose();
        self.sky.node.set_position(FOCUS);
        Ok(())
    }

    fn read_material_clock_ms(&self) -> Result<u32, String> {
        let clock = self
            .base()
            .try_get_node_as::<WowMaterialClock>(MATERIAL_CLOCK_PATH)
            .ok_or("SkyboxDebug requires /root/M2MaterialClock (WowMaterialClock)")?;
        Ok(clock.bind().elapsed_time_ms() as u32)
    }

    fn update_camera_pose(&mut self) {
        self.camera.look_at_from_position(self.orbit.eye(), FOCUS);
    }

    fn consume_orbit_input(&mut self, event: Gd<InputEvent>) -> bool {
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            if self.dragging {
                self.orbit
                    .drag(motion.get_screen_relative(), self.options.mouse_sensitivity);
            }
            return self.dragging;
        }
        if let Ok(button) = event.try_cast::<InputEventMouseButton>() {
            return self.consume_mouse_button(&button);
        }
        false
    }

    fn consume_mouse_button(&mut self, button: &Gd<InputEventMouseButton>) -> bool {
        match button.get_button_index() {
            MouseButton::LEFT => {
                self.dragging = button.is_pressed();
                true
            }
            MouseButton::WHEEL_UP if button.is_pressed() => {
                self.orbit.zoom(button.get_factor());
                true
            }
            MouseButton::WHEEL_DOWN if button.is_pressed() => {
                self.orbit.zoom(-button.get_factor());
                true
            }
            _ => false,
        }
    }
}
