//! `CharacterModelScene`: the local player's current visual rendered in its own world
//! behind the paperdoll, re-dressed whenever the replicated appearance changes.

use godot::classes::control::{LayoutPreset, MouseFilter};
use godot::classes::sub_viewport::UpdateMode;
use godot::classes::texture_rect::{ExpandMode, StretchMode};
use godot::classes::{
    Camera3D, Control, DirectionalLight3D, MeshInstance3D, Node3D, ShaderMaterial, SubViewport,
    TextureRect,
};
use godot::prelude::*;
use shared::components::SheathState;

use crate::GameClient;
use crate::world_models::{UnitAppearance, mesh_bounds};

/// Vertical field of view of the sheet camera, degrees.
const CAMERA_FOV: f32 = 30.0;
/// Space kept above and below the model, as a fraction of its height.
const FRAME_MARGIN: f32 = 0.08;
/// Uniform model-scene light: ambient fill plus a frontal key light.
const AMBIENT: Vector3 = Vector3::new(0.55, 0.55, 0.55);
const DIRECT: Vector3 = Vector3::new(0.65, 0.62, 0.58);

#[derive(Default)]
pub(crate) struct ModelPreview {
    scene: Option<Scene>,
    /// The appearance the shown model was built from.
    shown: Option<UnitAppearance>,
    /// Detached world request still loading, and its appearance.
    pending: Option<(u64, UnitAppearance)>,
    /// User turn from the rotate drag, radians.
    pub(crate) yaw: f32,
}

struct Scene {
    view: Gd<TextureRect>,
    viewport: Gd<SubViewport>,
    root: Gd<Node3D>,
    camera: Gd<Camera3D>,
    model: Option<Gd<Node3D>>,
}

impl ModelPreview {
    pub(crate) fn reset(&mut self) {
        if let Some(scene) = self.scene.take() {
            scene.view.free();
        }
        self.shown = None;
        self.pending = None;
        self.yaw = 0.0;
    }
}

impl GameClient {
    pub(super) fn sync_character_model(&mut self) -> Result<(), String> {
        if !self.character_frame.is_open() {
            self.character_frame.preview.reset();
            return Ok(());
        }
        let Some(host) = self.character_model_host()? else {
            return Ok(());
        };
        let preview = &mut self.character_frame.preview;
        if preview.scene.is_none() {
            preview.scene = Some(Scene::new(host));
        }
        self.request_character_model();
        self.attach_character_model()?;
        let preview = &mut self.character_frame.preview;
        let scene = preview.scene.as_mut().expect("scene created above");
        scene.fit_viewport();
        if let Some(model) = scene.model.as_mut() {
            model.set_rotation(Vector3::new(0.0, preview.yaw, 0.0));
        }
        Ok(())
    }

    fn character_model_host(&self) -> Result<Option<Gd<Control>>, String> {
        let Some(ui) = self.character_frame.ui.as_ref() else {
            return Ok(None);
        };
        let ui = ui.bind();
        let host = ui
            .frame_control(game_engine_ui_model::character_frame::MODEL_SCENE)
            .ok_or("CharacterModelScene control missing")?;
        Ok(Some(host))
    }

    fn request_character_model(&mut self) {
        let Some(appearance) = self.world.local_player_appearance().cloned() else {
            return;
        };
        let preview = &self.character_frame.preview;
        let current = preview
            .pending
            .as_ref()
            .map(|(_, appearance)| appearance)
            .or(preview.shown.as_ref());
        if current == Some(&appearance) {
            return;
        }
        let id = self.world.request_detached_visual(&appearance);
        self.character_frame.preview.pending = Some((id, appearance));
    }

    fn attach_character_model(&mut self) -> Result<(), String> {
        let Some((id, _)) = self.character_frame.preview.pending else {
            return Ok(());
        };
        let Some(loaded) = self.world.take_detached_visual(id) else {
            return Ok(());
        };
        let (_, appearance) = self
            .character_frame
            .preview
            .pending
            .take()
            .expect("pending");
        let model = loaded?;
        // Retail's character sheet shows the weapons sheathed.
        if let Err(error) =
            self.world
                .place_player_weapons(&model, &appearance, SheathState::Unarmed)
        {
            model.free();
            return Err(error);
        }
        bind_sheet_light(&model);
        let preview = &mut self.character_frame.preview;
        preview.shown = Some(appearance);
        preview
            .scene
            .as_mut()
            .ok_or("Character model scene missing")?
            .replace_model(model);
        Ok(())
    }
}

impl Scene {
    fn new(mut host: Gd<Control>) -> Self {
        let (viewport, root, camera) = model_viewport();
        let mut view = TextureRect::new_alloc();
        view.set_name("CharacterModelView");
        view.set_mouse_filter(MouseFilter::IGNORE);
        view.set_expand_mode(ExpandMode::IGNORE_SIZE);
        view.set_stretch_mode(StretchMode::SCALE);
        view.add_child(&viewport);
        host.add_child(&view);
        view.set_anchors_and_offsets_preset(LayoutPreset::FULL_RECT);
        view.set_texture(&viewport.get_texture().expect("SubViewport has a texture"));
        Self {
            view,
            viewport,
            root,
            camera,
            model: None,
        }
    }

    /// Render at the scene's physical pixel size.
    fn fit_viewport(&mut self) {
        let size = self.view.get_global_rect().size;
        let size = Vector2i::new(
            size.x.round().max(1.0) as i32,
            size.y.round().max(1.0) as i32,
        );
        if self.viewport.get_size() != size {
            self.viewport.set_size(size);
        }
    }

    fn replace_model(&mut self, model: Gd<Node3D>) {
        if let Some(previous) = self.model.take() {
            previous.free();
        }
        self.root.add_child(&model);
        self.frame_camera(&model);
        self.model = Some(model);
    }

    /// Face the model and fit its height into the frame.
    fn frame_camera(&mut self, model: &Gd<Node3D>) {
        let bounds = mesh_bounds(model);
        let height = bounds.size.y.max(0.1) * (1.0 + 2.0 * FRAME_MARGIN);
        let center = bounds.center();
        let distance = height / 2.0 / (CAMERA_FOV.to_radians() / 2.0).tan();
        let eye = center + Vector3::new(distance, 0.0, 0.0);
        self.camera.look_at_from_position(eye, center);
    }
}

/// A transparent own-world viewport with the sheet camera and its light.
fn model_viewport() -> (Gd<SubViewport>, Gd<Node3D>, Gd<Camera3D>) {
    let mut viewport = SubViewport::new_alloc();
    viewport.set_name("CharacterModelViewport");
    viewport.set_use_own_world_3d(true);
    viewport.set_transparent_background(true);
    viewport.set_update_mode(UpdateMode::ALWAYS);
    let mut root = Node3D::new_alloc();
    root.set_name("CharacterModelSceneRoot");
    let mut camera = Camera3D::new_alloc();
    camera.set_name("Camera");
    camera.set_fov(CAMERA_FOV);
    camera.set_near(0.05);
    camera.set_far(100.0);
    // m2.gdshader lights opaque batches inside a directional light pass.
    let mut light = DirectionalLight3D::new_alloc();
    light.set_name("SceneLight");
    root.add_child(&camera);
    root.add_child(&light);
    viewport.add_child(&root);
    (viewport, root, camera)
}

/// The scene light uniforms of every batch (`TerrainLight::bind_model`'s inputs).
fn bind_sheet_light(visual: &Gd<Node3D>) {
    let meshes = visual
        .find_children_ex("*")
        .type_("MeshInstance3D")
        .owned(false)
        .done();
    for node in meshes.iter_shared() {
        let mesh = node.cast::<MeshInstance3D>();
        let Some(material) = mesh.get_material_override() else {
            continue;
        };
        let mut material = material.cast::<ShaderMaterial>();
        for name in ["ambient", "horizon_ambient", "ground_ambient"] {
            material.set_shader_parameter(name, &AMBIENT.to_variant());
        }
        material.set_shader_parameter("direct", &DIRECT.to_variant());
        let sun = Vector3::new(1.0, 0.6, 0.3).normalized();
        material.set_shader_parameter("sun_direction", &sun.to_variant());
    }
}

/// Visual slots of a player appearance, for automation.
pub(super) fn appearance_slots(appearance: Option<&UnitAppearance>) -> VarArray {
    let mut slots = VarArray::new();
    if let Some(UnitAppearance::Player(_, equipment)) = appearance {
        for entry in &equipment.entries {
            slots.push(&format!("{:?}", entry.slot).to_variant());
        }
    }
    slots
}

impl ModelPreview {
    /// The shown model, its appearance's visual slots and whether a re-dress is loading.
    pub(super) fn snapshot(&self, state: &mut VarDictionary) {
        let model = self.scene.as_ref().and_then(|scene| scene.model.as_ref());
        state.set("model_shown", model.is_some());
        let bounds = model.map(mesh_bounds).unwrap_or_default();
        state.set("model_bounds", &bounds.to_variant());
        state.set("model_pending", self.pending.is_some());
        state.set("model_slots", &appearance_slots(self.shown.as_ref()));
        state.set("model_yaw", self.yaw);
    }
}
