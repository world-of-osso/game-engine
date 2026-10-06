//! Unit frame portraits (`SetPortraitTexture`, Blizzard_UnitFrame/Mainline/UnitFrame.lua:188):
//! the unit's model seen from its M2 portrait camera, rendered into the PlayerFrame,
//! TargetFrame and PetFrame portrait slots, and the interacting NPC's into the
//! MerchantFrame and QuestFrame portrait rings, over black and rounded by their masks. A portrait follows its
//! unit and re-renders when the unit's visual appearance changes (`UNIT_PORTRAIT_UPDATE`,
//! UnitFrame.lua:199). Like Retail's portrait texture it is a still image: the model's first
//! pose rendered once per change, the viewport idle in between.

#[cfg(test)]
mod camera_tests;
mod fixture;
mod hud_fixture;
mod party;

use party::PartyPortraits;
use std::fs;

use shared::components::Player;

use game_engine_core::asset::m2_format::m2_camera::parse_portrait_camera;
use game_engine_core::creation_scene_data::vertical_fov;
use game_engine_ui_model::character_frame::PORTRAIT as CHARACTER_FRAME_PORTRAIT;
use game_engine_ui_model::inworld_unit_frames_component::{
    PET_PORTRAIT, PLAYER_PORTRAIT, PortraitSlot, TARGET_PORTRAIT,
};
use game_engine_ui_model::merchant_frame_component::PORTRAIT as MERCHANT_PORTRAIT;
use game_engine_ui_model::micro_menu::CHARACTER_PORTRAIT;
use game_engine_ui_model::quest_frame_component::PORTRAIT as QUEST_PORTRAIT;
use godot::classes::control::{LayoutPreset, MouseFilter};
use godot::classes::node::ProcessMode;
use godot::classes::sub_viewport::UpdateMode;
use godot::classes::texture_rect::{ExpandMode, StretchMode};
use godot::classes::{
    Camera3D, Control, DirectionalLight3D, Node3D, Shader, ShaderMaterial, SubViewport, TextureRect,
};
use godot::prelude::*;
use shared::components::SheathState;

use crate::GameClient;
use crate::assets::{M2_BOUNDS_META, M2_SOURCE_META, wow_vec3};
use crate::character_frame::preview::bind_preview_light;
use crate::world::WorldUnits;
use crate::world_models::UnitAppearance;

/// The model over black, kept where the mask's alpha is; outside the mask rect is clear
/// (`CLAMPTOBLACKADDITIVE`).
const MASK_SHADER: &str = "shader_type canvas_item;
uniform sampler2D mask_texture : filter_linear;
uniform vec4 mask_rect;
uniform vec4 tex_rect = vec4(0.0, 0.0, 1.0, 1.0);
uniform bool desaturated = false;
uniform vec4 portrait_tint = vec4(1.0);
void fragment() {
    vec4 model = texture(TEXTURE, tex_rect.xy + UV * tex_rect.zw);
    vec2 mask_uv = (UV - mask_rect.xy) / mask_rect.zw;
    bool inside = all(greaterThanEqual(mask_uv, vec2(0.0))) && all(lessThanEqual(mask_uv, vec2(1.0)));
    float keep = inside ? texture(mask_texture, mask_uv).a : 0.0;
    vec3 rgb = model.rgb * model.a;
    if (desaturated) { rgb = vec3(dot(rgb, vec3(0.299, 0.587, 0.114))); }
    COLOR = vec4(rgb * portrait_tint.rgb, keep * portrait_tint.a);
}
";

pub(crate) struct UnitPortraits {
    player: Portrait,
    target: Portrait,
    pet: Portrait,
    /// The CharacterMicroButton's player portrait.
    micro: Portrait,
    character: Portrait,
    launcher: Portrait,
    /// The open vendor in MerchantFrame (`SetPortraitToUnit("npc")`).
    merchant: Portrait,
    /// The quest giver or gossip NPC in QuestFrame (`SetPortraitTexture(.., "questnpc")`).
    quest: Portrait,
    party: PartyPortraits,
}

impl Default for UnitPortraits {
    fn default() -> Self {
        Self {
            player: Portrait::new(PLAYER_PORTRAIT),
            target: Portrait::new(TARGET_PORTRAIT),
            pet: Portrait::new(PET_PORTRAIT),
            micro: Portrait::new(CHARACTER_PORTRAIT),
            character: Portrait::new(CHARACTER_FRAME_PORTRAIT),
            launcher: Portrait::new(game_engine_ui_model::launcher::CHARACTER_ICON),
            merchant: Portrait::new(MERCHANT_PORTRAIT),
            quest: Portrait::new(QUEST_PORTRAIT),
            party: PartyPortraits::default(),
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

    fn set_slot(&mut self, slot: PortraitSlot) {
        if self.slot.mask_fdid != slot.mask_fdid
            && let Some(scene) = self.scene.as_mut()
            && scene.view.is_instance_valid()
        {
            scene.mask_loaded = false;
            scene.view.set_visible(false);
        }
        self.slot = slot;
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
        set_slot_rects(&mut scene.material, &slot);
        scene.load_mask(&slot)?;
        scene.fit_viewport(slot.tex_coords);
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
        if let Some(scene) = self.scene.as_mut() {
            scene.move_to_host(&host);
        }
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
        let mut model = loaded?;
        // A player's weapons sheathed, as on the character sheet.
        if let Err(error) = world.place_player_weapons(&model, &appearance, SheathState::Unarmed) {
            model.free();
            return Err(error);
        }
        // A still image: the model keeps the pose it was built in.
        model.set_process_mode(ProcessMode::DISABLED);
        scene.root.add_child(&model);
        scene.model = Some(model.clone());
        if let Err(error) = frame_portrait(&mut scene.camera, &model) {
            // No camera to frame it: show no model rather than a stale view.
            scene.remove_model();
            return Err(format!("{} portrait: {error}", self.slot.frame));
        }
        // The M2 shader uses -sun_direction. Keep the key in front of every
        // model's authored portrait camera, including scaled/rotated creatures.
        let rays = -scene.camera.get_global_basis().col_c();
        bind_preview_light(&model, rays);
        scene.render_once();
        Ok(())
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

    /// A surviving member moves slots without allocating another viewport.
    fn move_to_host(&mut self, host: &Gd<Control>) {
        if !self.view.is_instance_valid() || self.host == host.instance_id() {
            return;
        }
        self.view
            .reparent_ex(host)
            .keep_global_transform(false)
            .done();
        self.view
            .set_anchors_and_offsets_preset(LayoutPreset::FULL_RECT);
        self.host = host.instance_id();
    }

    fn set_style(&mut self, desaturated: bool, tint: [f32; 4]) {
        self.material
            .set_shader_parameter("desaturated", &desaturated.to_variant());
        let tint = Vector4::new(tint[0], tint[1], tint[2], tint[3]);
        self.material
            .set_shader_parameter("portrait_tint", &tint.to_variant());
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
        state.set(
            "desaturated",
            &self.material.get_shader_parameter("desaturated"),
        );
        let tint = self
            .material
            .get_shader_parameter("portrait_tint")
            .to::<Vector4>();
        state.set("tint", Color::from_rgba(tint.x, tint.y, tint.z, tint.w));
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
    /// The whole portrait image at the pixel density the slot shows its crop at.
    fn fit_viewport(&mut self, [left, right, top, bottom]: [f32; 4]) {
        let size = self.view.get_global_rect().size;
        let size = Vector2i::new(
            (size.x / (right - left)).round().max(1.0) as i32,
            (size.y / (bottom - top)).round().max(1.0) as i32,
        );
        if self.viewport.get_size() != size {
            self.viewport.set_size(size);
            self.render_once();
        }
    }

    fn remove_model(&mut self) {
        if let Some(model) = self.model.take() {
            model.free();
            self.render_once();
        }
    }

    /// Redraw the portrait on the next frame only; it then stays as drawn.
    fn render_once(&mut self) {
        self.viewport.set_update_mode(UpdateMode::ONCE);
    }
}

/// A transparent own-world viewport with the portrait camera and its light.
fn portrait_viewport() -> (Gd<SubViewport>, Gd<Node3D>, Gd<Camera3D>) {
    let mut viewport = SubViewport::new_alloc();
    viewport.set_name("PortraitViewport");
    viewport.set_use_own_world_3d(true);
    viewport.set_transparent_background(true);
    viewport.set_update_mode(UpdateMode::ONCE);
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

/// `MASK_SHADER` with the slot's rects; the mask texture follows.
fn mask_material(slot: &PortraitSlot) -> Gd<ShaderMaterial> {
    let mut shader = Shader::new_gd();
    shader.set_code(MASK_SHADER);
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shader);
    set_slot_rects(&mut material, slot);
    material.set_shader_parameter("desaturated", &false.to_variant());
    material.set_shader_parameter("portrait_tint", &Vector4::ONE.to_variant());
    material
}

/// The slot's mask rect in view UVs and its `SetTexCoord` crop of the portrait image.
fn set_slot_rects(material: &mut Gd<ShaderMaterial>, slot: &PortraitSlot) {
    let [left, right, top, bottom] = slot.tex_coords;
    let tex_rect = Vector4::new(left, top, right - left, bottom - top);
    material.set_shader_parameter("tex_rect", &tex_rect.to_variant());
    let (x, y, width, height) = slot.rect;
    let (mask_x, mask_y, mask_w, mask_h) = slot.mask_rect;
    let mask_rect = Vector4::new(
        (mask_x - x) / width,
        (mask_y - y) / height,
        mask_w / width,
        mask_h / height,
    );
    material.set_shader_parameter("mask_rect", &mask_rect.to_variant());
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
    /// The local player's portrait in PlayerFrame, the target's in TargetFrame, the pet's
    /// in PetFrame.
    pub(super) fn sync_unit_portraits(&mut self) -> Result<(), String> {
        let ui = self.targeting.frame_ui().cloned();
        let host = |slot: &PortraitSlot| {
            ui.as_ref()
                .and_then(|ui| ui.bind().frame_control(slot.frame))
        };
        let player_host = host(&PLAYER_PORTRAIT);
        let target_host = host(&TARGET_PORTRAIT);
        let pet_host = host(&PET_PORTRAIT);
        let player = self
            .world
            .local_player_id()
            .and_then(|id| self.world.unit_appearance(id))
            .cloned();
        let target = self
            .targeting_target()
            .and_then(|id| self.world.unit_appearance(id))
            .cloned();
        let pet = self
            .local_pet_id()
            .and_then(|id| self.world.unit_appearance(id))
            .cloned();
        let micro_slot = self.micro_menu_view().character_portrait();
        let micro_host = self
            .character_frame
            .micro_ui()
            .and_then(|ui| ui.bind().frame_control(micro_slot.frame));
        let character_host = self
            .character_frame
            .frame_ui()
            .and_then(|ui| ui.bind().frame_control(CHARACTER_FRAME_PORTRAIT.frame));
        let launcher_host = self.launcher.ui.as_ref().and_then(|ui| {
            ui.bind()
                .frame_control(game_engine_ui_model::launcher::CHARACTER_ICON.frame)
        });
        let (merchant_host, merchant) = self.npc_portrait(
            self.merchant.ui.as_ref(),
            &MERCHANT_PORTRAIT,
            self.merchant.session.portrait_unit(),
        );
        let quest_npc = self.account.quests.dialog.as_ref().map(|dialog| dialog.npc);
        let (quest_host, quest) =
            self.npc_portrait(self.quests.frame_ui(), &QUEST_PORTRAIT, quest_npc);
        let portraits = &mut self.targeting.portraits;
        portraits.micro.slot = micro_slot;
        let player_result = portraits
            .player
            .sync(&mut self.world, player_host, player.clone());
        let target_result = portraits.target.sync(&mut self.world, target_host, target);
        let pet_result = portraits.pet.sync(&mut self.world, pet_host, pet);
        let micro_result = portraits
            .micro
            .sync(&mut self.world, micro_host, player.clone());
        let character_result =
            portraits
                .character
                .sync(&mut self.world, character_host, player.clone());
        let launcher_result = portraits
            .launcher
            .sync(&mut self.world, launcher_host, player);
        let merchant_result = portraits
            .merchant
            .sync(&mut self.world, merchant_host, merchant);
        let quest_result = portraits.quest.sync(&mut self.world, quest_host, quest);
        player_result
            .and(target_result)
            .and(pet_result)
            .and(micro_result)
            .and(character_result)
            .and(launcher_result)
            .and(merchant_result)
            .and(quest_result)
    }

    /// The window's portrait slot and `npc`'s appearance while the window shows that NPC.
    fn npc_portrait(
        &self,
        ui: Option<&Gd<crate::ui::RegistryUi>>,
        slot: &PortraitSlot,
        npc: Option<u64>,
    ) -> (Option<Gd<Control>>, Option<UnitAppearance>) {
        let Some(npc) = npc else {
            return (None, None);
        };
        let host = ui.and_then(|ui| ui.bind().frame_control(slot.frame));
        (host, self.world.unit_appearance(npc).cloned())
    }

    /// Called after the group canvas sync, so every host matches the current roster.
    pub(super) fn sync_party_portraits(&mut self) -> Result<(), String> {
        let compact = game_engine_ui_model::hud_layout::active_layout_settings()
            .use_raid_style_party_frames
            .unwrap_or(true);
        let local = self.account.session.selected_character_name.as_deref();
        let sort = game_engine_ui_model::hud_layout::active_layout_settings()
            .party
            .sort
            .unwrap_or_default();
        let bindings = party::bindings_with_sort(&self.account.group, local, compact, sort);
        if bindings.is_empty() {
            self.clear_party_portraits();
            return Ok(());
        }
        let appearances: std::collections::HashMap<_, _> = self
            .replica
            .units()
            .filter_map(|unit| {
                let player = unit.get::<Player>()?;
                let appearance = self.world.unit_appearance(unit.server_id)?.clone();
                Some((player.name.clone(), appearance))
            })
            .collect();
        self.targeting.portraits.party.sync(
            &mut self.world,
            self.group_frames.frame_ui(),
            bindings,
            |name| appearances.get(name).cloned(),
        )
    }

    pub(super) fn clear_party_portraits(&mut self) {
        self.targeting.portraits.party.clear(&mut self.world);
    }

    pub(super) fn clear_unit_portraits(&mut self) {
        let portraits = &mut self.targeting.portraits;
        portraits.player.clear(&mut self.world);
        portraits.target.clear(&mut self.world);
        portraits.pet.clear(&mut self.world);
        portraits.micro.clear(&mut self.world);
        portraits.character.clear(&mut self.world);
        portraits.launcher.clear(&mut self.world);
        portraits.merchant.clear(&mut self.world);
        portraits.quest.clear(&mut self.world);
        portraits.party.clear(&mut self.world);
    }
}

#[godot_api(secondary)]
impl GameClient {
    /// Portrait state for automation: `frame` is `PlayerPortrait`, `TargetFramePortrait`,
    /// `PetPortrait`, `CharacterMicroButtonPortrait`, `MerchantFramePortrait` or
    /// `QuestFramePortrait` or `CharacterFramePortrait`.
    #[func]
    fn unit_portrait_state(&self, frame: GString) -> VarDictionary {
        let portraits = &self.targeting.portraits;
        let frame = frame.to_string();
        [
            &portraits.player,
            &portraits.target,
            &portraits.pet,
            &portraits.micro,
            &portraits.character,
            &portraits.launcher,
            &portraits.merchant,
            &portraits.quest,
        ]
        .into_iter()
        .find(|portrait| portrait.slot.frame == frame)
        .map(Portrait::snapshot)
        .or_else(|| {
            portraits
                .party
                .members
                .values()
                .find(|member| member.portrait.slot.frame == frame)
                .map(|member| member.portrait.snapshot())
        })
        .unwrap_or_default()
    }
}
