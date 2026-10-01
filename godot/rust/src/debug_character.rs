//! `--screen debugcharacter`: the original geoset debug scene
//! (`src/scenes/geoset_debug/mod.rs`) — two copies of one character side by side, the
//! left with geoset-heavy gear, the right with runtime-model gear, on a grass plane
//! under a warm directional light and an orbit camera. Needs no server.
//!
//! `DEBUG_CHARACTER_*` environment variables override race, class, sex, appearance and
//! every display ID, as in the original; an unparsable value is an error.

use std::path::Path;

use glam::Vec3;
use godot::{
    classes::{Camera3D, INode3D, InputEvent},
    prelude::*,
};
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Player,
};

use crate::{
    GameClient,
    assets::player::load_player_model,
    m2_debug::{environment_node, ground_node, light_node, orbit_input},
    particle_debug::Orbit,
};

const FOCUS: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const EYE: Vec3 = Vec3::new(0.0, 1.8, 6.0);
/// `particle_debug::Orbit`'s fixed base pitch, which the original overrides here.
const ORBIT_BASE_PITCH: f32 = 0.15;
const FOV_DEGREES: f32 = 45.0;
const GROUND_SIZE: f32 = 30.0;
const GROUND_UV_TILES: f32 = 6.0;
const SIDE_OFFSET: f32 = 1.7;

/// The original `DebugCharacterConfig`, read from the environment.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DebugCharacterConfig {
    pub race: u8,
    pub class: u8,
    pub appearance: CharacterAppearance,
    pub shoulder: u32,
    pub back: u32,
    pub chest: u32,
    pub left: SideGear,
    pub right: SideGear,
}

/// Display IDs that differ between the two characters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SideGear {
    pub head: u32,
    pub hands: u32,
    pub waist: u32,
    pub legs: u32,
    pub feet: u32,
}

impl DebugCharacterConfig {
    /// `lookup` reads one variable; the original's defaults fill unset ones.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let number = |name: &str, default: u32| -> Result<u32, String> {
            lookup(name).map_or(Ok(default), |value| {
                value
                    .parse()
                    .map_err(|error| format!("{name}={value:?}: {error}"))
            })
        };
        let byte = |name: &str, default: u8| -> Result<u8, String> {
            u8::try_from(number(name, u32::from(default))?)
                .map_err(|error| format!("{name}: {error}"))
        };
        let side = |prefix: &str, defaults: [u32; 5]| -> Result<SideGear, String> {
            let var = |slot: &str| format!("DEBUG_CHARACTER_{prefix}_{slot}_DISPLAY");
            Ok(SideGear {
                head: number(&var("HEAD"), defaults[0])?,
                hands: number(&var("HANDS"), defaults[1])?,
                waist: number(&var("WAIST"), defaults[2])?,
                legs: number(&var("LEGS"), defaults[3])?,
                feet: number(&var("FEET"), defaults[4])?,
            })
        };
        Ok(Self {
            race: byte("DEBUG_CHARACTER_RACE", 1)?,
            class: byte("DEBUG_CHARACTER_CLASS", 1)?,
            appearance: CharacterAppearance {
                sex: byte("DEBUG_CHARACTER_SEX", 0)?,
                skin_color: byte("DEBUG_CHARACTER_SKIN_COLOR", 2)?,
                face: byte("DEBUG_CHARACTER_FACE", 3)?,
                eye_color: byte("DEBUG_CHARACTER_EYE_COLOR", 0)?,
                hair_style: byte("DEBUG_CHARACTER_HAIR_STYLE", 4)?,
                hair_color: byte("DEBUG_CHARACTER_HAIR_COLOR", 5)?,
                facial_style: byte("DEBUG_CHARACTER_FACIAL_STYLE", 1)?,
                customization_choices: Vec::new(),
                visage: None,
            },
            shoulder: number("DEBUG_CHARACTER_SHOULDER_DISPLAY", 148865)?,
            back: number("DEBUG_CHARACTER_BACK_DISPLAY", 181925)?,
            chest: number("DEBUG_CHARACTER_CHEST_DISPLAY", 175942)?,
            // Plate helm and hood runtime models; texture-only cloth glove 510 against
            // leather glove runtime model 154616; belt buckle 109162; hybrid legs 159629
            // against geoset-only legs 73783.
            left: side("LEFT", [1128, 510, 109162, 159629, 154620])?,
            right: side("RIGHT", [685129, 154616, 160997, 73783, 154620])?,
        })
    }

    /// One side's equipment: shared shoulder, back and chest, then its own slots;
    /// display 0 leaves a slot empty.
    pub fn equipment(&self, side: &SideGear) -> EquipmentAppearance {
        let entries = [
            (EquipmentVisualSlot::Head, side.head),
            (EquipmentVisualSlot::Shoulder, self.shoulder),
            (EquipmentVisualSlot::Back, self.back),
            (EquipmentVisualSlot::Chest, self.chest),
            (EquipmentVisualSlot::Hands, side.hands),
            (EquipmentVisualSlot::Waist, side.waist),
            (EquipmentVisualSlot::Legs, side.legs),
            (EquipmentVisualSlot::Feet, side.feet),
        ]
        .into_iter()
        .filter(|(_, display)| *display != 0)
        .map(|(slot, display)| EquippedAppearanceEntry {
            slot,
            item_id: None,
            display_info_id: Some(display),
            inventory_type: 0,
            hidden: false,
        })
        .collect();
        EquipmentAppearance { entries }
    }
}

/// The geoset debug scene: two characters, ground, light and orbit camera.
#[derive(GodotClass)]
#[class(base = Node3D, no_init)]
pub struct WowDebugCharacter {
    base: Base<Node3D>,
    sensitivity: f32,
    orbit: Orbit,
    dragging: bool,
    camera: Option<Gd<Camera3D>>,
}

#[godot_api]
impl INode3D for WowDebugCharacter {
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

/// The original orbit: focus (0, 1, 0), eye (0, 1.8, 6), zoom 1.5 to 12 yd.
fn framing_orbit() -> Orbit {
    let offset = EYE - FOCUS;
    let distance = offset.length();
    let mut orbit = Orbit::new(FOCUS, distance);
    orbit.pitch = (offset.y / distance).asin() - ORBIT_BASE_PITCH;
    orbit.min_distance = 1.5;
    orbit.max_distance = 12.0;
    orbit
}

impl WowDebugCharacter {
    fn attach_scene(&mut self, data_root: &Path, config: &DebugCharacterConfig) -> Result<(), String> {
        let environment = environment_node(
            Color::from_rgb(0.05, 0.06, 0.08),
            Color::from_rgb(1.0, 0.95, 0.85),
        );
        let light = light_node(
            Color::from_rgb(1.0, 0.92, 0.8),
            [-std::f32::consts::FRAC_PI_4, std::f32::consts::FRAC_PI_6, 0.0],
        );
        let ground = ground_node(data_root, GROUND_SIZE, GROUND_UV_TILES)?;
        for node in [environment, light, ground] {
            self.base_mut().add_child(&node);
        }
        let mut camera = Camera3D::new_alloc();
        camera.set_name("Camera");
        camera.set_fov(FOV_DEGREES);
        self.base_mut().add_child(&camera);
        camera.make_current();
        self.camera = Some(camera);
        let player = Player {
            name: String::new(),
            race: config.race,
            class: config.class,
            appearance: config.appearance.clone(),
        };
        for (name, x, side) in [
            ("DebugCharacterGeoset", -SIDE_OFFSET, &config.left),
            ("DebugCharacterM2", SIDE_OFFSET, &config.right),
        ] {
            let mut model = load_player_model(data_root, &player, &config.equipment(side))
                .map_err(|error| format!("{name}: {error}"))?;
            model.set_name(name);
            model.set_position(Vector3::new(x, 0.0, 0.0));
            model.set_rotation(Vector3::new(0.0, -std::f32::consts::FRAC_PI_2, 0.0));
            self.base_mut().add_child(&model);
        }
        Ok(())
    }
}

impl GameClient {
    /// Shows the geoset debug scene in place of the login screen.
    pub(super) fn open_debug_character(&mut self) -> Result<(), String> {
        let config = DebugCharacterConfig::from_lookup(|name| std::env::var(name).ok())?;
        godot_print!("debugcharacter displays: {config:?}");
        if let Some(login) = self.login_ui.as_mut() {
            login.set_visible(false);
        }
        let sensitivity = self.client_options.camera.mouse_sensitivity;
        let mut scene = Gd::from_init_fn(|base| WowDebugCharacter {
            base,
            sensitivity,
            orbit: framing_orbit(),
            dragging: false,
            camera: None,
        });
        scene.set_name("DebugCharacter");
        let attached = scene.bind_mut().attach_scene(&self.data_root, &config);
        if let Err(error) = attached {
            scene.free();
            return Err(format!("Debug character: {error}"));
        }
        self.base_mut().add_child(&scene);
        Ok(())
    }
}

#[cfg(test)]
#[path = "debug_character_tests.rs"]
mod tests;
