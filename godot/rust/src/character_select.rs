//! Selected-roster character and authored campsite ownership, independent of UI projection.

mod background;

use std::path::PathBuf;

use game_engine_core::{
    char_select_camera_data::solo_camera_params, customization_data::ModelPresentation,
    npc_appearance_assets::load_customization_db,
};
use godot::{
    classes::{Camera3D, Node3D},
    prelude::*,
};
use shared::protocol::CharacterListEntry;

use crate::assets::player::load_player_model;
use background::{Background, wow_position};

struct Preview {
    character: CharacterListEntry,
    root: Gd<Node3D>,
    model: Gd<Node3D>,
    camera: Gd<Camera3D>,
    background: Background,
    presentation: ModelPresentation,
}

impl Preview {
    fn sync(&mut self, minutes: f32) -> Result<(), String> {
        self.background.sync(&mut self.root, &self.model, minutes)?;
        let authored = wow_position(self.background.placement.position);
        let mut position = authored;
        if let Some(height) = self.background.height_at(position) {
            position.y = position.y.max(height);
        }
        self.model.set_position(position);
        self.sync_camera_and_facing(authored);
        Ok(())
    }

    fn sync_camera_and_facing(&mut self, authored: Vector3) {
        let scene = &self.background.scene;
        let (eye, focus, fov) = solo_camera_params(
            glam::Vec3::from_array(wow_position(scene.position).to_array()),
            glam::Vec3::from_array(wow_position(scene.look_at).to_array()),
            scene.fov,
            glam::Vec3::from_array(authored.to_array()),
            self.presentation,
        );
        let mut eye = Vector3::from_array(eye.to_array());
        let direction = eye - authored;
        let yaw = if direction.x == 0.0 && direction.z == 0.0 {
            self.background.placement.rotation.to_radians() - std::f32::consts::FRAC_PI_2
        } else {
            -direction.z.atan2(direction.x)
        };
        self.model.set_rotation(Vector3::new(0.0, yaw, 0.0));
        if let Some(height) = self.background.height_at(eye) {
            eye.y = eye.y.max(height + 0.5);
        }
        self.camera.set_fov(fov);
        self.camera
            .look_at_from_position(eye, Vector3::from_array(focus.to_array()));
    }
}

pub(crate) struct CharacterPreview {
    data_root: PathBuf,
    cache_root: PathBuf,
    preview: Option<Preview>,
}

impl CharacterPreview {
    pub fn new(data_root: PathBuf, cache_root: PathBuf) -> Self {
        Self {
            data_root,
            cache_root,
            preview: None,
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
    ) -> Result<(), String> {
        if self.preview.as_ref().map(|preview| &preview.character) != character {
            self.reset();
            if let Some(character) = character {
                self.load(parent, character)?;
            }
        }
        if let Some(preview) = self.preview.as_mut() {
            preview.sync(minutes)?;
        }
        Ok(())
    }

    fn load(
        &mut self,
        parent: &mut Gd<Node3D>,
        character: &CharacterListEntry,
    ) -> Result<(), String> {
        let background = Background::load(self.data_root.clone(), self.cache_root.clone())?;
        let db = load_customization_db(&self.data_root)?;
        let presentation = db.presentation_for(character.race, character.appearance.sex);
        let mut model = load_player_model(&self.data_root, &self.cache_root, character)?;
        model.set_name("SelectedCharacter");
        model.set_scale(Vector3::ONE * presentation.customize_scale.max(0.01));
        let mut root = Node3D::new_alloc();
        root.set_name("CharacterSelectScene");
        root.add_child(&model);
        parent.add_child(&root);
        let mut camera = Camera3D::new_alloc();
        camera.set_name("Camera");
        camera.set_near(0.1);
        camera.set_far(2000.0);
        root.add_child(&camera);
        camera.make_current();
        self.preview = Some(Preview {
            character: character.clone(),
            root,
            model,
            camera,
            background,
            presentation,
        });
        Ok(())
    }
}
