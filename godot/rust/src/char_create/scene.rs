//! Authored race backdrop and selected character behind the creation screen
//! (original `src/scenes/char_create/scene.rs`).

use std::{
    f32::consts::FRAC_PI_2,
    fs,
    path::{Path, PathBuf},
};

use game_engine_core::{
    asset::m2_format::{m2_camera::parse_camera_snapshot, m2_light},
    creation_scene_data::{CreationSceneCatalog, Framing, normalize_scene, vertical_fov},
    customization_data::CustomizationDb,
};
use godot::{
    classes::{Camera3D, DirectionalLight3D, MeshInstance3D, Node3D, ShaderMaterial},
    prelude::*,
};
use shared::components::{CharacterAppearance, EquipmentAppearance, Player};

use super::{CharCreateState, camera_orbit::CreationOrbit};
use crate::assets::{
    build_model,
    creature::{cache_model_files, cache_model_textures, local_resolver},
    player::load_player_model,
    read_model,
};

struct Backdrop {
    fdid: u32,
    node: Gd<Node3D>,
    framing: Framing,
    ambient: Vector3,
}

#[derive(PartialEq)]
struct CharacterKey {
    race: u8,
    class: u8,
    appearance: CharacterAppearance,
}

struct Scene {
    root: Gd<Node3D>,
    camera: Gd<Camera3D>,
    orbit: CreationOrbit,
    backdrop: Backdrop,
    character: Option<(CharacterKey, Gd<Node3D>)>,
}

pub(crate) struct CreationScene {
    data_root: PathBuf,
    catalog: Option<CreationSceneCatalog>,
    scene: Option<Scene>,
}

impl CreationScene {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            data_root,
            catalog: None,
            scene: None,
        }
    }

    pub fn reset(&mut self) {
        if let Some(scene) = self.scene.take() {
            scene.root.free();
        }
    }

    /// `drag` is the scaled left-drag orbit delta for this frame.
    pub fn sync(
        &mut self,
        parent: &mut Gd<Node3D>,
        state: &mut CharCreateState,
        db: &CustomizationDb,
        aspect: f32,
        drag: glam::Vec2,
        delta_secs: f32,
    ) -> Result<(), String> {
        let fdid = self.catalog()?.lookup(state.selected_race)?;
        if self
            .scene
            .as_ref()
            .is_none_or(|scene| scene.backdrop.fdid != fdid)
        {
            let backdrop = load_backdrop(&self.data_root, fdid)?;
            self.replace_backdrop(parent, backdrop);
        }
        let scene = self.scene.as_mut().expect("backdrop loaded above");
        scene.sync_character(&self.data_root, state, db)?;
        if let Some(control) = state.camera_action.take() {
            scene.orbit.apply_control(control);
        }
        if drag != glam::Vec2::ZERO {
            scene.orbit.drag(drag);
        }
        let dropdown = state.open_dropdown.and_then(|id| {
            db.option_by_id(state.customization_race(), state.selected_sex, id)
                .map(|option| option.option_type)
        });
        let presentation = db.presentation_for(state.customization_race(), state.selected_sex);
        scene.orbit.ease_toward(dropdown, presentation, delta_secs);
        scene.place_camera(aspect);
        Ok(())
    }

    fn catalog(&mut self) -> Result<&CreationSceneCatalog, String> {
        if self.catalog.is_none() {
            self.catalog = Some(CreationSceneCatalog::load(
                &self.data_root.join("ChrRaces.csv"),
            )?);
        }
        Ok(self.catalog.as_ref().expect("catalog loaded above"))
    }

    fn replace_backdrop(&mut self, parent: &mut Gd<Node3D>, backdrop: Backdrop) {
        let Some(scene) = self.scene.as_mut() else {
            let mut root = Node3D::new_alloc();
            root.set_name("CharacterCreateScene");
            root.add_child(&backdrop.node);
            let mut camera = Camera3D::new_alloc();
            camera.set_name("Camera");
            root.add_child(&camera);
            // m2.gdshader lights opaque batches only inside a directional light pass;
            // with `direct` zero this contributes the authored ambient alone.
            let mut light = DirectionalLight3D::new_alloc();
            light.set_name("SceneLight");
            root.add_child(&light);
            parent.add_child(&root);
            camera.make_current();
            self.scene = Some(Scene {
                root,
                camera,
                orbit: CreationOrbit::new(backdrop.framing.eye, backdrop.framing.focus),
                backdrop,
                character: None,
            });
            return;
        };
        scene.root.add_child(&backdrop.node);
        scene.orbit = CreationOrbit::new(backdrop.framing.eye, backdrop.framing.focus);
        let previous = std::mem::replace(&mut scene.backdrop, backdrop);
        previous.node.free();
        if let Some((_, node)) = &scene.character {
            bind_scene_ambient(node, scene.backdrop.ambient);
        }
    }
}

impl Scene {
    fn sync_character(
        &mut self,
        data_root: &Path,
        state: &CharCreateState,
        db: &CustomizationDb,
    ) -> Result<(), String> {
        let key = CharacterKey {
            race: state.customization_race(),
            class: state.selected_class,
            appearance: state.appearance.clone(),
        };
        if self
            .character
            .as_ref()
            .is_some_and(|(shown, _)| *shown == key)
        {
            return Ok(());
        }
        if let Some((_, node)) = self.character.take() {
            node.free();
        }
        let player = Player {
            name: String::new(),
            race: key.race,
            class: key.class,
            appearance: key.appearance.clone(),
        };
        // Original creation previews the unequipped body.
        let mut model = load_player_model(data_root, &player, &EquipmentAppearance::default())?;
        model.set_name("CreationCharacter");
        model.set_rotation(Vector3::new(0.0, -FRAC_PI_2, 0.0));
        let scale = db
            .presentation_for(key.race, key.appearance.sex)
            .customize_scale;
        model.set_scale(Vector3::ONE * scale);
        bind_scene_ambient(&model, self.backdrop.ambient);
        self.root.add_child(&model);
        self.character = Some((key, model));
        Ok(())
    }

    fn place_camera(&mut self, aspect: f32) {
        let framing = self.backdrop.framing;
        self.camera
            .set_fov(vertical_fov(framing.fov, aspect).to_degrees());
        self.camera.set_near(framing.near);
        self.camera.set_far(framing.far);
        self.camera.look_at_from_position(
            Vector3::from_array(self.orbit.eye().to_array()),
            Vector3::from_array(self.orbit.focus.to_array()),
        );
    }
}

fn load_backdrop(data_root: &Path, fdid: u32) -> Result<Backdrop, String> {
    let resolver = local_resolver(data_root);
    let path = cache_model_files(&resolver, data_root, fdid)?;
    let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let camera = parse_camera_snapshot(&bytes)?;
    let lights = m2_light::parse_file_lights(&bytes)?;
    let gpath = GString::from(path.to_string_lossy().as_ref());
    let model = read_model(&gpath)?;
    cache_model_textures(&resolver, data_root, &[0; 3], &model)?;
    let anchor = model
        .attachments
        .iter()
        .find(|point| point.id == 0)
        .ok_or_else(|| format!("creation scene {fdid}: attachment 0 missing"))?;
    let normalized = normalize_scene(&camera, anchor.position);
    let ambient = Vector3::from_array(m2_light::scene_ambient(&lights, &model.global_sequences));
    let (mut node, missing) = build_model(&model, &gpath, &[0; 3], None)?;
    if !missing.is_empty() {
        godot_warn!("Creation scene {fdid} missing authored texture FDIDs: {missing:?}");
    }
    node.set_name(&format!("CharCreateBackdrop_{fdid}"));
    let rotation = normalized.rotation.to_array();
    node.set_transform(Transform3D::new(
        Basis::from_quaternion(Quaternion::new(
            rotation[0],
            rotation[1],
            rotation[2],
            rotation[3],
        )),
        Vector3::from_array(normalized.translation.to_array()),
    ));
    // Retail lights a standalone model scene with the model's own ambient.
    bind_scene_ambient(&node, ambient);
    Ok(Backdrop {
        fdid,
        node,
        framing: normalized.framing,
        ambient,
    })
}

/// `RetailSceneLight::m2_scene`: uniform ambient, no direct light or fog.
fn bind_scene_ambient(visual: &Gd<Node3D>, ambient: Vector3) {
    let meshes = visual
        .find_children_ex("*")
        .type_("MeshInstance3D")
        .owned(false)
        .done();
    for node in meshes.iter_shared() {
        let mesh = node.cast::<MeshInstance3D>();
        // The native M2 loader creates one surface and one ShaderMaterial per batch.
        let mut material = mesh
            .get_surface_override_material(0)
            .expect("M2 batch has an authored material")
            .cast::<ShaderMaterial>();
        for name in ["ambient", "horizon_ambient", "ground_ambient"] {
            material.set_shader_parameter(name, &ambient.to_variant());
        }
        material.set_shader_parameter("direct", &Vector3::ZERO.to_variant());
    }
}
