//! Unit frame portraits (`SetPortraitTexture`, Blizzard_UnitFrame/Mainline/UnitFrame.lua:188):
//! the unit's model seen from its M2 portrait camera, rendered into the PlayerFrame and
//! TargetFrame portrait slots over black and rounded by their masks. A portrait follows its
//! unit and re-renders when the unit's visual appearance changes (`UNIT_PORTRAIT_UPDATE`).

use std::fs;

use game_engine_core::asset::m2_format::m2_camera::parse_portrait_camera;
use game_engine_core::creation_scene_data::vertical_fov;
use game_engine_ui_model::inworld_unit_frames_component::{
    PLAYER_PORTRAIT, PortraitSlot, TARGET_PORTRAIT,
};
use godot::classes::control::{LayoutPreset, MouseFilter};
use godot::classes::sub_viewport::UpdateMode;
use godot::classes::texture_rect::{ExpandMode, StretchMode};
use godot::classes::{
    Camera3D, Control, DirectionalLight3D, Node3D, Shader, ShaderMaterial, SubViewport, TextureRect,
};
use godot::prelude::*;
use shared::components::SheathState;

use crate::GameClient;
use crate::assets::{M2_BOUNDS_META, M2_SOURCE_META, wow_vec3};
use crate::character_frame::preview::bind_sheet_light;
use crate::world::WorldUnits;
use crate::world_models::UnitAppearance;

/// The model over black, kept where the mask's alpha is; outside the mask rect is clear
/// (`CLAMPTOBLACKADDITIVE`).
const MASK_SHADER: &str = "shader_type canvas_item;
uniform sampler2D mask_texture : filter_linear;
uniform vec4 mask_rect;
void fragment() {
    vec4 model = texture(TEXTURE, UV);
    vec2 mask_uv = (UV - mask_rect.xy) / mask_rect.zw;
    bool inside = all(greaterThanEqual(mask_uv, vec2(0.0))) && all(lessThanEqual(mask_uv, vec2(1.0)));
    float keep = inside ? texture(mask_texture, mask_uv).a : 0.0;
    COLOR = vec4(model.rgb * model.a, keep);
}
";

pub(crate) struct UnitPortraits {
    player: Portrait,
    target: Portrait,
}

impl Default for UnitPortraits {
    fn default() -> Self {
        Self {
            player: Portrait::new(PLAYER_PORTRAIT),
            target: Portrait::new(TARGET_PORTRAIT),
        }
    }
}

struct Portrait {
    slot: PortraitSlot,
    scene: Option<Scene>,
    /// The appearance the shown model was built from.
    shown: Option<UnitAppearance>,
    /// Detached world request still loading, and its appearance.
    pending: Option<(u64, UnitAppearance)>,
}

struct Scene {
    /// The portrait slot control the view fills.
    host: InstanceId,
    view: Gd<TextureRect>,
    material: Gd<ShaderMaterial>,
    mask_loaded: bool,
    viewport: Gd<SubViewport>,
    root: Gd<Node3D>,
    camera: Gd<Camera3D>,
    model: Option<Gd<Node3D>>,
}

impl Portrait {
    fn new(slot: PortraitSlot) -> Self {
        Self {
            slot,
            scene: None,
            shown: None,
            pending: None,
        }
    }

    fn cancel_pending(&mut self, world: &mut WorldUnits) {
        if let Some((id, _)) = self.pending.take() {
            world.cancel_detached_visual(id);
        }
    }

    fn clear(&mut self, world: &mut WorldUnits) {
        self.cancel_pending(world);
        if let Some(scene) = self.scene.take()
            && scene.view.is_instance_valid()
        {
            scene.view.free();
        }
        self.shown = None;
    }

    /// Show `appearance` in `host`: request its model when it changed, attach it once
    /// loaded, and keep the render at the slot's pixel size.
    fn sync(
        &mut self,
        world: &mut WorldUnits,
        host: Option<Gd<Control>>,
        appearance: Option<UnitAppearance>,
    ) -> Result<(), String> {
        let Some(host) = host else {
            self.clear(world);
            return Ok(());
        };
        let slot = self.slot;
        let scene = self.scene_in(host);
        scene.load_mask(&slot)?;
        scene.fit_viewport();
        let Some(appearance) = appearance else {
            scene.remove_model();
            self.cancel_pending(world);
            self.shown = None;
            return Ok(());
        };
        let current = self
            .pending
            .as_ref()
            .map(|(_, appearance)| appearance)
            .or(self.shown.as_ref());
        if current != Some(&appearance) {
            self.cancel_pending(world);
            let id = world.request_detached_visual(&appearance);
            self.pending = Some((id, appearance));
        }
        self.attach_loaded(world)
    }

    /// The scene in `host`, made anew when the slot was rebuilt (freeing the old view and
    /// the model inside it).
    fn scene_in(&mut self, host: Gd<Control>) -> &mut Scene {
        let current = self.scene.as_ref().is_some_and(|scene| {
            scene.host == host.instance_id() && scene.view.is_instance_valid()
        });
        if !current {
            if let Some(stale) = self.scene.take()
                && stale.view.is_instance_valid()
            {
                stale.view.free();
            }
            self.shown = None;
            self.scene = Some(Scene::new(host, &self.slot));
        }
        self.scene.as_mut().expect("scene made above")
    }

    fn attach_loaded(&mut self, world: &mut WorldUnits) -> Result<(), String> {
        let Some((id, _)) = self.pending else {
            return Ok(());
        };
        let Some(loaded) = world.take_detached_visual(id) else {
            return Ok(());
        };
        let (_, appearance) = self.pending.take().expect("pending");
        // Shown even when it fails, so a broken model is reported once, not re-requested.
        self.shown = Some(appearance.clone());
        let scene = self.scene.as_mut().expect("synced scene");
        scene.remove_model();
        let model = loaded?;
        // A player's weapons sheathed, as on the character sheet.
        if let Err(error) = world.place_player_weapons(&model, &appearance, SheathState::Unarmed) {
            model.free();
            return Err(error);
        }
        bind_sheet_light(&model);
        scene.root.add_child(&model);
        scene.model = Some(model.clone());
        frame_portrait(&mut scene.camera, &model)
            .map_err(|error| format!("{} portrait: {error}", self.slot.frame))
    }

    fn snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let scene = self
            .scene
            .as_ref()
            .filter(|scene| scene.view.is_instance_valid());
        state.set("exists", scene.is_some());
        let Some(scene) = scene else {
            return state;
        };
        scene.snapshot(&mut state);
        state.set("mask_fdid", i64::from(self.slot.mask_fdid));
        state.set("pending", self.pending.is_some());
        let shown = self.shown.as_ref().map(describe).unwrap_or_default();
        state.set("appearance", shown.as_str());
        state
    }
}

impl Scene {
    fn new(mut host: Gd<Control>, slot: &PortraitSlot) -> Self {
        let (viewport, root, camera) = portrait_viewport();
        let material = mask_material(slot);
        let mut view = TextureRect::new_alloc();
        view.set_name("PortraitView");
        view.set_mouse_filter(MouseFilter::IGNORE);
        view.set_expand_mode(ExpandMode::IGNORE_SIZE);
        view.set_stretch_mode(StretchMode::SCALE);
        view.set_material(&material);
        // Hidden until the mask arrives: an unmasked square is never shown.
        view.set_visible(false);
        view.add_child(&viewport);
        host.add_child(&view);
        view.set_anchors_and_offsets_preset(LayoutPreset::FULL_RECT);
        view.set_texture(&viewport.get_texture().expect("SubViewport has a texture"));
        Self {
            host: host.instance_id(),
            view,
            material,
            mask_loaded: false,
            viewport,
            root,
            camera,
            model: None,
        }
    }

    /// The view's screen rect and mask, the render and its camera, for automation.
    fn snapshot(&self, state: &mut VarDictionary) {
        state.set("rect", self.view.get_global_rect());
        state.set("visible", self.view.is_visible_in_tree());
        state.set("mask_loaded", self.mask_loaded);
        state.set(
            "mask_rect",
            &self.material.get_shader_parameter("mask_rect"),
        );
        state.set("viewport_size", self.viewport.get_size());
        state.set("model_shown", self.model.is_some());
        state.set("camera_position", self.camera.get_global_position());
        if let Some(image) = self
            .viewport
            .get_texture()
            .and_then(|texture| texture.get_image())
        {
            state.set("image", &image);
        }
    }

    fn load_mask(&mut self, slot: &PortraitSlot) -> Result<(), String> {
        if self.mask_loaded {
            return Ok(());
        }
        let Some(mask) = crate::ui::assets::load_file_data_id(slot.mask_fdid)
            .map_err(|error| format!("{} mask: {error}", slot.frame))?
        else {
            return Ok(());
        };
        self.material
            .set_shader_parameter("mask_texture", &mask.to_variant());
        self.mask_loaded = true;
        self.view.set_visible(true);
        Ok(())
    }

    /// Render at the slot's physical pixel size.
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

    fn remove_model(&mut self) {
        if let Some(model) = self.model.take() {
            model.free();
        }
    }
}

/// A transparent own-world viewport with the portrait camera and its light.
fn portrait_viewport() -> (Gd<SubViewport>, Gd<Node3D>, Gd<Camera3D>) {
    let mut viewport = SubViewport::new_alloc();
    viewport.set_name("PortraitViewport");
    viewport.set_use_own_world_3d(true);
    viewport.set_transparent_background(true);
    viewport.set_update_mode(UpdateMode::WHEN_VISIBLE);
    let mut root = Node3D::new_alloc();
    root.set_name("PortraitSceneRoot");
    let mut camera = Camera3D::new_alloc();
    camera.set_name("PortraitCamera");
    // m2.gdshader lights opaque batches inside a directional light pass.
    let mut light = DirectionalLight3D::new_alloc();
    light.set_name("PortraitLight");
    root.add_child(&camera);
    root.add_child(&light);
    viewport.add_child(&root);
    (viewport, root, camera)
}

/// `MASK_SHADER` with the slot's mask rect in portrait UVs; the mask texture follows.
fn mask_material(slot: &PortraitSlot) -> Gd<ShaderMaterial> {
    let mut shader = Shader::new_gd();
    shader.set_code(MASK_SHADER);
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shader);
    let (x, y, width, height) = slot.rect;
    let (mask_x, mask_y, mask_w, mask_h) = slot.mask_rect;
    let mask_rect = Vector4::new(
        (mask_x - x) / width,
        (mask_y - y) / height,
        mask_w / width,
        mask_h / height,
    );
    material.set_shader_parameter("mask_rect", &mask_rect.to_variant());
    material
}

/// The visual's M2 model root: the visual itself or its model child (creature visuals
/// wrap theirs in a scaled root).
fn model_root(visual: &Gd<Node3D>) -> Option<Gd<Node3D>> {
    std::iter::once(visual.clone())
        .chain(
            visual
                .get_children()
                .iter_shared()
                .filter_map(|child| child.try_cast::<Node3D>().ok()),
        )
        .find(|node| node.has_meta(M2_BOUNDS_META))
}

/// Place `camera` as the model's M2 portrait camera, through the model root's transform.
fn frame_portrait(camera: &mut Gd<Camera3D>, visual: &Gd<Node3D>) -> Result<(), String> {
    let root = model_root(visual).ok_or("visual has no M2 model root")?;
    let path = root.get_meta(M2_SOURCE_META).to::<GString>().to_string();
    let bytes = fs::read(&path).map_err(|error| format!("{path}: {error}"))?;
    let portrait = parse_portrait_camera(&bytes).map_err(|error| format!("{path}: {error}"))?;
    let to_scene = root.get_global_transform();
    let scale = to_scene.basis.get_scale().x;
    let eye = to_scene * wow_vec3(portrait.position);
    let target = to_scene * wow_vec3(portrait.target);
    camera.look_at_from_position(eye, target);
    camera.set_fov(vertical_fov(portrait.fov, 1.0).to_degrees());
    camera.set_near(portrait.near_clip * scale);
    camera.set_far(portrait.far_clip * scale);
    Ok(())
}

fn describe(appearance: &UnitAppearance) -> String {
    match appearance {
        UnitAppearance::Creature { display_id, items } => {
            format!(
                "creature display {display_id} items {}",
                items.entries.len()
            )
        }
        UnitAppearance::Player(player, equipment) => format!(
            "player {} race {} items {}",
            player.name,
            player.race,
            equipment.entries.len()
        ),
    }
}

impl GameClient {
    /// The local player's portrait in PlayerFrame, the target's in TargetFrame.
    pub(super) fn sync_unit_portraits(&mut self) -> Result<(), String> {
        let ui = self.targeting.frame_ui().cloned();
        let host = |slot: &PortraitSlot| {
            ui.as_ref()
                .and_then(|ui| ui.bind().frame_control(slot.frame))
        };
        let player_host = host(&PLAYER_PORTRAIT);
        let target_host = host(&TARGET_PORTRAIT);
        let player = self
            .world
            .local_player_id()
            .and_then(|id| self.world.unit_appearance(id))
            .cloned();
        let target = self
            .targeting_target()
            .and_then(|id| self.world.unit_appearance(id))
            .cloned();
        let portraits = &mut self.targeting.portraits;
        let player_result = portraits.player.sync(&mut self.world, player_host, player);
        let target_result = portraits.target.sync(&mut self.world, target_host, target);
        player_result.and(target_result)
    }

    pub(super) fn clear_unit_portraits(&mut self) {
        let portraits = &mut self.targeting.portraits;
        portraits.player.clear(&mut self.world);
        portraits.target.clear(&mut self.world);
    }
}

#[godot_api(secondary)]
impl GameClient {
    /// Portrait state for automation: `frame` is `PlayerPortrait` or `TargetFramePortrait`.
    #[func]
    fn unit_portrait_state(&self, frame: GString) -> VarDictionary {
        let portraits = &self.targeting.portraits;
        let frame = frame.to_string();
        [&portraits.player, &portraits.target]
            .into_iter()
            .find(|portrait| portrait.slot.frame == frame)
            .map(Portrait::snapshot)
            .unwrap_or_default()
    }
}
