//! Authoritative corpse loot, shared Retail LootFrame, cursor and golden glow.

use std::collections::HashSet;

use game_engine_core::input_bindings_data::InputState;
use game_engine_session::SessionScreen;
use game_engine_ui_model::loot_data::{LootState, auto_loot, loot_chat_text};
use game_engine_ui_model::loot_frame_data::{
    LootFrameAction, anchor_under_cursor, build_state, request_for_action,
};
use godot::classes::{MeshInstance3D, SphereMesh, StandardMaterial3D, base_material_3d};
use godot::prelude::*;

use crate::GameClient;
use crate::account::LootMessage;
use crate::frame_error::FrameError;
use crate::ui::RegistryUi;

const LOOT_UI: &str = "LootUI";
const SPARKLE: &str = "LootSparkle";
const LOOT_RANGE: f32 = 5.0;
const SPARKLE_RADIUS: f32 = 0.15;
const SPARKLE_HEIGHT: f32 = 0.5;
const SPARKLE_EMISSION: f32 = 3.0;

#[derive(Default)]
pub(crate) struct Loot {
    pub state: LootState,
    pub lootable: HashSet<u64>,
    ui: Option<Gd<RegistryUi>>,
    anchor_corpse: Option<u64>,
    anchor: [f32; 2],
}

impl Loot {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }

    pub(crate) fn reset(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        *self = Self::default();
    }
}

impl GameClient {
    pub(super) fn receive_loot_message(&mut self, message: LootMessage) -> Result<(), String> {
        match message {
            LootMessage::Lootable(update) => {
                // Match the original replication-map boundary: unknown corpses are ignored.
                if self.units.contains_key(&update.corpse) {
                    if update.lootable {
                        self.loot.lootable.insert(update.corpse);
                    } else {
                        self.loot.lootable.remove(&update.corpse);
                    }
                    self.sync_loot_sparkle(update.corpse);
                }
            }
            LootMessage::Opened(response) => self.loot.state.open(response),
            LootMessage::Removed(removed) => {
                if let Some(content) = self.loot.state.remove(removed.corpse, removed.slot) {
                    game_engine_ui_model::chat_frame::add_system_line(
                        &mut self.chat.model.log,
                        &loot_chat_text(&content),
                    );
                }
            }
            LootMessage::Closed(closed) => self.loot.state.close(closed.corpse),
            LootMessage::Failed(failed) => self.add_world_error(failed.error.message())?,
        }
        Ok(())
    }

    /// True means this is a lootable NPC corpse, including an out-of-range one.
    pub(super) fn send_corpse_loot(&self, id: u64) -> Result<bool, FrameError> {
        let is_lootable_npc = self.loot.lootable.contains(&id)
            && self.units.get(&id).is_some_and(|unit| unit.npc.is_some());
        if !is_lootable_npc {
            return Ok(false);
        }
        let distance = self
            .world
            .local_player_transform()
            .zip(self.world.unit_node(id))
            .map_or(f32::INFINITY, |(player, node)| {
                player.origin.distance_to(node.get_global_position())
            });
        if distance <= LOOT_RANGE {
            let shift = self.physical_input.gameplay_state(true).shift_held();
            let auto = auto_loot(self.client_options.hud.auto_loot, shift);
            self.account.send_loot_unit(id, auto)?;
        }
        Ok(true)
    }

    pub(super) fn update_loot(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.loot.reset();
            return Ok(());
        }
        if self.game_menu_ui.is_none() && self.account.session.gameplay_input_allowed() {
            self.send_loot_frame_actions()?;
        }
        self.sync_loot_anchor();
        Ok(self.sync_loot_ui()?)
    }

    fn send_loot_frame_actions(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.loot.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            self.send_loot_frame_action(&action)?;
        }
        while let Some((action, _, _)) = ui.bind_mut().pop_alt_click() {
            self.send_loot_frame_action(&action)?;
        }
        Ok(())
    }

    fn send_loot_frame_action(&self, action: &str) -> Result<(), FrameError> {
        let Some(corpse) = self.loot.state.corpse else {
            return Ok(());
        };
        match request_for_action(action) {
            Some(LootFrameAction::Release) => self.account.send_loot_release(corpse)?,
            Some(LootFrameAction::Take { slot }) => self.account.send_loot_slot(corpse, slot)?,
            None => {}
        }
        Ok(())
    }

    fn sync_loot_anchor(&mut self) {
        let corpse = self.loot.state.corpse;
        if self.loot.anchor_corpse == corpse {
            return;
        }
        let scale = self.effective_ui_scale();
        let viewport = self.base().get_viewport();
        let size = viewport
            .map(|viewport| viewport.get_visible_rect().size)
            .unwrap_or_default();
        let screen = [size.x / scale, size.y / scale];
        let [x, y] = self.physical_input.pointer();
        self.loot.anchor =
            anchor_under_cursor([x / scale, y / scale], screen, self.loot.state.slots.len());
        self.loot.anchor_corpse = corpse;
    }

    fn sync_loot_ui(&mut self) -> Result<(), String> {
        let state = build_state(&self.loot.state, self.loot.anchor);
        let scale = self.effective_ui_scale();
        if let Some(ui) = &mut self.loot.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(state);
        }
        if !self.loot.state.is_open() {
            return Ok(());
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(LOOT_UI);
        self.base_mut().add_child(&ui);
        let scaled = ui.bind_mut().set_ui_scale(scale);
        let shown = scaled.and_then(|()| ui.bind_mut().show_loot_frame(state));
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.loot.ui = Some(ui);
        Ok(())
    }

    fn sync_loot_sparkle(&self, corpse: u64) {
        let Some(mut unit) = self.world.unit_node(corpse) else {
            return;
        };
        let sparkle = unit.get_node_or_null(SPARKLE);
        match (self.loot.lootable.contains(&corpse), sparkle) {
            (true, None) => unit.add_child(&spawn_loot_sparkle()),
            (false, Some(sparkle)) => sparkle.free(),
            _ => {}
        }
    }
}

fn spawn_loot_sparkle() -> Gd<MeshInstance3D> {
    let mut sphere = SphereMesh::new_gd();
    sphere.set_radius(SPARKLE_RADIUS);
    sphere.set_height(SPARKLE_RADIUS * 2.0);
    let mut material = StandardMaterial3D::new_gd();
    material.set_albedo(Color::from_rgba(1.0, 0.85, 0.0, 0.6));
    material.set_transparency(base_material_3d::Transparency::ALPHA);
    material.set_shading_mode(base_material_3d::ShadingMode::UNSHADED);
    material.set_emission_enabled(true);
    material.set_emission(Color::from_rgb(1.0, 0.85, 0.0));
    material.set_emission_energy_multiplier(SPARKLE_EMISSION);
    let mut sparkle = MeshInstance3D::new_alloc();
    sparkle.set_name(SPARKLE);
    sparkle.set_mesh(&sphere);
    sparkle.set_material_override(&material);
    sparkle.set_position(Vector3::new(0.0, SPARKLE_HEIGHT, 0.0));
    sparkle
}
