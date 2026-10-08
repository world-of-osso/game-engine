//! Owner death dialogs/grading and per-player ghost appearance.
use game_engine_ui_model::popup::PopupResult;
use godot::classes::{MeshInstance3D, Node3D};
use godot::prelude::*;
use shared::protocol::{DeathPositionSnapshot, DeathStateUpdate};
use shared::{
    components::{Player, UnitLevel},
    death::DeathState,
};

use crate::{GameClient, frame_error::SessionError, replicated::UnitFields};

const GHOST_TRANSPARENCY: f32 = 0.45;
const GHOST_META: &str = "deathstate_ghost";

impl GameClient {
    pub(crate) fn receive_death(&mut self, update: DeathStateUpdate) -> Result<(), String> {
        if let Some(error) = self.death_flow.receive(update) {
            self.add_world_error(&error)?;
        }
        Ok(())
    }

    /// Re-evaluate proximity every frame: the server doesn't publish range changes.
    pub(super) fn sync_death_popups(&mut self) {
        let position = self.death_player_position();
        let level = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
            .and_then(|unit| unit.get::<UnitLevel>())
            .map_or(0, |level| level.0);
        self.death_flow
            .sync_popups(&mut self.group_frames.popups, position.as_ref(), level);
        self.world_lighting
            .set_ghost_grading(self.death_flow.is_ghost());
        self.sync_ghost_visual();
    }

    pub(super) fn dispatch_death_popup_results(
        &mut self,
        results: &[PopupResult],
    ) -> Result<(), SessionError> {
        if let Some(request) = self.death_flow.popup_results(results) {
            self.account.send_death(request)?;
        }
        Ok(())
    }

    /// Right-click and InteractUnit share this path; only an actual spirit healer
    /// within server range can open the penalty confirmation.
    pub(super) fn interact_spirit_healer(&mut self, id: u64) -> bool {
        use shared::components::Position;
        use shared::protocol::NpcFlags;
        let Some(unit) = self.replica.unit(id) else {
            return false;
        };
        let flags = NpcFlags(unit.npc_flags().unwrap_or(0));
        if !flags.contains(NpcFlags::SPIRIT_HEALER) {
            return false;
        }
        let Some(healer) = unit.get::<Position>() else {
            return false;
        };
        let Some(player) = self.death_player_position() else {
            return false;
        };
        let distance_squared = (player.x - healer.x).powi(2)
            + (player.y - healer.y).powi(2)
            + (player.z - healer.z).powi(2);
        let range = shared::death::SPIRIT_HEALER_RANGE;
        if distance_squared > range * range {
            return false;
        }
        self.death_flow.request_spirit_healer(&player)
    }

    fn death_player_position(&self) -> Option<DeathPositionSnapshot> {
        let map_id = u16::try_from(self.world_map_id?).ok()?;
        let position = self.world.local_player_server_position()?;
        Some(DeathPositionSnapshot {
            map_id,
            x: position.x,
            y: position.y,
            z: position.z,
        })
    }

    /// Apply on snapshot change and on asynchronously replaced player visuals, without
    /// changing shared materials (which would ghost other units using the same model).
    fn sync_ghost_visual(&mut self) {
        let local = self.world.local_player_id();
        for unit in self.replica.units().filter(|unit| unit.has::<Player>()) {
            let Some(visual) = self.world.unit_visual(unit.server_id) else {
                continue;
            };
            let ghost = if local == Some(unit.server_id) {
                self.death_flow.is_ghost()
            } else {
                unit.death_state() == Some(DeathState::Ghost)
            };
            apply_ghost_meshes(visual, ghost);
        }
    }
}

pub(crate) fn ghost_transparency(ghost: bool) -> f32 {
    if ghost { GHOST_TRANSPARENCY } else { 0.0 }
}

fn apply_ghost_meshes(mut visual: Gd<Node3D>, ghost: bool) {
    let drawn_ghost = visual.has_meta(GHOST_META) && visual.get_meta(GHOST_META).to::<bool>();
    if drawn_ghost == ghost {
        return;
    }
    let transparency = ghost_transparency(ghost);
    let meshes = visual
        .find_children_ex("*")
        .type_("MeshInstance3D")
        .owned(false)
        .done();
    for node in meshes.iter_shared() {
        node.cast::<MeshInstance3D>().set_transparency(transparency);
    }
    visual.set_meta(GHOST_META, &ghost.to_variant());
}
