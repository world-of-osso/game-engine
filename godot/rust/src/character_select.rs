//! Selected-roster character and authored campsite ownership, independent of UI projection.

mod background;
mod objects;
pub(crate) mod sky;

use std::path::{Path, PathBuf};

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

use crate::{
    assets::player::{PlayerParts, build_player_model, prepare_player_parts},
    background_load::BackgroundLoad,
    scene_export::{SceneEntry, camera_entry, character_entry},
};
use background::{Background, wow_position};

/// The roster character shown in the campsite.
struct Shown {
    entry: CharacterListEntry,
    model: Gd<Node3D>,
}

/// Campsite scene kept for the whole character-select visit; selection
/// changes swap only the character.
struct Preview {
    root: Gd<Node3D>,
    camera: Gd<Camera3D>,
    background: Background,
    /// The selected roster character; its model is `shown` once it has loaded.
    selected: Option<CharacterListEntry>,
    shown: Option<Shown>,
    /// The selected character's race/sex presentation, which frames the shot; `None`
    /// until the customization catalog has loaded.
    presentation: Option<ModelPresentation>,
    /// User orbit offset; the authored shot is recomputed every frame.
    orbit_yaw_pitch: glam::Vec2,
}

impl Preview {
    /// Select `character`: the previous character leaves at once, the orbit resets, and
    /// the new character appears once its model has loaded.
    fn select(&mut self, character: &CharacterListEntry) {
        if let Some(previous) = self.shown.take() {
            previous.model.free();
        }
        self.selected = Some(character.clone());
        self.presentation = None;
        self.orbit_yaw_pitch = glam::Vec2::ZERO;
    }

    fn show(&mut self, entry: CharacterListEntry, mut model: Gd<Node3D>, scale: f32) {
        model.set_name("SelectedCharacter");
        model.set_scale(Vector3::ONE * scale.max(0.01));
        self.root.add_child(&model);
        self.background.bind_light(&model);
        self.shown = Some(Shown { entry, model });
    }

    fn sync(&mut self, minutes: f32, drag: glam::Vec2) -> Result<(), String> {
        let model = self.shown.as_ref().map(|shown| shown.model.clone());
        // The camera from the previous frame: this frame's shot needs the terrain height.
        let camera = self.camera.get_position();
        self.background
            .sync(&mut self.root, model.as_ref(), minutes, camera)?;
        let authored = wow_position(self.background.placement.position);
        let Some(presentation) = self.presentation else {
            return Ok(());
        };
        let mut position = authored;
        if let Some(height) = self.background.height_at(position) {
            position.y = position.y.max(height);
        }
        let yaw = frame_shot(
            &mut self.background,
            &mut self.camera,
            presentation,
            &mut self.orbit_yaw_pitch,
            authored,
            drag,
        );
        if let Some(shown) = self.shown.as_mut() {
            shown.model.set_position(position);
            shown.model.set_rotation(Vector3::new(0.0, yaw, 0.0));
        }
        Ok(())
    }
}

/// Place the camera on the authored shot of a character at `authored` and return the
/// yaw that faces the character toward the shot; the orbit moves only the camera.
fn frame_shot(
    background: &mut Background,
    camera: &mut Gd<Camera3D>,
    presentation: ModelPresentation,
    orbit_yaw_pitch: &mut glam::Vec2,
    authored: Vector3,
    drag: glam::Vec2,
) -> f32 {
    let scene = &background.scene;
    let (eye, focus, fov) = solo_camera_params(
        glam::Vec3::from_array(wow_position(scene.position).to_array()),
        glam::Vec3::from_array(wow_position(scene.look_at).to_array()),
        scene.fov,
        glam::Vec3::from_array(authored.to_array()),
        presentation,
    );
    let facing_eye = Vector3::from_array(eye.to_array());
    let mut orbit = SelectOrbit::from_eye_focus(eye, focus);
    orbit.yaw = orbit_yaw_pitch.x;
    orbit.pitch = orbit_yaw_pitch.y;
    orbit.drag(drag);
    *orbit_yaw_pitch = glam::Vec2::new(orbit.yaw, orbit.pitch);
    let mut eye = Vector3::from_array(orbit.eye().to_array());
    let direction = facing_eye - authored;
    let yaw = if direction.x == 0.0 && direction.z == 0.0 {
        background.placement.rotation.to_radians() - std::f32::consts::FRAC_PI_2
    } else {
        -direction.z.atan2(direction.x)
    };
    if let Some(height) = background.height_at(eye) {
        eye.y = eye.y.max(height + 0.5);
    }
    camera.set_fov(fov);
    let focus = Vector3::from_array(focus.to_array());
    camera.look_at_from_position(eye, focus);
    background.position_sky(focus);
    yaw
}

/// A roster character's model loading on its own thread: its files are extracted and
/// parsed and its textures composed and decoded there, so selecting a character never
/// waits for them.
struct CharacterLoad {
    entry: CharacterListEntry,
    parts: BackgroundLoad<Result<PlayerParts, String>>,
}

impl CharacterLoad {
    fn start(data_root: &Path, entry: &CharacterListEntry) -> Self {
        let data_root = data_root.to_owned();
        let player = Player {
            name: entry.name.clone(),
            race: entry.race,
            class: entry.class,
            appearance: entry.appearance.clone(),
        };
        let equipment = entry.equipment_appearance.clone();
        Self {
            entry: entry.clone(),
            parts: BackgroundLoad::start("character-preview", move || {
                prepare_player_parts(&data_root, &player, &equipment)
            }),
        }
    }
}

pub(crate) struct CharacterPreview {
    data_root: PathBuf,
    /// Race/sex presentations, loaded from client start.
    customization: BackgroundLoad<Result<CustomizationDb, String>>,
    preview: Option<Preview>,
    loading: Option<CharacterLoad>,
    /// Chosen campsite; `None` shows the first authored scene.
    scene_id: Option<u32>,
}

impl CharacterPreview {
    pub fn new(data_root: PathBuf) -> Self {
        let catalog_root = data_root.clone();
        Self {
            data_root,
            customization: BackgroundLoad::start("customization-catalog", move || {
                load_customization_db(&catalog_root)
            }),
            preview: None,
            loading: None,
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

    /// `CharSelectScene` (`char_select/scene_tree.rs`): background, the shown character,
    /// camera and sun; `None` before the campsite loads.
    pub fn scene_entry(&self) -> Result<Option<SceneEntry>, String> {
        let Some(preview) = &self.preview else {
            return Ok(None);
        };
        let mut children = vec![preview.background.scene_entry()?];
        if let Some(shown) = &preview.shown {
            let entry = &shown.entry;
            children.push(character_entry(
                "Character",
                &shown.model,
                entry.race,
                entry.appearance.sex,
                (Some(entry.name.clone()), Some(entry.character_id)),
                &entry.equipment_appearance,
            )?);
        }
        children.push(camera_entry(&preview.camera));
        children.extend(preview.background.sun_entry());
        Ok(Some(SceneEntry::scene(
            "CharSelectScene",
            Some(preview.root.clone()),
            children,
        )))
    }

    pub fn reset(&mut self) {
        self.loading = None;
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
            let _span = crate::profile::span(|| "preview.load_scene".to_owned());
            self.preview = Some(self.load_scene(parent)?);
        }
        let preview = self.preview.as_mut().expect("scene loaded above");
        if preview.selected.as_ref() != Some(character) {
            preview.select(character);
        }
        self.show_selected(character)?;
        let _span = crate::profile::span(|| "preview.scene_sync".to_owned());
        self.preview
            .as_mut()
            .expect("scene loaded above")
            .sync(minutes, drag)
    }

    /// Frame the selected character once its presentation is known, and show its model
    /// once it has loaded.
    fn show_selected(&mut self, character: &CharacterListEntry) -> Result<(), String> {
        let preview = self.preview.as_mut().expect("scene loaded above");
        if preview.shown.is_some() {
            return Ok(());
        }
        if preview.presentation.is_none() {
            let db = match self.customization.poll() {
                None => None,
                Some(Ok(db)) => Some(db),
                Some(Err(error)) => return Err(format!("Customization catalog: {error}")),
            };
            preview.presentation =
                db.map(|db| db.presentation_for(character.race, character.appearance.sex));
        }
        if self
            .loading
            .as_ref()
            .is_none_or(|loading| &loading.entry != character)
        {
            self.loading = Some(CharacterLoad::start(&self.data_root, character));
        }
        let loading = self.loading.as_mut().expect("load started above");
        match loading.parts.poll() {
            None => return Ok(()),
            Some(Err(error)) => return Err(error.clone()),
            Some(Ok(_)) => {}
        }
        // Scaled by its presentation, so it waits for the catalog too.
        let Some(presentation) = preview.presentation else {
            return Ok(());
        };
        let CharacterLoad { entry, parts } = self.loading.take().expect("load polled above");
        let parts = parts
            .into_loaded()
            .expect("load polled above")
            .expect("load succeeded above");
        let _span = crate::profile::span(|| "preview.build_player_model".to_owned());
        let model = build_player_model(&self.data_root, parts)?;
        preview.show(entry, model, presentation.customize_scale);
        Ok(())
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
            selected: None,
            shown: None,
            presentation: None,
            orbit_yaw_pitch: glam::Vec2::ZERO,
        })
    }
}
