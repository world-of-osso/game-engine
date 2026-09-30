//! Unit selection, ported from the Bevy client's `rendering/ui/target.rs`: left-click
//! on a unit selects it through a camera ray against the unit's drawn triangles that
//! world geometry occludes (`unit_pick`), Tab cycles selectable NPCs nearest first, F1 targets the local
//! player, Escape clears the target before the game menu opens. The server learns
//! each change through `SetTarget`; the TargetFrame and the selection ring show it.

use std::path::PathBuf;
use std::time::Instant;

use game_engine_core::{
    creature_health_scaling_data::CreatureHealthByLevel,
    input_bindings_data::{BindingMouseButton, InputAction, InputState},
    target_selection_data::{next_target, opaque_to_alpha_mask},
};
use game_engine_network::replica::Unit;
use game_engine_session::SessionScreen;
use game_engine_ui_model::inworld_unit_frames_component::class_bars::ClassBarAnimator;
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, PowerBarState, UnitFrameMenuState, UnitFrameState, format_value_text,
    fraction, target_level_text,
};
use game_engine_ui_model::status::{ClassBarPlayer, ClassBarResource};
use godot::{
    classes::{
        Area3D, BoxShape3D, Camera3D, CollisionShape3D, Decal, Image, ImageTexture, MeshInstance3D,
        Node3D, decal::DecalTexture, image,
    },
    prelude::*,
};
use shared::components::{Health, Npc, Player, UnitLevel, UnitPowers, UnitRunes};
use shared::level_scaling::{LevelScaling, level_for_viewer};

use crate::replicated::{UnitFields, is_unit};

use crate::frame_error::{FrameError, SessionError, report_once};
pub(crate) use crate::unit_pick::pick_unit;
use crate::unit_pick::{UNIT_ID_META, UNIT_VISUAL_META};
use crate::{GameClient, assets::M2_BOUNDS_META, ui::RegistryUi};

/// Physics layer 3 holds only unit pick shapes; terrain and WMO bodies use layers 1-2.
pub(crate) const UNIT_PICK_LAYER: u32 = 1 << 2;
const PICK_AREA: &str = "UnitPick";
const TARGET_CIRCLE: &str = "TargetCircle";

/// Default `TargetCircleStyle`: "Fat Ring" FDID 167207 tinted (255, 220, 50).
const RING_FDID: u32 = 167_207;
const RING_RGB: [u8; 3] = [255, 220, 50];
/// Bevy `TARGET_CIRCLE_SIZE_FACTOR`, its 0.35 floor and the decal's 2x marker scale.
const CIRCLE_SIZE_FACTOR: f32 = 0.7;
const MIN_CIRCLE_SIZE: f32 = 0.35;
const DECAL_SCALE: f32 = 2.0;
/// The ring sits 0.08 above the unit origin and projects onto ground within this height.
const CIRCLE_LIFT: f32 = 0.08;
const CIRCLE_DEPTH: f32 = 1.0;
const EXPECTED_STAT_CSV: &str = "db2/12.1.0.69933/ExpectedStat.csv";

pub(crate) struct Targeting {
    target: Option<u64>,
    /// The target last sent in `SetTarget`.
    sent: Option<u64>,
    /// The ring on the target, sized for the pick box (visual) it was spawned for.
    circle: Option<TargetCircle>,
    /// The loaded ring art; `Some(None)` caches art that failed to load.
    ring: Option<Option<Gd<ImageTexture>>>,
    /// `RING_FDID` unless a test points the ring at other art.
    ring_fdid: u32,
    frame_ui: Option<Gd<RegistryUi>>,
    /// The player's class resource bar, timed from `started`.
    class_bar: ClassBarAnimator,
    started: Instant,
    data_root: PathBuf,
    /// ExpectedStat creature health, loaded with the first tuned target.
    health_by_level: Option<Result<CreatureHealthByLevel, String>>,
    /// The TargetFrame's level and health texts last shown, for automation.
    frame_texts: (String, String),
}

struct TargetCircle {
    unit: u64,
    /// `unit_pick_shape` of the visual the ring was sized for: a swapped model
    /// (a Polymorph display change) resizes it.
    shape: Option<InstanceId>,
    decal: Gd<Decal>,
}

impl Targeting {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.frame_ui {
            visit(ui)?;
        }
        Ok(())
    }

    pub fn new(data_root: PathBuf) -> Self {
        Self {
            target: None,
            sent: None,
            circle: None,
            ring: None,
            ring_fdid: RING_FDID,
            frame_ui: None,
            class_bar: ClassBarAnimator::default(),
            started: Instant::now(),
            data_root,
            health_by_level: None,
            frame_texts: Default::default(),
        }
    }

    fn health_by_level(&mut self) -> Result<&CreatureHealthByLevel, String> {
        self.health_by_level
            .get_or_insert_with(|| {
                let path = self.data_root.join(EXPECTED_STAT_CSV);
                std::fs::read_to_string(&path)
                    .map_err(|error| format!("Read {}: {error}", path.display()))
                    .and_then(|text| CreatureHealthByLevel::parse_expected_stat_csv(&text))
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    /// The unit frames UI, once shown.
    pub(crate) fn frame_ui(&self) -> Option<&Gd<RegistryUi>> {
        self.frame_ui.as_ref()
    }

    fn free_circle(&mut self) {
        if let Some(circle) = self.circle.take()
            && circle.decal.is_instance_valid()
        {
            circle.decal.free();
        }
    }

    fn free_frame_ui(&mut self) {
        if let Some(ui) = self.frame_ui.take() {
            ui.free();
        }
    }

    /// Test hook: read the ring from `fdid`; the next frame respawns it.
    pub(crate) fn override_ring_fdid(&mut self, fdid: u32) {
        self.ring_fdid = fdid;
        self.ring = None;
        self.free_circle();
    }

    /// The ring art; a load failure is reported once and the ring stays absent.
    fn ring_texture(&mut self) -> Option<Gd<ImageTexture>> {
        if let Some(ring) = &self.ring {
            return ring.clone();
        }
        let ring = self
            .load_ring()
            .inspect_err(|error| report_once(error))
            .ok();
        self.ring = Some(ring.clone());
        ring
    }

    fn load_ring(&self) -> Result<Gd<ImageTexture>, String> {
        let path = self
            .data_root
            .join(format!("textures/{}.blp", self.ring_fdid));
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("Read target ring {}: {error}", path.display()))?;
        let mut decoded = game_engine_core::blp::decode_rgba(&bytes)
            .map_err(|error| format!("Decode target ring {}: {error}", path.display()))?;
        opaque_to_alpha_mask(&mut decoded.pixels);
        let pixels = PackedByteArray::from(decoded.pixels.as_slice());
        let image = Image::create_from_data(
            decoded.width as i32,
            decoded.height as i32,
            false,
            image::Format::RGBA8,
            &pixels,
        )
        .ok_or_else(|| format!("Godot rejected target ring image {}", path.display()))?;
        ImageTexture::create_from_image(&image)
            .ok_or_else(|| format!("Godot rejected target ring texture {}", path.display()))
    }
}

/// Give a unit visual its broad-phase pick shape: a box of the M2 header bounding box
/// on `UNIT_PICK_LAYER`, identified by the unit's server id, naming the visual whose
/// triangles decide the hit (`unit_pick`). Areas are ignored by the default body-only
/// rays (camera, ground), so units never block them.
pub(crate) fn attach_pick_area(visual: &Gd<Node3D>, server_id: u64) -> Result<(), String> {
    let mut model = std::iter::once(visual.clone())
        .chain(
            visual
                .get_children()
                .iter_shared()
                .filter_map(|child| child.try_cast::<Node3D>().ok()),
        )
        .find(|node| node.has_meta(M2_BOUNDS_META))
        .ok_or_else(|| format!("Unit {server_id} visual has no M2 bounding box"))?;
    let bounds = model.get_meta(M2_BOUNDS_META).to::<Aabb>();
    let mut area = Area3D::new_alloc();
    area.set_name(PICK_AREA);
    area.set_collision_layer(UNIT_PICK_LAYER);
    area.set_collision_mask(0);
    area.set_monitoring(false);
    area.set_meta(UNIT_ID_META, &(server_id as i64).to_variant());
    area.set_meta(
        UNIT_VISUAL_META,
        &visual.instance_id().to_i64().to_variant(),
    );
    let mut shape = BoxShape3D::new_gd();
    shape.set_size(bounds.size.abs());
    let mut node = CollisionShape3D::new_alloc();
    node.set_shape(&shape);
    node.set_position(bounds.center());
    area.add_child(&node);
    model.add_child(&area);
    Ok(())
}

/// The pick box of a unit node's visual, once its model has loaded.
pub(crate) fn unit_pick_shape(unit: &Gd<Node3D>) -> Option<Gd<CollisionShape3D>> {
    unit.find_child_ex(PICK_AREA)
        .owned(false)
        .done()?
        .get_child(0)?
        .try_cast::<CollisionShape3D>()
        .ok()
}

/// Bevy `target_circle_size`: the widest horizontal extent of the unit's meshes from
/// its origin, scaled; the display scale when the unit has no mesh yet.
fn circle_size(unit: &Gd<Node3D>) -> f32 {
    let origin = unit.get_global_position();
    let meshes = unit
        .find_children_ex("*")
        .type_("MeshInstance3D")
        .owned(false)
        .done();
    let mut extent = 0.0_f32;
    for node in meshes.iter_shared() {
        let mesh = node.cast::<MeshInstance3D>();
        let bounds = mesh.get_aabb();
        let transform = mesh.get_global_transform();
        for corner in 0..8 {
            let pick = |bit: i32, size: f32| if corner & bit != 0 { size } else { 0.0 };
            let offset = Vector3::new(
                pick(1, bounds.size.x),
                pick(2, bounds.size.y),
                pick(4, bounds.size.z),
            );
            let point = transform * (bounds.position + offset);
            extent = extent
                .max((point.x - origin.x).abs())
                .max((point.z - origin.z).abs());
        }
    }
    let size = if extent > 0.0 {
        extent
    } else {
        unit.get_scale().x
    };
    (size * CIRCLE_SIZE_FACTOR).max(MIN_CIRCLE_SIZE)
}

fn spawn_circle(unit: &mut Gd<Node3D>, ring: &Gd<ImageTexture>) -> Gd<Decal> {
    let mut decal = Decal::new_alloc();
    decal.set_name(TARGET_CIRCLE);
    let diameter = circle_size(unit) * DECAL_SCALE;
    decal.set_size(Vector3::new(diameter, CIRCLE_DEPTH, diameter));
    decal.set_position(Vector3::new(0.0, CIRCLE_LIFT, 0.0));
    decal.set_texture(DecalTexture::ALBEDO, ring);
    decal.set_texture(DecalTexture::EMISSION, ring);
    // Bevy draws the ring unlit with 1.5x its tint as emission.
    let [r, g, b] = RING_RGB.map(|channel| channel as f32 / 255.0);
    decal.set_modulate(Color::from_rgb(r, g, b));
    decal.set_emission_energy(1.5);
    unit.add_child(&decal);
    decal
}

fn optional_id(id: Option<u64>) -> Variant {
    id.map(|id| (id as i64).to_variant()).unwrap_or_default()
}

fn unit_name(unit: Unit) -> String {
    unit.name().unwrap_or("Unknown").to_owned()
}

/// Bevy `build_target_state` from the replicated values the native snapshot carries. A
/// tuned creature shows its level against the viewer (`UnitEffectiveLevel`,
/// TargetFrameMixin:CheckLevel, TargetFrame.lua:266-282) and its health pool times
/// `health_multiplier` (`GetHealthMultiplierForTarget`).
fn target_frame_state(
    unit: Unit,
    viewer_level: Option<u8>,
    health_multiplier: f32,
) -> UnitFrameState {
    let mut state = UnitFrameState::named(unit_name(unit));
    let level = unit.get::<UnitLevel>().map(|&level| {
        level_for_viewer(
            level,
            unit.get::<LevelScaling>(),
            viewer_level.unwrap_or(level.0),
        )
    });
    state.level_text = target_level_text(level, viewer_level);
    if let Some(health) = unit.get::<Health>() {
        let (current, max) = (
            (health.current * health_multiplier).round(),
            (health.max * health_multiplier).round(),
        );
        state.health_text = format_value_text(current, max);
        state.health_fraction = fraction(health.current, health.max);
    }
    state
}

/// The health multiplier a tuned `unit` has for a viewer of `viewer_level`; 1 untuned.
fn target_health_multiplier(
    table: &CreatureHealthByLevel,
    unit: Unit,
    viewer_level: Option<u8>,
) -> f32 {
    match (unit.get::<LevelScaling>(), viewer_level) {
        (Some(scaling), Some(viewer)) => {
            table.health_multiplier(scaling.native_level(), scaling.level_for_target(viewer))
        }
        _ => 1.0,
    }
}

fn player_frame_state(unit: Unit, in_rest_area: bool) -> UnitFrameState {
    let mut state = UnitFrameState::named(
        unit.get::<Player>()
            .map_or("", |player| player.name.as_str()),
    );
    state.level_text = unit
        .get::<UnitLevel>()
        .map(|level| level.0.to_string())
        .unwrap_or_default();
    state.show_combat_icon = unit.in_combat();
    state.show_resting_icon = in_rest_area;
    if let Some(health) = unit.get::<Health>() {
        state.health_text = format_value_text(health.current, health.max);
        state.health_fraction = fraction(health.current, health.max);
    }
    state.power = unit.get::<UnitPowers>().and_then(PowerBarState::primary);
    state
}

/// The player's class bar power, when Retail shows the class's bar.
fn player_class_resource(unit: Unit, spec: Option<u32>) -> Option<ClassBarResource> {
    let player = ClassBarPlayer {
        class: unit.get::<Player>()?.class,
        spec,
        level: unit.get::<UnitLevel>().map_or(0, |level| level.0),
        in_combat: unit.in_combat(),
    };
    ClassBarResource::for_player(unit.get::<UnitPowers>()?, unit.get::<UnitRunes>(), &player)
}

fn unit_frames_state(
    player: Option<UnitFrameState>,
    target: Option<UnitFrameState>,
    show_health_bars: bool,
) -> InWorldUnitFramesState {
    InWorldUnitFramesState {
        show_player_frame: show_health_bars && player.is_some(),
        show_target_frame: show_health_bars,
        player: player.unwrap_or_else(|| UnitFrameState::named("")),
        target,
        target_of_target: None,
        focus: None,
        bosses: Vec::new(),
        menu: UnitFrameMenuState::default(),
    }
}

impl GameClient {
    pub(super) fn targeting_target(&self) -> Option<u64> {
        self.targeting.target
    }

    /// Select `target`; `SetTarget` follows on the next update.
    pub(super) fn set_target(&mut self, target: Option<u64>) {
        self.targeting.target = target;
    }

    /// Per frame, before input edges clear: selection input, then its presentation.
    pub(super) fn update_targeting(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.targeting.target = None;
            self.targeting.sent = None;
            self.targeting.free_circle();
            self.targeting.free_frame_ui();
            return Ok(());
        }
        if self
            .targeting
            .target
            .is_some_and(|id| !self.replica.unit(id).is_some_and(is_unit))
        {
            self.targeting.target = None;
        }
        if self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            self.apply_targeting_input();
        }
        self.follow_selection_with_auto_attack()?;
        self.send_target()?;
        self.sync_target_circle()?;
        if let Some(circle) = self.targeting.circle.as_mut() {
            circle
                .decal
                .set_visible(self.client_options.hud.show_target_marker);
        }
        Ok(self.sync_unit_frames()?)
    }

    fn apply_targeting_input(&mut self) {
        let bindings = &self.client_options.bindings;
        let input = self.physical_input.gameplay_state(true);
        if bindings.is_just_pressed(InputAction::TargetNearest, &input) {
            let sorted = self.selectable_npcs_nearest_first();
            self.targeting.target = next_target(&sorted, self.targeting.target);
        } else if bindings.is_just_pressed(InputAction::TargetSelf, &input) {
            self.targeting.target = self.world.local_player_id();
        } else if self
            .physical_input
            .mouse_just_pressed(BindingMouseButton::Left)
            && let Some(unit) = self.unit_under_pointer()
        {
            // A world click keeps the target: Retail "Sticky Targeting", the negated
            // `deselectOnClick` CVar (Blizzard_SettingsDefinitions_Frame/Controls.lua:43-47).
            self.targeting.target = Some(unit);
        }
    }

    fn unit_under_pointer(&self) -> Option<u64> {
        let viewport = self.base().get_viewport()?;
        if viewport.gui_get_hovered_control().is_some() {
            return None;
        }
        let camera = viewport.get_camera_3d()?;
        pick_unit(&camera, Vector2::from_array(self.physical_input.pointer()))
    }

    /// Bevy `sorted_targets_by_distance`: visible NPCs by distance from the player.
    fn selectable_npcs_nearest_first(&self) -> Vec<u64> {
        let Some(player) = self.world.local_player_transform() else {
            return Vec::new();
        };
        let mut npcs: Vec<(u64, f32)> = self
            .replica
            .units()
            .filter(|unit| unit.has::<Npc>())
            .filter_map(|unit| {
                let node = self.world.unit_node(unit.server_id)?;
                let distance = node
                    .get_global_position()
                    .distance_squared_to(player.origin);
                node.is_visible_in_tree()
                    .then_some((unit.server_id, distance))
            })
            .collect();
        npcs.sort_by(|a, b| a.1.total_cmp(&b.1));
        npcs.into_iter().map(|(id, _)| id).collect()
    }

    fn send_target(&mut self) -> Result<(), SessionError> {
        let target = self.targeting.target;
        if target == self.targeting.sent || !self.account.session.gameplay_input_allowed() {
            return Ok(());
        }
        self.account.send_set_target(target)?;
        self.targeting.sent = target;
        Ok(())
    }

    fn sync_target_circle(&mut self) -> Result<(), String> {
        let target = self.targeting.target;
        let unit = target.and_then(|id| Some((id, self.world.unit_node(id)?)));
        let shape = unit
            .as_ref()
            .and_then(|(_, node)| unit_pick_shape(node))
            .map(|shape| shape.instance_id());
        let current = self
            .targeting
            .circle
            .as_ref()
            .map(|circle| (circle.unit, circle.shape));
        if current == target.map(|id| (id, shape))
            && self
                .targeting
                .circle
                .as_ref()
                .is_none_or(|circle| circle.decal.is_instance_valid())
        {
            return Ok(());
        }
        self.targeting.free_circle();
        let Some((id, mut unit)) = unit else {
            return Ok(());
        };
        let Some(ring) = self.targeting.ring_texture() else {
            return Ok(());
        };
        self.targeting.circle = Some(TargetCircle {
            unit: id,
            shape,
            decal: spawn_circle(&mut unit, &ring),
        });
        Ok(())
    }

    fn sync_unit_frames(&mut self) -> Result<(), String> {
        let viewer_level = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.get::<UnitLevel>())
            .map(|level| level.0);
        let target_id = self.targeting.target;
        let target_unit = target_id
            .and_then(|id| self.replica.unit(id))
            .filter(|unit| is_unit(*unit));
        let multiplier = match target_unit.filter(|unit| unit.has::<LevelScaling>()) {
            Some(unit) => {
                target_health_multiplier(self.targeting.health_by_level()?, unit, viewer_level)
            }
            None => 1.0,
        };
        let mut target = target_unit.map(|unit| target_frame_state(unit, viewer_level, multiplier));
        if let (Some(state), Some(id)) = (target.as_mut(), target_id) {
            self.fill_target_auras(state, id);
        }
        let target_state = target.clone();
        self.targeting.frame_texts = target
            .as_ref()
            .map(|state| (state.level_text.clone(), state.health_text.clone()))
            .unwrap_or_default();
        let player = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
            .map(|unit| {
                let spec = self.account.spells.spec();
                let mut state = player_frame_state(unit, self.in_rest_area);
                let resource = player_class_resource(unit, spec);
                let now = self.targeting.started.elapsed().as_secs_f64();
                state.class_bar = self.targeting.class_bar.update_received(
                    unit.server_id,
                    resource.as_ref(),
                    now,
                );
                state
            });
        let class_bar = player.as_ref().and_then(|player| player.class_bar.clone());
        let state = unit_frames_state(player, target, self.client_options.hud.show_health_bars);
        if let Some(ui) = self.targeting.frame_ui.as_mut() {
            ui.bind_mut().set_state(state)?;
        } else {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("UnitFramesUI");
            self.base_mut().add_child(&ui);
            let shown = ui.bind_mut().show_unit_frames(state);
            if let Err(error) = shown {
                ui.free();
                return Err(error);
            }
            self.targeting.frame_ui = Some(ui);
        }
        self.sync_target_aura_swipes(target_state.as_ref())?;
        self.sync_class_bar_swipes(class_bar.as_ref())
    }

    /// Bevy `handle_inworld_escape`: with no window open, Escape clears the target
    /// before it opens the game menu.
    pub(super) fn clear_target_on_escape(&mut self) -> bool {
        if self.game_menu_ui.is_some()
            || self.account.session.screen != SessionScreen::InWorld
            || !self.account.session.gameplay_input_allowed()
        {
            return false;
        }
        self.targeting.target.take().is_some()
    }

    /// Selection state for automation: the target and the TargetFrame's name text.
    pub(super) fn targeting_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let target = self.targeting.target;
        state.set("target", &optional_id(target));
        state.set(
            "target_name",
            target
                .and_then(|id| self.replica.unit(id))
                .map(unit_name)
                .unwrap_or_default()
                .as_str(),
        );
        state.set("level_text", self.targeting.frame_texts.0.as_str());
        state.set("health_text", self.targeting.frame_texts.1.as_str());
        state.set("sent", &optional_id(self.targeting.sent));
        let (pick_us, pick_candidates) = crate::unit_pick::last_pick_stats();
        state.set("last_pick_us", pick_us as i64);
        state.set("last_pick_candidates", i64::from(pick_candidates));
        state.set("auto_attack", &optional_id(self.auto_attack_victim()));
        state.set(
            "circle_on",
            &optional_id(self.targeting.circle.as_ref().map(|circle| circle.unit)),
        );
        state.set(
            "circle_diameter",
            self.targeting
                .circle
                .as_ref()
                .map_or(0.0, |circle| circle.decal.get_size().x),
        );
        // The server's echo: the local player's replicated `UnitTarget`.
        state.set(
            "server_target",
            &optional_id(
                self.world
                    .local_player_id()
                    .and_then(|id| self.replica.unit(id)?.unit_target()),
            ),
        );
        state
    }
}

/// Script access to the native pick path, for physics-level fixtures.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
pub struct UnitPicker {
    base: Base<RefCounted>,
}

#[godot_api]
impl UnitPicker {
    /// Attach the pick shape a unit visual gets in world; returns the error or "".
    #[func]
    fn attach(visual: Gd<Node3D>, server_id: i64) -> GString {
        match attach_pick_area(&visual, server_id as u64) {
            Ok(()) => GString::new(),
            Err(error) => GString::from(error.as_str()),
        }
    }

    /// The unit id the click ray through `screen_point` selects, or nil.
    #[func]
    fn pick(camera: Gd<Camera3D>, screen_point: Vector2) -> Variant {
        optional_id(pick_unit(&camera, screen_point))
    }
}
