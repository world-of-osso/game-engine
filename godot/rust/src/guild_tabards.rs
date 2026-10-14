//! GuildTabardVendor canvas and real local-player draft preview.
use crate::{
    GameClient,
    account::NpcMessage,
    character_frame::preview::{Scene, bind_sheet_light},
    frame_error::FrameError,
    ui::RegistryUi,
    world_models::UnitAppearance,
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::guild_tabards::{GuildTabards as View, calculate_rotation};
use godot::prelude::*;
use shared::{
    components::{
        EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Player, SheathState,
    },
    protocol::{GuildTabardReply, InteractionKind, NpcRole},
};

#[derive(Default)]
pub(crate) struct GuildTabards {
    view: View,
    pub(crate) ui: Option<Gd<RegistryUi>>,
    scene: Option<Scene>,
    shown: Option<UnitAppearance>,
    pending: Option<(u64, UnitAppearance)>,
    yaw: f32,
}
impl GuildTabards {
    pub(crate) fn portrait_unit(&self) -> Option<u64> {
        self.view.vendor.as_ref().map(|vendor| vendor.npc)
    }
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }
    fn free_ui(&mut self) {
        if let Some(scene) = self.scene.take() {
            scene.free();
        }
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.shown = None;
        self.yaw = 0.0;
    }
}

impl GameClient {
    pub(super) fn tabard_npc_message(&mut self, message: &NpcMessage) -> Result<bool, FrameError> {
        match message {
            NpcMessage::Opened(opened)
                if opened.kind == InteractionKind::Role(NpcRole::TabardDesigner) =>
            {
                self.hide_guild_charter_canvas();
                self.reset_guild_tabard();
                self.auction_interaction_closed(opened.npc);
                self.close_trainer();
                self.merchant.session.close();
                let request = self.guild_tabards.view.begin_open(opened.npc);
                self.account.send_guild_tabard(request)?;
                Ok(true)
            }
            NpcMessage::Opened(_) => {
                self.reset_guild_tabard();
                Ok(false)
            }
            NpcMessage::Closed(npc)
                if self
                    .guild_tabards
                    .portrait_unit()
                    .or(self.guild_tabards.view.opening)
                    == Some(*npc) =>
            {
                self.reset_guild_tabard();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(super) fn receive_guild_tabard(
        &mut self,
        reply: GuildTabardReply,
    ) -> Result<(), FrameError> {
        if let GuildTabardReply::Failed { message } = &reply {
            self.add_world_error(message)?;
        }
        self.guild_tabards.view.receive(reply)?;
        Ok(())
    }

    fn reset_guild_tabard(&mut self) {
        if let Some((id, _)) = self.guild_tabards.pending.take() {
            self.world.cancel_detached_visual(id);
        }
        self.guild_tabards.view = View::default();
        self.guild_tabards.free_ui();
    }

    pub(super) fn close_guild_tabard(&mut self) -> Result<bool, FrameError> {
        let npc = self
            .guild_tabards
            .portrait_unit()
            .or(self.guild_tabards.view.opening);
        if let Some(npc) = npc {
            self.account.send_close_interaction(npc)?;
        }
        self.reset_guild_tabard();
        Ok(npc.is_some())
    }

    pub(super) fn update_guild_tabards(&mut self, delta: f64) -> Result<(), FrameError> {
        if !matches!(
            self.account.session.screen,
            SessionScreen::InWorld | SessionScreen::Loading
        ) {
            self.reset_guild_tabard();
            return Ok(());
        }
        if self.account.session.screen != SessionScreen::InWorld
            || self.guild_tabards.view.vendor.is_none()
        {
            return Ok(());
        }
        self.guild_tabards.view.gold = self
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.gold());
        self.sync_guild_tabard_ui()?;
        self.dispatch_guild_tabard_actions()?;
        if self.guild_tabards.view.vendor.is_some() {
            self.sync_guild_tabard_preview(delta)?;
        }
        Ok(())
    }

    fn dispatch_guild_tabard_actions(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.guild_tabards.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if action == "tabard:close" {
                self.close_guild_tabard()?;
                break;
            }
            if let Some(request) = self.guild_tabards.view.click(&action)? {
                if let Err(error) = self.account.send_guild_tabard(request) {
                    self.guild_tabards.view.waiting = false;
                    return Err(error.into());
                }
            }
        }
        Ok(())
    }

    fn sync_guild_tabard_ui(&mut self) -> Result<(), String> {
        let state = self.guild_tabards.view.clone();
        let scale = self.effective_ui_scale();
        if let Some(ui) = &mut self.guild_tabards.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(state);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("GuildTabardUI");
        self.base_mut().add_child(&ui);
        let result = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_guild_tabard(state))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        self.guild_tabards.ui = Some(ui);
        Ok(())
    }

    fn guild_tabard_preview_appearance(&self) -> Option<UnitAppearance> {
        let unit = self.replica.unit(self.local_player_id()?)?;
        let player = unit.get::<Player>()?.clone();
        let mut equipment = unit.get::<EquipmentAppearance>()?.clone();
        let design = self.guild_tabards.view.draft?;
        equipment
            .entries
            .retain(|entry| entry.slot != EquipmentVisualSlot::Tabard);
        // TabardModel previews the design on a guild tabard; it does not grant/equip an owned item.
        equipment.entries.push(EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Tabard,
            item_id: Some(5976),
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
            display_info_id: None,
            inventory_type: 19,
            hidden: false,
        });
        Some(UnitAppearance::Player(player, equipment, Some(design)))
    }

    fn sync_guild_tabard_preview(&mut self, delta: f64) -> Result<(), String> {
        let Some(appearance) = self.guild_tabard_preview_appearance() else {
            return Ok(());
        };
        self.request_guild_tabard_preview(appearance);
        self.attach_guild_tabard_preview()?;
        let ui = self.guild_tabards.ui.as_ref().ok_or("Tabard UI missing")?;
        let left = held_button(ui, "TabardCharacterModelRotateLeftButton")?;
        let right = held_button(ui, "TabardCharacterModelRotateRightButton")?;
        self.guild_tabards.yaw =
            calculate_rotation(self.guild_tabards.yaw, delta as f32, left, right);
        if let Some(scene) = &mut self.guild_tabards.scene {
            scene.fit_viewport();
            if let Some(mut model) = scene.model() {
                model.set_rotation(Vector3::new(0.0, self.guild_tabards.yaw, 0.0));
            }
        }
        Ok(())
    }

    fn request_guild_tabard_preview(&mut self, appearance: UnitAppearance) {
        let current = self
            .guild_tabards
            .pending
            .as_ref()
            .map(|(_, appearance)| appearance)
            .or(self.guild_tabards.shown.as_ref());
        if current == Some(&appearance) {
            return;
        }
        if let Some((id, _)) = self.guild_tabards.pending.take() {
            self.world.cancel_detached_visual(id);
        }
        let id = self.world.request_detached_visual(&appearance);
        self.guild_tabards.pending = Some((id, appearance));
    }

    fn attach_guild_tabard_preview(&mut self) -> Result<(), String> {
        let Some((id, _)) = self.guild_tabards.pending else {
            return Ok(());
        };
        let Some(loaded) = self.world.take_detached_visual(id) else {
            return Ok(());
        };
        let (_, appearance) = self
            .guild_tabards
            .pending
            .take()
            .expect("pending tabard model");
        let model = loaded?;
        if let Err(error) =
            self.world
                .place_player_weapons(&model, &appearance, SheathState::Unarmed)
        {
            model.free();
            return Err(error);
        }
        bind_sheet_light(&model);
        if self.guild_tabards.scene.is_none() {
            let host = self
                .guild_tabards
                .ui
                .as_ref()
                .ok_or("Tabard UI missing")?
                .bind()
                .frame_control(game_engine_ui_model::guild_tabard_frame::MODEL)
                .ok_or("TabardModel control missing")?;
            self.guild_tabards.scene = Some(Scene::new(host));
        }
        self.guild_tabards
            .scene
            .as_mut()
            .expect("tabard scene created")
            .replace_model(model);
        self.guild_tabards.shown = Some(appearance);
        Ok(())
    }
}

fn held_button(ui: &Gd<RegistryUi>, name: &str) -> Result<bool, String> {
    let control = ui
        .bind()
        .frame_control(name)
        .ok_or_else(|| format!("Tabard rotate control{name} missing"))?;
    let button = control
        .try_cast::<godot::classes::Button>()
        .map_err(|_| format!("Tabard rotate control{name} is not a Button"))?;
    Ok(button.is_pressed())
}
