//! Floating damage labels and their animation.

use crate::{GameClient, combat_text};
use godot::classes::{Label3D, base_material_3d::BillboardMode};
use godot::prelude::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};

/// Floating combat text rises this far over its lifetime.
const FLOAT_TEXT_RISE: f32 = 1.5;
const FLOAT_TEXT_SECS: f32 = 1.5;
/// Height above the unit origin where combat text starts.
const FLOAT_TEXT_HEIGHT: f32 = 3.6;
/// Combat text `pixel_size` at its settled size.
const FLOAT_TEXT_PIXEL: f32 = 0.0016;
/// Font size of a hit; a crit is 1.5 times it.
const FLOAT_TEXT_FONT_SIZE: i32 = 64;
/// Normal text heights in one unit of the combat text start spread.
const FLOAT_TEXT_SPREAD_HEIGHTS: f32 = 1.5;

pub(super) struct FloatingText {
    pub(super) node: Gd<Label3D>,
    age: f32,
    origin: Vector3,
    crit: bool,
}

/// Floating combat text size: 64, × 1.5 for a crit and × 0.75 for a glancing blow
/// (`CombatFeedback_OnCombatEvent`, CombatFeedback.lua:39-42).
pub(super) fn combat_text_size(event: &CombatLogEvent) -> i32 {
    if event.crit {
        FLOAT_TEXT_FONT_SIZE * 3 / 2
    } else if event.glancing {
        FLOAT_TEXT_FONT_SIZE * 3 / 4
    } else {
        FLOAT_TEXT_FONT_SIZE
    }
}

impl GameClient {
    /// Retail floating combat text over the target for the player's own damage.
    pub(super) fn float_combat_text(&mut self, delta: f32) {
        let player = self.world.local_player_id();
        let seq = self.account.combat_log_seq;
        let fresh = (seq - self.spells.combat_seen).min(self.account.combat_log.len() as u64);
        self.spells.combat_seen = seq;
        let events: Vec<_> = self
            .account
            .combat_log
            .iter()
            .skip(self.account.combat_log.len() - fresh as usize)
            .filter(|event| event.source.is_some() && event.source == player)
            .filter(|event| matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_)))
            .cloned()
            .collect();
        let camera = self
            .base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d())
            .map(|camera| camera.get_global_transform());
        for event in events {
            let (Some(unit), Some(camera)) =
                (event.target.and_then(|id| self.world.unit_node(id)), camera)
            else {
                continue;
            };
            let text = match event.kind {
                CombatLogKind::Miss(kind) => format!("{kind:?}"),
                _ if event.crit => format!("{}!", event.amount),
                _ => event.amount.to_string(),
            };
            let mut label = Label3D::new_alloc();
            label.set_name("CombatText");
            label.set_text(&text);
            label.set_billboard_mode(BillboardMode::ENABLED);
            label.set_draw_flag(
                godot::classes::label_3d::DrawFlags::DISABLE_DEPTH_TEST,
                true,
            );
            label.set_font_size(combat_text_size(&event));
            label.set_outline_size(8);
            // Constant on-screen size, as Retail combat text.
            label.set_draw_flag(godot::classes::label_3d::DrawFlags::FIXED_SIZE, true);
            // Retail white for physical damage, yellow for spell schools.
            let color = if event.school_mask == 1 {
                Color::from_rgb(1.0, 1.0, 1.0)
            } else {
                Color::from_rgb(1.0, 1.0, 0.0)
            };
            label.set_modulate(color);
            // Under the client root: unit nodes carry model scale. Each number starts at
            // its own offset in the camera plane so simultaneous ones do not stack.
            let anchor = unit.get_global_position() + Vector3::new(0.0, FLOAT_TEXT_HEIGHT, 0.0);
            // A fixed-size label is `font_size * pixel_size` world units tall per unit of
            // camera distance; the spread unit is 1.5 normal text heights at its depth.
            let spread = FLOAT_TEXT_SPREAD_HEIGHTS
                * FLOAT_TEXT_FONT_SIZE as f32
                * FLOAT_TEXT_PIXEL
                * camera.origin.distance_to(anchor);
            let origin = anchor
                + combat_text::start_offset(
                    self.spells.floats_spawned,
                    camera.basis.col_a(),
                    spread,
                );
            self.spells.floats_spawned = self.spells.floats_spawned.wrapping_add(1);
            label.set_position(origin);
            label.set_pixel_size(FLOAT_TEXT_PIXEL * combat_text::ramp_scale(0.0, event.crit));
            self.base_mut().add_child(&label);
            self.spells.floating.push(FloatingText {
                node: label,
                age: 0.0,
                origin,
                crit: event.crit,
            });
        }
        self.spells.floating.retain_mut(|text| {
            text.age += delta;
            if !text.node.is_instance_valid() {
                return false;
            }
            if text.age >= FLOAT_TEXT_SECS {
                text.node.clone().queue_free();
                return false;
            }
            let t = text.age / FLOAT_TEXT_SECS;
            text.node
                .set_position(text.origin + Vector3::new(0.0, FLOAT_TEXT_RISE * t, 0.0));
            text.node
                .set_pixel_size(FLOAT_TEXT_PIXEL * combat_text::ramp_scale(text.age, text.crit));
            let mut color = text.node.get_modulate();
            color.a = 1.0 - t * t;
            text.node.set_modulate(color);
            true
        });
    }
}
