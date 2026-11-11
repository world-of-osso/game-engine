//! Ground cursor placement before unit picking or camera steering.
use std::path::Path;

use game_engine_core::spell_catalog::SpellRange;
use godot::{
    classes::{Decal, InputEvent, InputEventMouseButton, decal::DecalTexture},
    global::MouseButton,
    prelude::*,
};

use super::SpellsHud;
use crate::{GameClient, frame_error::FrameError};

const RETICLE_DEPTH: f32 = 1.0;
const RETICLE_LIFT: f32 = 0.06;

impl SpellsHud {
    pub(super) fn cancel_ground_target(&mut self) {
        self.ground.cancel();
        self.ground_toy = None;
        self.item.cancel();
        self.clear_ground_reticle();
    }

    fn clear_ground_reticle(&mut self) {
        if let Some(reticle) = self.reticle.take() {
            reticle.free();
        }
        self.reticle_valid = None;
    }
}

impl GameClient {
    pub(crate) fn begin_ground_toy(&mut self, item_id: u32, spell_id: u32) -> bool {
        let Some(spell) = self.spells.catalog().and_then(|data| data.get(spell_id)) else {
            return false;
        };
        if !spell.ground_targeted {
            return false;
        }
        let name = spell.name.to_string();
        self.spells.cancel_ground_target();
        self.spells.ground.begin(shared::protocol::SpellCastIntent {
            spell_id: Some(spell_id),
            spell: name,
            target_entity: None,
            target_item_guid: None,
            witness: None,
            destination: None,
        });
        self.spells.ground_toy = Some(item_id);
        true
    }

    pub(crate) fn cancel_ground_target(&mut self) -> bool {
        let active = self.spells.ground.active() || self.spells.item.active();
        self.spells.cancel_ground_target();
        active
    }

    /// Only an unhandled world click places a spell; clicks on frames keep targeting.
    pub(crate) fn ground_spell_pointer(
        &mut self,
        event: &Gd<InputEvent>,
    ) -> Result<bool, FrameError> {
        if self.spells.item.active() {
            return Ok(self.item_spell_world_pointer(event));
        }
        if !self.spells.ground.active() {
            return Ok(false);
        }
        let Ok(mouse) = event.clone().try_cast::<InputEventMouseButton>() else {
            return Ok(false);
        };
        if !mouse.is_pressed() {
            return Ok(false);
        }
        match mouse.get_button_index() {
            MouseButton::RIGHT => {
                self.spells.cancel_ground_target();
            }
            MouseButton::LEFT => self.place_ground_spell(mouse.get_position())?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn item_spell_world_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        let Ok(mouse) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        if !mouse.is_pressed() {
            return false;
        }
        match mouse.get_button_index() {
            MouseButton::RIGHT => self.spells.item.cancel(),
            // Disenchant cannot target world units; retain the cursor.
            MouseButton::LEFT => {}
            _ => return false,
        }
        true
    }

    fn place_ground_spell(&mut self, pointer: Vector2) -> Result<(), FrameError> {
        let Some(point) = self.raycast_ground_spell_point(pointer) else {
            return Ok(());
        };
        let root = self
            .world
            .root()
            .ok_or("Ground targeting has no world root")?;
        let destination = crate::pet_bar::position_from_world(root.get_global_transform(), point);
        let Some(intent) = self.spells.ground.place(destination) else {
            return Ok(());
        };
        let id = intent.spell_id.expect("ground cursor stores a spell ID");
        if let Some(item_id) = self.spells.ground_toy.take() {
            self.send_toy_activation(item_id, intent.destination)?;
        } else {
            self.account.send_spell_intent(intent)?;
        }
        self.spells.sent.push(id);
        self.spells.cancel_ground_target();
        Ok(())
    }

    fn raycast_ground_spell_point(&self, pointer: Vector2) -> Option<Vector3> {
        crate::pet_bar::ground_under(self.world_camera.camera()?, pointer)
    }

    pub(crate) fn update_ground_spell_reticle(&mut self) -> Result<(), FrameError> {
        if !self.account.session.gameplay_input_allowed() || self.game_menu_ui.is_some() {
            self.spells.cancel_ground_target();
            return Ok(());
        }
        if !self.spells.ground.active() {
            return Ok(());
        }
        let spell_id = self
            .spells
            .ground
            .spell_id()
            .ok_or("Active ground target has no spell id")?;
        let Some((texture_fdid, radius_yd, range)) = self.ground_reticle_data(spell_id)? else {
            self.spells.clear_ground_reticle();
            return Ok(());
        };
        self.update_ground_reticle_at_pointer(texture_fdid, radius_yd, range)
    }

    fn update_ground_reticle_at_pointer(
        &mut self,
        texture_fdid: u32,
        radius_yd: f32,
        range: SpellRange,
    ) -> Result<(), FrameError> {
        let pointer = Vector2::from_array(self.physical_input.pointer());
        let point = self.raycast_ground_spell_point(pointer);
        let valid = point
            .map(|point| self.ground_target_point_is_valid(range, point))
            .transpose()?;
        self.ensure_ground_reticle(texture_fdid, radius_yd, valid)?;
        if let Some(valid) = valid {
            self.set_ground_reticle_valid(valid)?;
        }
        if let Some(reticle) = self.spells.reticle.as_mut() {
            let visible = point.is_some();
            if reticle.is_visible() != visible {
                reticle.set_visible(visible);
            }
            if let Some(point) = point {
                let position = point + Vector3::UP * RETICLE_LIFT;
                if reticle.get_global_position() != position {
                    reticle.set_global_position(position);
                }
            }
        }
        Ok(())
    }

    fn ensure_ground_reticle(
        &mut self,
        texture_fdid: u32,
        radius_yd: f32,
        valid: Option<bool>,
    ) -> Result<(), FrameError> {
        if self.spells.reticle.is_some() {
            return Ok(());
        }
        let Some(valid) = valid else {
            return Ok(());
        };
        let reticle = spawn_reticle(
            &self.data_root.join("textures"),
            texture_fdid,
            radius_yd,
            valid,
        )?;
        self.base_mut().add_child(&reticle);
        self.spells.reticle = Some(reticle);
        self.spells.reticle_valid = Some(valid);
        Ok(())
    }

    fn ground_target_point_is_valid(
        &self,
        range: SpellRange,
        point: Vector3,
    ) -> Result<bool, FrameError> {
        let player = self
            .world
            .local_player_node()
            .ok_or("Ground targeting has no local player")?;
        let distance_yd = player.get_global_position().distance_to(point);
        Ok(range.contains_hostile_distance_yd(distance_yd))
    }

    fn ground_reticle_data(
        &self,
        spell_id: u32,
    ) -> Result<Option<(u32, f32, SpellRange)>, FrameError> {
        let spell = self
            .spells
            .catalog()
            .and_then(|catalog| catalog.get(spell_id))
            .ok_or_else(|| format!("Ground-target spell {spell_id} is missing from the catalog"))?;
        let texture_fdid = spell
            .ground_target_texture_fdid()
            .ok_or_else(|| format!("Ground-target spell {spell_id} has no targetarea texture"))?;
        let Some(radius_yd) = spell.ground_target_radius_yd() else {
            return Ok(None);
        };
        Ok(Some((texture_fdid, radius_yd, spell.range)))
    }

    fn set_ground_reticle_valid(&mut self, valid: bool) -> Result<(), FrameError> {
        if self.spells.reticle_valid == Some(valid) {
            return Ok(());
        }
        let reticle = self
            .spells
            .reticle
            .as_mut()
            .ok_or("Ground target decal is missing")?;
        reticle.set_modulate(ground_reticle_tint(valid));
        self.spells.reticle_valid = Some(valid);
        crate::rebuild_guard::count(crate::rebuild_guard::Rebuild::MaterialRebind, 1);
        Ok(())
    }
}

fn ground_reticle_tint(valid: bool) -> Color {
    if valid {
        Color::from_rgba(0.1, 1.0, 0.2, 0.8)
    } else {
        Color::from_rgba(1.0, 0.1, 0.1, 0.8)
    }
}

fn spawn_reticle(
    texture_dir: &Path,
    texture_fdid: u32,
    radius_yd: f32,
    valid: bool,
) -> Result<Gd<Decal>, FrameError> {
    let mut missing = PackedInt32Array::new();
    let texture = crate::assets::material::shared_texture(texture_fdid, texture_dir, &mut missing)?
        .ok_or_else(|| format!("Ground target decal texture {texture_fdid} is missing"))?;
    let diameter_yd = radius_yd * 2.0;
    let mut decal = Decal::new_alloc();
    decal.set_name("GroundSpellReticle");
    decal.set_size(Vector3::new(diameter_yd, RETICLE_DEPTH, diameter_yd));
    decal.set_position(Vector3::new(0.0, RETICLE_LIFT, 0.0));
    decal.set_texture(DecalTexture::ALBEDO, &texture);
    decal.set_modulate(ground_reticle_tint(valid));
    crate::rebuild_guard::count(crate::rebuild_guard::Rebuild::MaterialRebind, 1);
    Ok(decal)
}
