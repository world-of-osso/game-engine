//! Ground cursor placement before unit picking or camera steering.
use super::SpellsHud;
use crate::{GameClient, frame_error::FrameError};
use godot::{
    classes::{
        InputEvent, InputEventMouseButton, MeshInstance3D, StandardMaterial3D, TorusMesh,
        base_material_3d::{ShadingMode, Transparency},
    },
    global::MouseButton,
    prelude::*,
};

impl SpellsHud {
    pub(super) fn cancel_ground_target(&mut self) {
        self.ground.cancel();
        self.item.cancel();
        if let Some(reticle) = self.reticle.take() {
            reticle.free();
        }
    }
}

impl GameClient {
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
        self.account.send_spell_intent(intent)?;
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
        let point =
            self.raycast_ground_spell_point(Vector2::from_array(self.physical_input.pointer()));
        if self.spells.reticle.is_none() {
            let reticle = spawn_reticle();
            self.base_mut().add_child(&reticle);
            self.spells.reticle = Some(reticle);
        }
        let reticle = self.spells.reticle.as_mut().expect("reticle created");
        reticle.set_visible(point.is_some());
        if let Some(point) = point {
            reticle.set_global_position(point + Vector3::UP * 0.06);
        }
        Ok(())
    }
}

fn spawn_reticle() -> Gd<MeshInstance3D> {
    let mut mesh = TorusMesh::new_gd();
    mesh.set_inner_radius(0.92);
    mesh.set_outer_radius(1.0);
    let mut material = StandardMaterial3D::new_gd();
    material.set_shading_mode(ShadingMode::UNSHADED);
    material.set_transparency(Transparency::ALPHA);
    material.set_albedo(Color::from_rgba(0.1, 1.0, 0.2, 0.8));
    let mut node = MeshInstance3D::new_alloc();
    node.set_name("GroundSpellReticle");
    node.set_mesh(&mesh);
    node.set_material_override(&material);
    node.set_scale(Vector3::new(1.0, 0.05, 1.0));
    node
}
