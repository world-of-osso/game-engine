//! Floating damage labels and their animation.

use crate::{GameClient, combat_text, replicated::UnitFields};
use godot::classes::{Camera3D, FontFile, Label3D, base_material_3d::BillboardMode};
use godot::prelude::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};
use ui_toolkit::widgets::font_string::GameFont;

use combat_text::{Attacker, Style};

/// Height above the unit origin where combat text starts.
const FLOAT_TEXT_HEIGHT: f32 = 3.6;

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
}

impl FloatingText {
    fn advance(&mut self, delta: f32, eye: Option<Vector3>) -> bool {
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
        let distance = eye.map_or(combat_text::DISTANCE_ANCHOR_YARDS, |eye| {
            eye.distance_to(self.origin)
        });
        let pixel = combat_text::TEXT_PIXEL
            * combat_text::height_scale(self.age, self.style)
            * combat_text::distance_scale(distance);
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

/// Settled font size: 64 at the style's own scale (0.6 for the avoidance words), × 1.5 for a
/// crit and × 0.75 for a glancing blow (`WorldText.cpp:44-56` for the crit ratio;
/// `CombatFeedback_OnCombatEvent`, CombatFeedback.lua:39-42, for the glancing size). A crit
/// then pops over its lifetime through [`combat_text::height_scale`].
pub(super) fn combat_text_size(event: &CombatLogEvent, style: Style) -> i32 {
    let mut size = combat_text::TEXT_FONT_SIZE as f32 * style.scale;
    if event.glancing {
        size *= 0.75;
    }
    size.round() as i32
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
    // Constant on-screen size, as 15890 draws world text in a screen-space ortho projection
    // at a fixed NDC height (`WorldText.cpp:234-253`).
    label.set_draw_flag(godot::classes::label_3d::DrawFlags::FIXED_SIZE, true);
    label.set_modulate(color);
    label
}

impl GameClient {
    /// Retail floating combat text over the target for the player's own damage.
    pub(super) fn float_combat_text(&mut self, delta: f32) {
        let events = self.take_combat_text_events();
        let camera = self.combat_text_camera();
        for (event, attacker) in events {
            self.spawn_combat_text(&event, attacker, camera.clone());
        }
        let eye = camera.map(|camera| camera.get_global_transform().origin);
        if !self.spells.combat_text_hold {
            self.spells
                .floating
                .retain_mut(|text| text.advance(delta, eye));
        }
    }

    fn combat_text_camera(&self) -> Option<Gd<Camera3D>> {
        self.base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d())
    }

    fn take_combat_text_events(&mut self) -> Vec<(CombatLogEvent, Attacker)> {
        let Some(player) = self.world.local_player_id() else {
            self.spells.combat_seen = self.account.combat_log_seq;
            return Vec::new();
        };
        let seq = self.account.combat_log_seq;
        let fresh = (seq - self.spells.combat_seen).min(self.account.combat_log.len() as u64);
        self.spells.combat_seen = seq;
        self.account
            .combat_log
            .iter()
            .skip(self.account.combat_log.len() - fresh as usize)
            .filter(|event| {
                matches!(
                    event.kind,
                    CombatLogKind::Damage | CombatLogKind::Heal | CombatLogKind::Miss(_)
                )
            })
            .filter_map(|event| {
                let attacker = self.combat_text_attacker(event, player)?;
                Some((event.clone(), attacker))
            })
            .collect()
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
        let Some(text) = combat_text::text_for(event, attacker) else {
            return self.spells.floating.len() as i64;
        };
        self.take_combat_text_slot(unit_id, text.spell_crit);
        let mut label = spawn_combat_label(event, &text);
        let mut shadow = spawn_label(event, &text, "CombatTextShadow", Color::BLACK);
        shadow.set_render_priority(-1);
        shadow.set_meta("combat_text_shadow", &true.to_variant());
        // Under the client root: unit nodes carry model scale. Each number starts at its own
        // sideways point in the camera plane, lifted one text height per live number on the
        // unit, so a unit's slots stack (Retail's per-slot placement, WorldText.cpp:441).
        let origin = self.place_combat_text(
            &camera,
            anchor,
            unit_id,
            combat_text_size(event, text.style) as f32,
        );
        let camera_transform = camera.get_global_transform();
        let distance = camera_transform.origin.distance_to(origin);
        let shadow_step = combat_text::shadow_step_yards(
            combat_text::TEXT_PIXEL * combat_text::distance_scale(distance),
            distance,
            combat_text_size(event, text.style) as f32,
            camera.get_fov(),
            Self::viewport_height(&camera),
        ) * (camera_transform.basis.col_a() - camera_transform.basis.col_b())
            .normalized();
        label.set_position(origin);
        label.set_pixel_size(
            combat_text::TEXT_PIXEL
                * combat_text::height_scale(0.0, text.style)
                * combat_text::distance_scale(
                    camera.get_global_transform().origin.distance_to(origin),
                ),
        );
        shadow.set_position(origin);
        shadow.set_pixel_size(label.get_pixel_size());
        self.base_mut().add_child(&shadow);
        self.base_mut().add_child(&label);
        self.spells.floating.push(FloatingText {
            node: label,
            shadow,
            shadow_offset: shadow_step,
            unit: unit_id,
            age: 0.0,
            origin,
            style: text.style,
            spell_crit: text.spell_crit,
        });
        self.spells.floating.len() as i64
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
        let eye = self
            .combat_text_camera()
            .map(|camera| camera.get_global_transform().origin);
        self.spells
            .floating
            .retain_mut(|text| text.advance(seconds, eye));
        self.spells.floating.len() as i64
    }

    /// Offline fixture: microseconds one frame spends advancing all live world texts.
    pub(crate) fn preview_float_text_cost(&mut self, iterations: u32) -> f64 {
        let iterations = iterations.clamp(1, 100_000);
        let started = std::time::Instant::now();
        for _ in 0..iterations {
            self.spells
                .floating
                .retain_mut(|text| text.advance(0.0, None));
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
        let spell_crit_slot = if spell_crit {
            slots
                .iter()
                .rev()
                .copied()
                .find(|index| self.spells.floating[*index].spell_crit)
        } else {
            None
        };
        if let Some(index) = combat_text::slot_eviction(slots.len(), spell_crit_slot) {
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
        let distance = camera_transform.origin.distance_to(anchor);
        // Fixed-size text is a screen rectangle: its world height grows with distance, so the
        // spread and the per-slot ladder are measured in that height at the target's depth.
        let height = combat_text::text_height_yards(
            font_px,
            combat_text::TEXT_PIXEL * combat_text::distance_scale(distance),
            distance,
            camera.get_fov(),
            Self::viewport_height(camera),
        );
        let origin = anchor
            + combat_text::side_offset(
                self.spells.floats_spawned,
                camera_transform.basis.col_a(),
                combat_text::SPREAD_HEIGHTS * height,
            );
        self.spells.floats_spawned = self.spells.floats_spawned.wrapping_add(1);
        let slot = self
            .spells
            .floating
            .iter()
            .filter(|live| live.unit == unit_id)
            .count();
        // `WorldTextRandomZMin_v2` is the lowest start height; each further slot sits one text
        // height above it.
        origin
            + camera_transform.basis.col_b() * (height * (combat_text::RANDOM_Z_MIN + slot as f32))
    }

    /// Viewport height in pixels: the reference Godot's fixed-size text factor scales with.
    fn viewport_height(camera: &Gd<Camera3D>) -> f32 {
        camera
            .get_viewport()
            .map_or(720.0, |viewport| viewport.get_visible_rect().size.y)
    }
}
