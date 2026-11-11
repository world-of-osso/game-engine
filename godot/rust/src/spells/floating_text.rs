//! Floating damage labels and their animation.

use crate::rebuild_guard::{Rebuild, ThemeOverrides};
use crate::{GameClient, combat_text, replicated::UnitFields};
use godot::classes::{
    Camera3D, CanvasLayer, FontFile, Label, Label3D, base_material_3d::BillboardMode,
};
use godot::prelude::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};
use ui_toolkit::widgets::font_string::GameFont;

use combat_text::{
    Attacker, Style,
    incoming::{Message, Motion},
};

/// Height above the unit origin where combat text starts.
const FLOAT_TEXT_HEIGHT: f32 = 3.6;

fn sample_incoming_stagger(message: &Message) -> f32 {
    if message.stagger {
        (godot::global::randi() as i32).rem_euclid(combat_text::incoming::STAGGER_STEP_COUNT) as f32
            + combat_text::incoming::STAGGER_START as f32
    } else {
        0.0
    }
}

pub(super) struct SelfFloatingText {
    label: Gd<Label>,
    shadow: Gd<Label>,
    motion: Motion,
    base_height: f32,
    age: f32,
}

impl SelfFloatingText {
    fn advance(&mut self, delta: f32, viewport: Vector2) -> bool {
        self.age += delta;
        let labels_are_valid = self.label.is_instance_valid() && self.shadow.is_instance_valid();
        if !labels_are_valid || self.age >= combat_text::incoming::SCROLL_SECONDS {
            self.free_labels();
            return false;
        }
        self.update_display(viewport);
        true
    }

    fn free_labels(&self) {
        if self.label.is_instance_valid() {
            self.label.clone().queue_free();
        }
        if self.shadow.is_instance_valid() {
            self.shadow.clone().queue_free();
        }
    }

    fn update_display(&mut self, viewport: Vector2) {
        self.motion.viewport = [viewport.x, viewport.y];
        let frame = self.motion.frame(self.age);
        let size = self.label.get_size();
        let scale = frame.height / self.base_height;
        let left = frame.position[0] - size.x * scale * 0.5;
        let origin = Vector2::new(left, frame.position[1]);
        self.label.set_position(origin);
        self.label.set_scale(Vector2::new(scale, scale));
        self.shadow.set_position(origin + Vector2::ONE);
        self.shadow.set_scale(Vector2::new(scale, scale));
        set_label_alpha(&mut self.label, frame.alpha);
        set_label_alpha(&mut self.shadow, frame.alpha);
    }
}

fn set_label_alpha(label: &mut Gd<Label>, alpha: f32) {
    let mut color = label.get_modulate();
    color.a = alpha;
    label.set_modulate(color);
}

struct WorldTextMetrics {
    em_base: f32,
    pixel_size: f32,
    shadow_offset: Vector3,
}

pub(super) struct FloatingText {
    pub(super) node: Gd<Label3D>,
    /// The black shadow one screen step down-right of the text. Retail draws world text with
    /// a black shadow (`GxuFontAddShadow`, WorldText.cpp:75); a shadow copy is used instead of
    /// Label3D outlines, which do not render for this font.
    shadow: Gd<Label3D>,
    /// The unit this text floats over, for the per-unit slot limit.
    unit: u64,
    age: f32,
    origin: Vector3,
    style: Style,
    /// A spell crit may replace this unit's previous spell crit.
    spell_crit: bool,
    /// Down-right screen direction the shadow is offset along.
    shadow_offset: Vector3,
    /// World height of a style-scale-1 em, from the screen fraction and the camera.
    em_base: f32,
}

impl FloatingText {
    fn advance(&mut self, delta: f32) -> bool {
        self.age += delta;
        if !self.node.is_instance_valid() {
            return false;
        }
        if self.age >= self.style.duration {
            self.node.clone().queue_free();
            if self.shadow.is_instance_valid() {
                self.shadow.clone().queue_free();
            }
            return false;
        }
        let t = self.age / self.style.duration;
        let position = self.origin + Vector3::new(0.0, self.style.ascend * t, 0.0);
        self.node.set_position(position);
        // A constant world em makes the screen height follow Retail's 1/d law with no
        // per-frame distance term: the em is the same at every distance and projection
        // divides it by the distance. Close text therefore grows; far text shrinks with no
        // floor (WorldTextMinSize is the minimum *character name* size, not combat text).
        let pixel = self.em_base * combat_text::height_scale(self.age, self.style)
            / combat_text::TEXT_FONT_SIZE as f32;
        self.node.set_pixel_size(pixel);
        let alpha = combat_text::fade_alpha(self.age, self.style);
        let mut color = self.node.get_modulate();
        color.a = alpha;
        self.node.set_modulate(color);
        if self.shadow.is_instance_valid() {
            self.shadow.set_position(position + self.shadow_offset);
            self.shadow.set_pixel_size(pixel);
            let mut shadow = self.shadow.get_modulate();
            shadow.a = alpha;
            self.shadow.set_modulate(shadow);
        }
        true
    }
}

/// Settled font size: 64 at the style's own scale (0.6 for the avoidance words, the Retail CVar
/// pair for a crit) and × 0.75 for a glancing blow (`CombatFeedback_OnCombatEvent`,
/// CombatFeedback.lua:39-42). A crit then pops over its lifetime through
/// [`combat_text::height_scale`].
pub(super) fn combat_text_size(event: &CombatLogEvent, style: Style) -> i32 {
    let mut size = combat_text::TEXT_FONT_SIZE as f32 * style.scale;
    if event.glancing {
        size *= 0.75;
    }
    size.round() as i32
}

fn preview_incoming_event(index: u32) -> CombatLogEvent {
    use shared::protocol::{CombatLogKind, MissKind};
    let mut event = CombatLogEvent {
        source: Some(2),
        target: Some(1),
        spell_id: Some(133),
        school_mask: 4,
        amount: 12345,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
        kind: CombatLogKind::Damage,
    };
    match index % 9 {
        0 => event.spell_id = None,
        1 => event.amount = 2345,
        2 => event.crit = true,
        3 => event.kind = CombatLogKind::Heal,
        4 => {
            event.kind = CombatLogKind::Heal;
            event.crit = true;
        }
        5 => {
            event.amount = 0;
            event.kind = CombatLogKind::Miss(MissKind::Dodge);
        }
        6 => {
            event.amount = 0;
            event.kind = CombatLogKind::Miss(MissKind::Absorb);
        }
        7 => event.blocked = 400,
        _ => {
            event.kind = CombatLogKind::Heal;
            event.periodic = true;
            event.crit = true;
        }
    }
    event
}

/// The fixture's scripted events, one per `index % SCRIPT.len()`: a plain hit, a crit, a
/// heal, a heal crit, Dodge, Absorb, pet melee, pet spell and a periodic tick.
fn preview_combat_event(index: u32) -> (CombatLogEvent, Attacker) {
    use shared::protocol::{CombatLogKind, MissKind};

    let event = |kind: CombatLogKind, amount: i32, crit: bool, periodic: bool, spell: bool| {
        CombatLogEvent {
            source: Some(1),
            target: Some(2),
            spell_id: spell.then_some(133),
            school_mask: 4,
            amount,
            overflow: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit,
            glancing: false,
            periodic,
            extra_spell_id: None,
            timestamp_unix_ms: 0,
            kind,
        }
    };
    let script = [
        (
            event(CombatLogKind::Damage, 1234, false, false, true),
            Attacker::Player,
        ),
        (
            event(CombatLogKind::Damage, 2345, true, false, true),
            Attacker::Player,
        ),
        (
            event(CombatLogKind::Heal, 700, false, false, true),
            Attacker::Player,
        ),
        (
            event(CombatLogKind::Heal, 900, true, false, true),
            Attacker::Player,
        ),
        (
            event(CombatLogKind::Miss(MissKind::Dodge), 0, false, false, false),
            Attacker::Player,
        ),
        (
            event(CombatLogKind::Miss(MissKind::Absorb), 0, false, false, true),
            Attacker::Player,
        ),
        (
            event(CombatLogKind::Damage, 321, false, false, false),
            Attacker::PetMelee,
        ),
        (
            event(CombatLogKind::Damage, 654, false, false, true),
            Attacker::PetSpell,
        ),
        (
            event(CombatLogKind::Damage, 88, false, true, true),
            Attacker::Player,
        ),
    ];
    script[index as usize % script.len()].clone()
}

/// The world font is `DAMAGE_TEXT_FONT` at `SystemFont_World`'s height 64 with a black
/// shadow (`Blizzard_Fonts_Shared/Shared/GameFonts.xml:22-23`, `Fonts.xml:1060-1088`).
fn world_font() -> Option<Gd<FontFile>> {
    crate::ui::assets::load_font(GameFont::FrizQuadrata).ok()
}

fn spawn_combat_label(event: &CombatLogEvent, text: &combat_text::FloatText) -> Gd<Label3D> {
    spawn_label(
        event,
        text,
        "CombatText",
        Color::from_rgb(text.color[0], text.color[1], text.color[2]),
    )
}

fn spawn_label(
    event: &CombatLogEvent,
    text: &combat_text::FloatText,
    name: &str,
    color: Color,
) -> Gd<Label3D> {
    let mut label = Label3D::new_alloc();
    label.set_name(name);
    label.set_text(&text.text);
    label.set_billboard_mode(BillboardMode::ENABLED);
    label.set_draw_flag(
        godot::classes::label_3d::DrawFlags::DISABLE_DEPTH_TEST,
        true,
    );
    if let Some(font) = world_font() {
        label.set_font(&font);
    }
    label.set_font_size(combat_text_size(event, text.style));
    label.set_modulate(color);
    label
}

fn world_text_metrics(
    camera: &Gd<Camera3D>,
    style: Style,
    options: &game_engine_core::combat_text_options::CombatTextOptions,
) -> WorldTextMetrics {
    let em_base = combat_text::em_yards_with_options(camera.get_fov(), options);
    let em = em_base * style.scale * combat_text::height_scale(0.0, style);
    let camera_basis = camera.get_global_transform().basis;
    let shadow_direction = (camera_basis.col_a() - camera_basis.col_b()).normalized();
    WorldTextMetrics {
        em_base,
        pixel_size: em_base * combat_text::height_scale(0.0, style)
            / combat_text::TEXT_FONT_SIZE as f32,
        shadow_offset: em * combat_text::SHADOW_OFFSET_EM * shadow_direction,
    }
}

fn create_world_text_nodes(
    event: &CombatLogEvent,
    text: &combat_text::FloatText,
) -> (Gd<Label3D>, Gd<Label3D>) {
    let label = spawn_combat_label(event, text);
    let mut shadow = spawn_label(event, text, "CombatTextShadow", Color::BLACK);
    shadow.set_render_priority(-1);
    shadow.set_meta("combat_text_shadow", &true.to_variant());
    (label, shadow)
}

enum CombatTextEvent {
    Incoming(CombatLogEvent, Option<String>),
    Outgoing(CombatLogEvent, Attacker),
}

impl GameClient {
    /// Advance screen-anchored self messages and world-anchored outgoing messages.
    pub(super) fn float_combat_text(&mut self, delta: f32) {
        for event in self.take_combat_text_events() {
            match event {
                CombatTextEvent::Incoming(event, healer) => {
                    self.spawn_incoming_combat_text(&event, healer.as_deref(), false);
                }
                CombatTextEvent::Outgoing(event, attacker) => {
                    let camera = self.combat_text_camera();
                    self.spawn_combat_text(&event, attacker, camera);
                }
            }
        }
        if !self.spells.combat_text_hold {
            self.spells.floating.retain_mut(|text| text.advance(delta));
            let viewport = self
                .base()
                .get_viewport()
                .map(|viewport| viewport.get_visible_rect().size)
                .unwrap_or(Vector2::ZERO);
            self.spells
                .self_floating
                .retain_mut(|text| text.advance(delta, viewport));
        }
    }

    fn combat_text_camera(&self) -> Option<Gd<Camera3D>> {
        self.base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d())
    }

    fn spawn_incoming_combat_text(
        &mut self,
        event: &CombatLogEvent,
        healer_name: Option<&str>,
        preview: bool,
    ) -> i64 {
        let options = self.self_text_options(preview);
        let Some(message) = combat_text::incoming::message_for(event, &options, healer_name) else {
            return self.spells.self_floating.len() as i64;
        };
        if self.spells.self_floating.len() >= combat_text::incoming::TEXT_POOL_SIZE {
            return self.spells.self_floating.len() as i64;
        }
        let Some(viewport) = self.incoming_viewport_size() else {
            return self.spells.self_floating.len() as i64;
        };
        let layer = self.self_combat_text_layer();
        let motion = self.build_incoming_motion(&message, options.float_mode, viewport);
        self.append_incoming_text(&layer, &message, motion, viewport);
        self.spells.self_floating.len() as i64
    }

    fn self_text_options(
        &self,
        preview: bool,
    ) -> game_engine_core::combat_text_options::CombatTextOptions {
        let mut options = self.client_options.hud.combat_text;
        if preview {
            options.self_enabled = true;
            options.avoidance = true;
            options.reductions = true;
            options.friendly_healers = true;
        }
        options
    }

    fn incoming_viewport_size(&self) -> Option<Vector2> {
        let size = self.base().get_viewport()?.get_visible_rect().size;
        (size.x > 0.0 && size.y > 0.0).then_some(size)
    }

    fn build_incoming_motion(&mut self, message: &Message, mode: i32, viewport: Vector2) -> Motion {
        let start_y = self.self_combat_text_start_y(mode, viewport);
        let start_x = sample_incoming_stagger(message);
        let x_dir = self.next_incoming_x_dir();
        let (start_x, start_y, x_dir) =
            self.adjust_overflow_motion(message, mode, viewport, start_x, start_y, x_dir);
        Motion {
            mode,
            crit: message.crit,
            start: [start_x, start_y],
            x_dir,
            viewport: [viewport.x, viewport.y],
        }
    }

    fn next_incoming_x_dir(&mut self) -> f32 {
        let next = -self.spells.self_float_x_dir;
        self.spells.self_float_x_dir = next;
        next
    }

    fn adjust_overflow_motion(
        &mut self,
        message: &Message,
        mode: i32,
        viewport: Vector2,
        mut start_x: f32,
        mut start_y: f32,
        mut x_dir: f32,
    ) -> (f32, f32, f32) {
        let lower = combat_text::incoming::TEXT_START_Y - combat_text::incoming::TEXT_MAX_SPREAD;
        let upper = combat_text::incoming::TEXT_START_Y + combat_text::incoming::TEXT_MAX_SPREAD;
        let is_down = mode == 2;
        let overflows = if is_down {
            start_y > upper
        } else {
            start_y < lower
        };
        if !overflows {
            return (start_x, start_y, x_dir);
        }
        if message.crit {
            return (start_x, combat_text::incoming::TEXT_START_Y, x_dir);
        }
        self.spells.self_float_offset_adjustment *= -1.0;
        start_y = if is_down { upper } else { lower };
        start_x += self.spells.self_float_offset_adjustment * viewport.x
            / combat_text::incoming::REFERENCE_VIEWPORT_WIDTH;
        x_dir = if self.spells.self_float_offset_adjustment > 0.0 {
            -1.0
        } else {
            1.0
        };
        (start_x, start_y, x_dir)
    }

    fn append_incoming_text(
        &mut self,
        layer: &Gd<CanvasLayer>,
        message: &Message,
        motion: Motion,
        viewport: Vector2,
    ) {
        let base_height = combat_text::incoming::font_height(message.crit, viewport.y);
        let (label, shadow) = self.create_incoming_labels(layer, message, base_height, viewport);
        let mut text = SelfFloatingText {
            label,
            shadow,
            motion,
            base_height,
            age: 0.0,
        };
        text.advance(0.0, viewport);
        self.spells.self_floating.push(text);
    }

    fn self_combat_text_layer(&mut self) -> Gd<CanvasLayer> {
        if let Some(layer) = &self.spells.self_float_layer {
            return layer.clone();
        }
        let mut layer = CanvasLayer::new_alloc();
        layer.set_name("FloatingCombatTextLayer");
        layer.set_layer(10);
        crate::rebuild_guard::count(Rebuild::UiTree, 1);
        self.base_mut().add_child(&layer);
        self.spells.self_float_layer = Some(layer.clone());
        layer
    }

    fn create_incoming_labels(
        &self,
        layer: &Gd<CanvasLayer>,
        message: &Message,
        base_height: f32,
        viewport: Vector2,
    ) -> (Gd<Label>, Gd<Label>) {
        let shadow = self.new_incoming_label(message, base_height, viewport, Color::BLACK);
        let color = Color::from_rgb(message.color[0], message.color[1], message.color[2]);
        let label = self.new_incoming_label(message, base_height, viewport, color);
        let mut layer = layer.clone();
        layer.add_child(&shadow);
        layer.add_child(&label);
        crate::rebuild_guard::count(Rebuild::UiTree, 2);
        (label, shadow)
    }

    fn new_incoming_label(
        &self,
        message: &Message,
        base_height: f32,
        viewport: Vector2,
        color: Color,
    ) -> Gd<Label> {
        let mut label = Label::new_alloc();
        label.set_name(if color == Color::BLACK {
            "CombatTextShadow"
        } else {
            "CombatText"
        });
        label.set_text(&message.text);
        label.set_horizontal_alignment(godot::global::HorizontalAlignment::CENTER);
        label.set_vertical_alignment(godot::global::VerticalAlignment::TOP);
        label.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
        let box_height = combat_text::incoming::FONT_BOX_HEIGHT * viewport.y
            / combat_text::incoming::REFERENCE_VIEWPORT_HEIGHT;
        label.set_size(Vector2::new(viewport.x, box_height));
        if color == Color::BLACK {
            label.set_meta("combat_text_shadow", &true.to_variant());
        }
        if let Some(font) = world_font() {
            label.change_theme_font_override("font", &font);
        }
        label.change_theme_font_size_override("font_size", base_height.round() as i32);
        label.change_theme_color_override("font_color", color);
        label
    }

    fn self_combat_text_start_y(&self, mode: i32, viewport: Vector2) -> f32 {
        let previous: Vec<f32> = self
            .spells
            .self_floating
            .iter()
            .map(|text| {
                let frame = text.motion.frame(text.age);
                (viewport.y - frame.position[1]) * combat_text::incoming::REFERENCE_VIEWPORT_HEIGHT
                    / viewport.y
            })
            .collect();
        combat_text::incoming::stack_start_y(mode, viewport.y, &previous)
    }

    pub(crate) fn preview_incoming_float_text(&mut self, index: u32) -> i64 {
        let event = preview_incoming_event(index);
        self.spawn_incoming_combat_text(&event, Some("Alice"), true)
    }

    fn take_combat_text_events(&mut self) -> Vec<CombatTextEvent> {
        let Some(player) = self.world.local_player_id() else {
            self.spells.combat_seen = self.account.combat_log_seq;
            return Vec::new();
        };
        let sequence = self.account.combat_log_seq;
        let fresh = (sequence - self.spells.combat_seen).min(self.account.combat_log.len() as u64);
        self.spells.combat_seen = sequence;
        let first_fresh = self.account.combat_log.len() - fresh as usize;
        self.account
            .combat_log
            .iter()
            .skip(first_fresh)
            .filter(|event| {
                matches!(
                    event.kind,
                    CombatLogKind::Damage | CombatLogKind::Heal | CombatLogKind::Miss(_)
                )
            })
            .filter_map(|event| self.classify_combat_text_event(event, player))
            .collect()
    }

    fn classify_combat_text_event(
        &self,
        event: &CombatLogEvent,
        player: u64,
    ) -> Option<CombatTextEvent> {
        if event.target == Some(player) {
            let healer = event
                .source
                .filter(|source| *source != player)
                .and_then(|source| self.replica.unit(source))
                .and_then(UnitFields::name)
                .map(str::to_owned);
            return Some(CombatTextEvent::Incoming(event.clone(), healer));
        }
        let attacker = self.combat_text_attacker(event, player)?;
        Some(CombatTextEvent::Outgoing(event.clone(), attacker))
    }

    /// Who caused `event` from the local player's view: the player, or a pet/guardian it owns
    /// (`IsOwnedByActivePlayer`, `Unit_C.cpp:17681-17702`). A pet's melee swing carries no
    /// spell, its abilities do (`Unit_C.cpp:17753-17757`).
    fn combat_text_attacker(&self, event: &CombatLogEvent, player: u64) -> Option<Attacker> {
        let source = event.source?;
        if source == player {
            return Some(Attacker::Player);
        }
        let unit = self.replica.unit(source)?;
        let owner = unit.summoned_by()?;
        let owner_of_owner = self.replica.unit(owner)?.summoned_by();
        if owner != player && owner_of_owner != Some(player) {
            return None;
        }
        Some(if event.spell_id.is_none() {
            Attacker::PetMelee
        } else {
            Attacker::PetSpell
        })
    }

    fn spawn_combat_text(
        &mut self,
        event: &CombatLogEvent,
        attacker: Attacker,
        camera: Option<Gd<Camera3D>>,
    ) {
        let (Some(unit_id), Some(unit), Some(camera)) = (
            event.target,
            event.target.and_then(|id| self.world.unit_node(id)),
            camera,
        ) else {
            return;
        };
        // 15890 draws no world text on the active player; its damage taken is the screen
        // combat text frame instead (`Unit_C.cpp:17744-17746`).
        if Some(unit_id) == self.world.local_player_id() {
            return;
        }
        let anchor = unit.get_global_position() + Vector3::new(0.0, FLOAT_TEXT_HEIGHT, 0.0);
        self.spawn_combat_text_at(unit_id, anchor, event, attacker, camera);
    }

    /// Spawn one world text for `unit_id` at `anchor` (over its head), respecting that unit's
    /// slots. Returns the live text count.
    fn spawn_combat_text_at(
        &mut self,
        unit_id: u64,
        anchor: Vector3,
        event: &CombatLogEvent,
        attacker: Attacker,
        camera: Gd<Camera3D>,
    ) -> i64 {
        let Some(text) = combat_text::text_for_with_options(
            event,
            attacker,
            &self.client_options.hud.combat_text,
        ) else {
            return self.spells.floating.len() as i64;
        };
        self.take_combat_text_slot(unit_id, text.spell_crit);
        let (label, shadow) = create_world_text_nodes(event, &text);
        let origin = self.place_combat_text(
            &camera,
            anchor,
            unit_id,
            combat_text_size(event, text.style) as f32,
        );
        let metrics = world_text_metrics(&camera, text.style, &self.client_options.hud.combat_text);
        self.attach_world_text(unit_id, origin, &text, label, shadow, metrics);
        self.spells.floating.len() as i64
    }

    fn attach_world_text(
        &mut self,
        unit: u64,
        origin: Vector3,
        text: &combat_text::FloatText,
        mut label: Gd<Label3D>,
        mut shadow: Gd<Label3D>,
        metrics: WorldTextMetrics,
    ) {
        label.set_position(origin);
        label.set_pixel_size(metrics.pixel_size);
        shadow.set_position(origin);
        shadow.set_pixel_size(metrics.pixel_size);
        self.base_mut().add_child(&shadow);
        self.base_mut().add_child(&label);
        self.spells.floating.push(FloatingText {
            node: label,
            shadow,
            shadow_offset: metrics.shadow_offset,
            em_base: metrics.em_base,
            unit,
            age: 0.0,
            origin,
            style: text.style,
            spell_crit: text.spell_crit,
        });
    }

    /// Offline fixture: one scripted world text over `at`, for pseudo-unit `unit`; `index`
    /// selects the event. Returns the live text count.
    pub(crate) fn preview_float_text(&mut self, index: u32, unit: u64, at: Vector3) -> i64 {
        let Some(camera) = self.combat_text_camera() else {
            return self.spells.floating.len() as i64;
        };
        let (event, attacker) = preview_combat_event(index);
        self.spawn_combat_text_at(unit, at, &event, attacker, camera)
    }

    /// Offline fixture: hold every live world text at its current age, so an offline capture
    /// can take as many slow frames as it needs.
    pub(crate) fn preview_float_text_hold(&mut self, hold: bool) {
        self.spells.combat_text_hold = hold;
    }

    /// Offline fixture: advance every live world text by `seconds`, as a frame would, and
    /// return the live count. Lets a capture decide the texts' age.
    pub(crate) fn preview_float_text_advance(&mut self, seconds: f32) -> i64 {
        self.spells
            .floating
            .retain_mut(|text| text.advance(seconds));
        let viewport = self
            .base()
            .get_viewport()
            .map(|viewport| viewport.get_visible_rect().size)
            .unwrap_or(Vector2::ZERO);
        self.spells
            .self_floating
            .retain_mut(|text| text.advance(seconds, viewport));
        (self.spells.floating.len() + self.spells.self_floating.len()) as i64
    }

    /// Offline fixture: microseconds one frame spends advancing all live world texts.
    pub(crate) fn preview_float_text_cost(&mut self, iterations: u32) -> f64 {
        let iterations = iterations.clamp(1, 100_000);
        let started = std::time::Instant::now();
        let viewport = self
            .base()
            .get_viewport()
            .map(|viewport| viewport.get_visible_rect().size)
            .unwrap_or(Vector2::ZERO);
        for _ in 0..iterations {
            self.spells.floating.retain_mut(|text| text.advance(0.0));
            self.spells
                .self_floating
                .retain_mut(|text| text.advance(0.0, viewport));
        }
        started.elapsed().as_secs_f64() * 1_000_000.0 / f64::from(iterations)
    }

    /// Reserve one of the unit's four world-text slots (Retail 12.1.5): a spell crit replaces
    /// the unit's existing spell crit; otherwise the first free slot is used, and when all
    /// four are taken the last one is evicted.
    fn take_combat_text_slot(&mut self, unit: u64, spell_crit: bool) {
        let slots: Vec<usize> = self
            .spells
            .floating
            .iter()
            .enumerate()
            .filter(|(_, text)| text.unit == unit)
            .map(|(index, _)| index)
            .collect();
        let spell_crit_slot =
            if spell_crit {
                slots.iter().enumerate().rev().find_map(|(slot, index)| {
                    self.spells.floating[*index].spell_crit.then_some(slot)
                })
            } else {
                None
            };
        if let Some(slot) = combat_text::slot_eviction(slots.len(), spell_crit_slot) {
            let Some(index) = slots.get(slot).copied() else {
                return;
            };
            let text = self.spells.floating.remove(index);
            text.node.clone().queue_free();
            if text.shadow.is_instance_valid() {
                text.shadow.clone().queue_free();
            }
        }
    }

    /// The number's start point: sideways jitter from the R2 sequence, lifted by one text
    /// height per live text already on that unit, so a unit's four slots stack instead of
    /// covering each other (Retail's per-slot placement, WorldText.cpp:441).
    fn place_combat_text(
        &mut self,
        camera: &Gd<Camera3D>,
        anchor: Vector3,
        unit_id: u64,
        font_px: f32,
    ) -> Vector3 {
        let camera_transform = camera.get_global_transform();
        // Text is a constant world size, so its height is the same at every distance and the
        // spread is a constant world offset too.
        let height = combat_text::em_yards_with_options(
            camera.get_fov(),
            &self.client_options.hud.combat_text,
        ) * font_px
            / combat_text::TEXT_FONT_SIZE as f32;
        let origin = anchor
            + combat_text::side_offset(
                self.spells.floats_spawned,
                camera_transform.basis.col_a(),
                combat_text::SPREAD_HEIGHTS * height,
            );
        self.spells.floats_spawned = self.spells.floats_spawned.wrapping_add(1);
        let live: Vec<(f32, f32)> = self
            .spells
            .floating
            .iter()
            .filter(|live| live.unit == unit_id)
            .map(|live| (live.style.ascend, live.em_base * live.style.scale))
            .collect();
        let offset = combat_text::stack_offset(&live, height);
        origin + camera_transform.basis.col_b() * offset
    }
}
