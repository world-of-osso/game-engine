//! The pet action bar (Retail `PetActionBar`, Blizzard_ActionBar/Shared/PetActionBar.lua):
//! the server's `PetSpells` bar while the local player's pet is replicated, its buttons
//! clicked or pressed through `BONUSACTIONBUTTON1..10`, each sending `PetAction`
//! (`CastPetAction`). Move To enters ground targeting: the next left click on the world
//! ground sends that point in `Position` space; right click or Escape cancels.

use game_engine_core::input_bindings_data::{BindingMouseButton, InputAction, InputBinding};
use game_engine_session::SessionScreen;
use game_engine_ui_model::pet_action_bar_component::{
    PET_BAR_ART_FDIDS, PET_BAR_BUTTONS, PetActionBarState, PetActionSlot, attack_flash_shown,
    parse_pet_action_button, pet_action_active, pet_bar_buttons,
};
use godot::classes::{Camera3D, CollisionObject3D, PhysicsRayQueryParameters3D};
use godot::prelude::*;
use shared::components::UnitFlags;
use shared::protocol::{COMMAND_ATTACK, COMMAND_MOVE_TO, PetAction, PetSpells};

use crate::frame_error::{FrameError, SessionError};
use crate::replicated::UnitFields;
use crate::terrain::doodad_collision::DOODAD_LAYER;
use crate::wmo::collision::{TERRAIN_LAYER, WMO_LAYER};
use crate::{GameClient, ui::RegistryUi};

/// `PushedTexture` stays for this long after a key press.
const PUSH_SECS: f32 = 0.15;

#[derive(Default)]
pub(crate) struct PetBarHud {
    ui: Option<Gd<RegistryUi>>,
    /// Seconds since the Attack button started flashing (`StartFlash`); `None` while not.
    flash: Option<f32>,
    pushed: [f32; PET_BAR_BUTTONS],
    /// The Move To button awaiting its ground click: the packed button and its pet.
    move_to: Option<(u64, u32)>,
    /// `PetAction`s sent, oldest first, for automation.
    sent: Vec<PetAction>,
}

impl PetBarHud {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        match &mut self.ui {
            Some(ui) => visit(ui),
            None => Ok(()),
        }
    }

    fn close(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.flash = None;
        self.move_to = None;
    }
}

/// The inverse of how units are placed: a unit's node sits at its replicated `Position` as
/// its local position under the WorldUnits root (`world::spawn_unit`), so a world point
/// under that root is the `Position` `[x, y, z]` of its local coordinates.
pub(crate) fn position_from_world(units_root: Transform3D, point: Vector3) -> [f32; 3] {
    let local = units_root.affine_inverse() * point;
    [local.x, local.y, local.z]
}

fn optional_id(id: Option<u64>) -> Variant {
    id.map_or(Variant::nil(), |id| (id as i64).to_variant())
}

/// The visible terrain, WMO or doodad surface under `screen_point`.
fn ground_under(camera: &Gd<Camera3D>, screen_point: Vector2) -> Option<Vector3> {
    let mut space = camera.get_world_3d()?.get_direct_space_state()?;
    let origin = camera.project_ray_origin(screen_point);
    let end = origin + camera.project_ray_normal(screen_point) * camera.get_far();
    let mut query = PhysicsRayQueryParameters3D::create(origin, end)
        .expect("Godot could not allocate the ground ray parameters");
    query.set_collision_mask(TERRAIN_LAYER | WMO_LAYER | DOODAD_LAYER);
    let mut excluded = Array::new();
    loop {
        let hit = space.intersect_ray(&query);
        let collider = hit.get("collider")?.to::<Gd<CollisionObject3D>>();
        if collider.is_visible_in_tree() {
            return hit.get("position").map(|position| position.to::<Vector3>());
        }
        excluded.push(collider.get_rid());
        query.set_exclude(&excluded);
    }
}

impl GameClient {
    /// Per frame, before targeting: a Move To ground click must not also select a unit.
    pub(super) fn update_pet_bar(&mut self, delta: f32) -> Result<(), FrameError> {
        let Some(spells) = self.shown_pet_bar() else {
            self.pet_bar.close();
            return Ok(());
        };
        if self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            self.apply_pet_bar_keys(&spells)?;
            self.apply_pet_move_to_click()?;
        }
        self.poll_pet_bar_clicks(&spells)?;
        for pushed in &mut self.pet_bar.pushed {
            *pushed = (*pushed - delta).max(0.0);
        }
        Ok(self.sync_pet_bar(&spells, delta)?)
    }

    /// `PetHasActionBar() and UnitIsVisible("pet")`: the bar of a replicated pet, in world.
    fn shown_pet_bar(&self) -> Option<PetSpells> {
        if self.account.session.screen != SessionScreen::InWorld {
            return None;
        }
        let spells = self.account.pet_bar?;
        self.replica.unit(spells.pet).map(|_| spells)
    }

    fn pet_in_combat(&self, pet: u64) -> bool {
        self.replica
            .unit(pet)
            .and_then(|unit| unit.unit_flags())
            .is_some_and(|flags| flags & UnitFlags::PET_IN_COMBAT != 0)
    }

    fn apply_pet_bar_keys(&mut self, spells: &PetSpells) -> Result<(), SessionError> {
        let input = self.physical_input.gameplay_state(self.keyboard_free());
        let bindings = &self.client_options.bindings;
        let pressed: Vec<usize> = InputAction::PET_ACTION_SLOTS
            .iter()
            .enumerate()
            .filter(|(_, action)| bindings.is_just_pressed(**action, &input))
            .map(|(index, _)| index)
            .collect();
        for index in pressed {
            self.pet_bar.pushed[index] = PUSH_SECS;
            self.use_pet_action(spells, index)?;
        }
        Ok(())
    }

    fn poll_pet_bar_clicks(&mut self, spells: &PetSpells) -> Result<(), FrameError> {
        let Some(ui) = self.pet_bar.ui.as_mut() else {
            return Ok(());
        };
        let action = ui.bind_mut().pop_action().to_string();
        match parse_pet_action_button(&action) {
            Some(index) => Ok(self.use_pet_action(spells, index)?),
            None if action.is_empty() => Ok(()),
            None => Err(format!("Unknown pet bar action: {action}").into()),
        }
    }

    /// `CastPetAction(index)`: Move To starts ground targeting; Attack and spells act on
    /// the current target; Follow, Stay and the stances need none.
    fn use_pet_action(&mut self, spells: &PetSpells, index: usize) -> Result<(), SessionError> {
        let packed = spells.action_buttons[index];
        let target = match PetActionSlot::from_packed(packed) {
            PetActionSlot::Empty => return Ok(()),
            PetActionSlot::Command(COMMAND_MOVE_TO) => {
                self.pet_bar.move_to = Some((spells.pet, packed));
                return Ok(());
            }
            PetActionSlot::Command(COMMAND_ATTACK) | PetActionSlot::Spell(_) => {
                self.targeting_target()
            }
            PetActionSlot::Command(_) | PetActionSlot::Reaction(_) => None,
        };
        self.send_pet_action(PetAction {
            pet: spells.pet,
            action: packed,
            target,
            position: None,
        })
    }

    fn send_pet_action(&mut self, action: PetAction) -> Result<(), SessionError> {
        self.account.send_pet_action(action)?;
        self.pet_bar.sent.push(action);
        Ok(())
    }

    /// While Move To targets: a left click on the world ground sends the point, a right
    /// click cancels. Both clicks are consumed.
    fn apply_pet_move_to_click(&mut self) -> Result<(), SessionError> {
        let Some((pet, packed)) = self.pet_bar.move_to else {
            return Ok(());
        };
        if self
            .physical_input
            .take_mouse_press(BindingMouseButton::Right)
        {
            self.pet_bar.move_to = None;
            return Ok(());
        }
        if !self
            .physical_input
            .take_mouse_press(BindingMouseButton::Left)
        {
            return Ok(());
        }
        let pointer = Vector2::from_array(self.physical_input.pointer());
        let point = self
            .base()
            .get_viewport()
            .and_then(|viewport| viewport.get_camera_3d())
            .and_then(|camera| ground_under(&camera, pointer));
        let root = self.world.root().map(|root| root.get_global_transform());
        // A click on the sky keeps targeting, as Retail's ground reticle does.
        let (Some(point), Some(root)) = (point, root) else {
            return Ok(());
        };
        self.pet_bar.move_to = None;
        self.send_pet_action(PetAction {
            pet,
            action: packed,
            target: None,
            position: Some(position_from_world(root, point)),
        })
    }

    /// Escape ends Move To targeting before it closes windows or clears the target.
    pub(super) fn cancel_pet_move_to(&mut self) -> bool {
        self.pet_bar.move_to.take().is_some()
    }

    fn pet_bar_state(&mut self, spells: &PetSpells, delta: f32) -> PetActionBarState {
        let pet_in_combat = self.pet_in_combat(spells.pet);
        let attack = PetActionSlot::Command(COMMAND_ATTACK);
        let attacking = pet_in_combat
            && spells
                .action_buttons
                .iter()
                .any(|&packed| PetActionSlot::from_packed(packed) == attack);
        self.pet_bar.flash = attacking.then(|| self.pet_bar.flash.map_or(0.0, |t| t + delta));
        let hotkeys = InputAction::PET_ACTION_SLOTS.map(|action| {
            self.client_options
                .bindings
                .binding(action)
                .map(InputBinding::hotkey_text)
                .unwrap_or_default()
        });
        let catalog_icon = |spell_id: u32| {
            self.spells
                .catalog()
                .and_then(|data| data.get(spell_id))
                .map_or(0, |spell| spell.icon_fdid)
        };
        let flash = self.pet_bar.flash.is_some_and(attack_flash_shown);
        let mut buttons = pet_bar_buttons(spells, pet_in_combat, flash, catalog_icon, &hotkeys);
        for (index, button) in buttons.iter_mut().enumerate() {
            button.icon_fdid = self.drawable_fdid(button.icon_fdid);
            button.pushed = self.pet_bar.pushed[index] > 0.0;
        }
        if let Some(index) = self
            .pet_bar
            .ui
            .as_ref()
            .and_then(|ui| ui.bind().hovered_button())
            .and_then(|(name, _)| {
                name.strip_prefix("PetActionButton")?
                    .parse::<usize>()
                    .ok()?
                    .checked_sub(1)
            })
            && let Some(button) = buttons.get_mut(index)
        {
            button.hovered = true;
        }
        PetActionBarState {
            visible: true,
            buttons,
        }
    }

    fn sync_pet_bar(&mut self, spells: &PetSpells, delta: f32) -> Result<(), String> {
        let state = self.pet_bar_state(spells, delta);
        let shown = self.client_options.hud.show_action_bars;
        if let Some(ui) = self.pet_bar.ui.as_mut() {
            ui.set_visible(shown);
            return ui.bind_mut().set_state(state);
        }
        for fdid in PET_BAR_ART_FDIDS {
            self.drawable_fdid(fdid);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("PetActionBarUI");
        ui.set_layer(2);
        self.base_mut().add_child(&ui);
        if let Err(error) = ui.bind_mut().show_pet_action_bar(state) {
            ui.free();
            return Err(error);
        }
        ui.set_visible(shown);
        self.pet_bar.ui = Some(ui);
        Ok(())
    }

    /// Pet bar state for automation: the pet, its command and react states, the buttons'
    /// packed actions and checked states, Move To targeting and the actions sent.
    pub(super) fn pet_bar_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let spells = self.account.pet_bar;
        state.set("pet", &optional_id(spells.map(|spells| spells.pet)));
        state.set("local_pet", &optional_id(self.local_pet_id()));
        state.set("shown", self.pet_bar.ui.is_some());
        if let Some(spells) = spells {
            let pet_in_combat = self.pet_in_combat(spells.pet);
            state.set("command_state", i64::from(spells.command_state));
            state.set("react_state", i64::from(spells.react_state));
            let buttons: PackedInt64Array = spells
                .action_buttons
                .iter()
                .map(|&packed| i64::from(packed))
                .collect();
            state.set("buttons", &buttons);
            let checked: PackedInt64Array = spells
                .action_buttons
                .iter()
                .map(|&packed| {
                    let slot = PetActionSlot::from_packed(packed);
                    i64::from(pet_action_active(slot, &spells, pet_in_combat))
                })
                .collect();
            state.set("checked", &checked);
            state.set("pet_in_combat", pet_in_combat);
        }
        state.set("move_to_pending", self.pet_bar.move_to.is_some());
        let sent: Array<VarDictionary> = self
            .pet_bar
            .sent
            .iter()
            .map(|action| {
                let mut entry = VarDictionary::new();
                entry.set("pet", action.pet as i64);
                entry.set("action", i64::from(action.action));
                entry.set("target", &optional_id(action.target));
                let position = action.position.map_or(Variant::nil(), |[x, y, z]| {
                    Vector3::new(x, y, z).to_variant()
                });
                entry.set("position", &position);
                entry
            })
            .collect();
        state.set("sent", &sent);
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Placing a unit at the returned `Position` under the same root puts its node back on
    /// the clicked ground point, for a moved and turned root too.
    #[test]
    fn move_to_position_is_the_inverse_of_unit_placement() {
        let goldshire = Vector3::new(-9_464.0, 58.3, 62.5);
        let identity = Transform3D::IDENTITY;
        assert_eq!(
            position_from_world(identity, goldshire),
            [-9_464.0, 58.3, 62.5]
        );
        let root = Transform3D::new(
            Basis::from_axis_angle(Vector3::UP, 0.5),
            Vector3::new(12.0, -3.0, 40.0),
        );
        let [x, y, z] = position_from_world(root, goldshire);
        let placed = root * Vector3::new(x, y, z);
        assert!(placed.distance_to(goldshire) < 1e-3, "{placed:?}");
    }
}
