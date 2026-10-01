//! Selected-roster character and authored campsite ownership, independent of UI projection.

mod background;
mod objects;

use std::path::PathBuf;

use game_engine_core::{
    char_select_camera_data::{SelectOrbit, solo_camera_params},
    customization_data::{CustomizationDb, ModelPresentation},
    npc_appearance_assets::load_customization_db,
};
use godot::{
    classes::{Camera3D, Node3D},
    prelude::*,
};
use shared::{components::Player, protocol::CharacterListEntry};

use crate::assets::player::load_player_model;
use background::{Background, wow_position};

/// The roster character shown in the campsite.
struct Shown {
    entry: CharacterListEntry,
    model: Gd<Node3D>,
    presentation: ModelPresentation,
    /// User orbit offset; the authored shot is recomputed every frame.
    orbit_yaw_pitch: glam::Vec2,
}

/// Campsite scene kept for the whole character-select visit; selection
/// changes swap only the character.
struct Preview {
    root: Gd<Node3D>,
    camera: Gd<Camera3D>,
    background: Background,
    shown: Option<Shown>,
}

impl Preview {
    fn sync(&mut self, minutes: f32, drag: glam::Vec2) -> Result<(), String> {
        let model = self.shown.as_ref().map(|shown| shown.model.clone());
        // The camera from the previous frame: this frame's shot needs the terrain height.
        let camera = self.camera.get_position();
        self.background
            .sync(&mut self.root, model.as_ref(), minutes, camera)?;
        let authored = wow_position(self.background.placement.position);
        let Some(shown) = self.shown.as_mut() else {
            return Ok(());
        };
        let mut position = authored;
        if let Some(height) = self.background.height_at(position) {
            position.y = position.y.max(height);
        }
        shown.model.set_position(position);
        sync_camera_and_facing(
            &mut self.background,
            &mut self.camera,
            shown,
            authored,
            drag,
        );
        Ok(())
    }
}

fn sync_camera_and_facing(
    background: &mut Background,
    camera: &mut Gd<Camera3D>,
    shown: &mut Shown,
    authored: Vector3,
    drag: glam::Vec2,
) {
    let scene = &background.scene;
    let (eye, focus, fov) = solo_camera_params(
        glam::Vec3::from_array(wow_position(scene.position).to_array()),
        glam::Vec3::from_array(wow_position(scene.look_at).to_array()),
        scene.fov,
        glam::Vec3::from_array(authored.to_array()),
        shown.presentation,
    );
    // The character faces the authored shot; the orbit moves only the camera.
    let facing_eye = Vector3::from_array(eye.to_array());
    let mut orbit = SelectOrbit::from_eye_focus(eye, focus);
    orbit.yaw = shown.orbit_yaw_pitch.x;
    orbit.pitch = shown.orbit_yaw_pitch.y;
    orbit.drag(drag);
    shown.orbit_yaw_pitch = glam::Vec2::new(orbit.yaw, orbit.pitch);
    let mut eye = Vector3::from_array(orbit.eye().to_array());
    let direction = facing_eye - authored;
    let yaw = if direction.x == 0.0 && direction.z == 0.0 {
        background.placement.rotation.to_radians() - std::f32::consts::FRAC_PI_2
    } else {
        -direction.z.atan2(direction.x)
    };
    shown.model.set_rotation(Vector3::new(0.0, yaw, 0.0));
    if let Some(height) = background.height_at(eye) {
        eye.y = eye.y.max(height + 0.5);
    }
    camera.set_fov(fov);
    let focus = Vector3::from_array(focus.to_array());
    camera.look_at_from_position(eye, focus);
    background.position_sky(focus);
}

pub(crate) struct CharacterPreview {
    data_root: PathBuf,
    customization: Option<CustomizationDb>,
    preview: Option<Preview>,
    /// Chosen campsite; `None` shows the first authored scene.
    scene_id: Option<u32>,
}

impl CharacterPreview {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            data_root,
            customization: None,
            preview: None,
            scene_id: None,
        }
    }

    /// Show campsite `id` from the next sync, rebuilding the scene when it changes.
    pub fn select_scene(&mut self, id: u32) {
        if self.scene_id != Some(id) {
            self.scene_id = Some(id);
            self.reset();
        }
    }

    pub fn reset(&mut self) {
        if let Some(mut preview) = self.preview.take() {
            preview.background.clear_nodes();
            preview.root.free();
        }
    }

    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        character: Option<&CharacterListEntry>,
        minutes: f32,
        drag: glam::Vec2,
    ) -> Result<(), String> {
        let Some(character) = character else {
            self.reset();
            return Ok(());
        };
        if self.preview.is_none() {
            self.preview = Some(self.load_scene(parent)?);
        }
        let shown = self
            .preview
            .as_ref()
            .and_then(|preview| preview.shown.as_ref());
        if shown.map(|shown| &shown.entry) != Some(character) {
            let next = self.load_character(character)?;
            let preview = self.preview.as_mut().expect("scene loaded above");
            if let Some(previous) = preview.shown.replace(next) {
                previous.model.free();
            }
            let shown = preview.shown.as_ref().expect("character shown above");
            preview.root.add_child(&shown.model);
            preview.background.bind_light(&shown.model);
        }
        self.preview
            .as_mut()
            .expect("scene loaded above")
            .sync(minutes, drag)
    }

    fn load_scene(&self, parent: &mut Gd<Node3D>) -> Result<Preview, String> {
        let background = Background::load(self.data_root.clone(), self.scene_id)?;
        let mut root = Node3D::new_alloc();
        root.set_name("CharacterSelectScene");
        parent.add_child(&root);
        let mut camera = Camera3D::new_alloc();
        camera.set_name("Camera");
        camera.set_near(0.1);
        camera.set_far(2000.0);
        root.add_child(&camera);
        camera.make_current();
        Ok(Preview {
            root,
            camera,
            background,
            shown: None,
        })
    }

    fn load_character(&mut self, character: &CharacterListEntry) -> Result<Shown, String> {
        if self.customization.is_none() {
            self.customization = Some(load_customization_db(&self.data_root)?);
        }
        let db = self.customization.as_ref().expect("catalog loaded above");
        let presentation = db.presentation_for(character.race, character.appearance.sex);
        let player = Player {
            name: character.name.clone(),
            race: character.race,
            class: character.class,
            appearance: character.appearance.clone(),
        };
        let mut model =
            load_player_model(&self.data_root, &player, &character.equipment_appearance)?;
        model.set_name("SelectedCharacter");
        model.set_scale(Vector3::ONE * presentation.customize_scale.max(0.01));
        Ok(Shown {
            entry: character.clone(),
            model,
            presentation,
            orbit_yaw_pitch: glam::Vec2::ZERO,
        })
    }
}
