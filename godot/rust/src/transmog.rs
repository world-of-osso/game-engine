//! Native TransmogFrame, authoritative quotes, and isolated pending character preview.
mod collection;

use crate::{
    GameClient,
    character_frame::preview::{Scene, bind_sheet_light},
    frame_error::FrameError,
    ui::RegistryUi,
    world_models::UnitAppearance,
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::{
    transmog::TransmogBook,
    transmog_frame::{HEIGHT, MODEL_SCENE, TransmogView, WIDTH, transmog_screen},
};
use godot::prelude::*;
use shared::components::{Gold, SheathState};
use shared::protocol::{
    ApplyTransmog, EquipmentSlot, TransmogChoice, TransmogError, TransmogFailed, TransmogState,
};

// The frontal body camera looks along drawn blades. A three-quarter view exposes them.
const CHARACTER_VIEW_YAW: f32 = std::f32::consts::FRAC_PI_4;
const ROTATE_STEP: f32 = std::f32::consts::FRAC_PI_8;

fn transmog_sheath(sheathed: bool) -> SheathState {
    if sheathed {
        SheathState::Unarmed
    } else {
        SheathState::Melee
    }
}

#[derive(Default)]
pub(crate) struct Transmog {
    pub(crate) book: TransmogBook,
    ui: Option<Gd<RegistryUi>>,
    scene: Option<Scene>,
    shown: Option<UnitAppearance>,
    yaw: f32,
    sheathed: bool,
    pending: Option<(u64, UnitAppearance)>,
    collection: std::collections::BTreeMap<u32, collection::CollectionPreview>,
}

impl Transmog {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }
}

impl GameClient {
    pub(super) fn receive_transmog_state(&mut self, state: TransmogState) {
        self.merchant.session.close();
        self.auction_interaction_closed_any();
        self.transmog.book.receive(state);
    }

    pub(super) fn receive_transmog_failure(
        &mut self,
        failure: TransmogFailed,
    ) -> Result<(), FrameError> {
        let refresh = matches!(
            failure.error,
            TransmogError::ItemChanged | TransmogError::AppearanceNotUsable
        );
        self.transmog.book.receive_failure(failure);
        if refresh {
            self.refresh_transmog()?;
        }
        Ok(())
    }

    pub(super) fn refresh_transmog(&self) -> Result<(), FrameError> {
        if let Some(state) = &self.transmog.book.state {
            self.account.send_transmog_query(state.npc)?;
        }
        Ok(())
    }

    pub(super) fn transmog_closed_for(&mut self, npc: u64) {
        if self
            .transmog
            .book
            .state
            .as_ref()
            .is_some_and(|state| state.npc == npc)
        {
            self.close_transmog();
        }
    }

    fn close_transmog(&mut self) -> bool {
        let shown = self.transmog.book.state.is_some();
        self.clear_transmog_collection();
        if let Some((id, _)) = self.transmog.pending.take() {
            self.world.cancel_detached_visual(id);
        }
        if let Some(scene) = self.transmog.scene.take() {
            scene.free();
        }
        if let Some(ui) = self.transmog.ui.take() {
            ui.free();
        }
        self.transmog.shown = None;
        self.transmog.yaw = 0.0;
        self.transmog.sheathed = false;
        self.transmog.book.close();
        shown
    }

    pub(super) fn close_transmog_window(&mut self) -> Result<bool, FrameError> {
        if let Some(state) = &self.transmog.book.state {
            self.account.send_close_interaction(state.npc)?;
        }
        Ok(self.close_transmog())
    }

    pub(super) fn update_transmog(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_transmog();
            return Ok(());
        }
        if self.transmog.book.state.is_none() {
            return Ok(());
        }
        if let Some(gold) = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
            .and_then(|unit| unit.get::<Gold>())
        {
            self.transmog.book.money = gold.0;
        }
        self.poll_transmog_actions()?;
        if self.transmog.book.state.is_some() {
            self.sync_transmog()?;
            self.sync_transmog_model()?;
            self.sync_transmog_collection()?;
        }
        Ok(())
    }

    fn poll_transmog_actions(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.transmog.ui.clone() else {
            return Ok(());
        };
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if action == "transmog:close" {
                self.close_transmog_window()?;
                break;
            }
            if let Some(request) = self.apply_transmog_action(&action)? {
                if let Err(error) = self.account.send_transmog_apply(request) {
                    self.transmog.book.applying = false;
                    self.transmog.book.error = error.to_string();
                    return Err(error.into());
                }
            }
        }
        Ok(())
    }

    fn apply_transmog_action(&mut self, action: &str) -> Result<Option<ApplyTransmog>, String> {
        if self.apply_transmog_preview_action(action)? {
            return Ok(None);
        }
        let book = &mut self.transmog.book;
        match action {
            "transmog:apply" => return Ok(book.apply()),
            "transmog:undo" => book.undo(),
            "transmog:original" => {
                book.stage(TransmogChoice::Original);
            }
            "transmog:hide" => {
                book.stage(TransmogChoice::Hidden);
            }
            "transmog:previous" => book.page = book.page.saturating_sub(1),
            "transmog:next" => book.page = (book.page + 1).min(book.pages() - 1),
            _ => {
                if let Some(value) = action.strip_prefix("transmog:slot:") {
                    let id = value
                        .parse::<u8>()
                        .map_err(|error| format!("Invalid transmog slot: {error}"))?;
                    let slot = EquipmentSlot::ALL
                        .into_iter()
                        .find(|slot| slot.inv_slot_id() == id)
                        .ok_or_else(|| format!("Unknown transmog slot {id}"))?;
                    book.select(slot);
                } else if let Some(value) = action.strip_prefix("transmog:source:") {
                    let id = value
                        .parse::<u32>()
                        .map_err(|error| format!("Invalid transmog source: {error}"))?;
                    if !book.stage(TransmogChoice::Source(id)) {
                        book.error =
                            "This appearance is not available for the selected slot.".into();
                    }
                } else {
                    return Err(format!("Unknown transmog action {action}"));
                }
            }
        }
        Ok(None)
    }

    fn apply_transmog_preview_action(&mut self, action: &str) -> Result<bool, String> {
        match action {
            "transmog:rotate-left" => self.transmog.yaw -= ROTATE_STEP,
            "transmog:rotate-right" => self.transmog.yaw += ROTATE_STEP,
            "transmog:sheathe" => {
                self.transmog.sheathed = !self.transmog.sheathed;
                if let (Some(model), Some(appearance)) = (
                    self.transmog.scene.as_ref().and_then(Scene::model),
                    self.transmog.shown.as_ref(),
                ) {
                    self.world.place_player_weapons(
                        &model,
                        appearance,
                        transmog_sheath(self.transmog.sheathed),
                    )?;
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn sync_transmog(&mut self) -> Result<(), String> {
        let size = self
            .base()
            .get_viewport()
            .ok_or("Transmog viewport missing")?
            .get_visible_rect()
            .size;
        let fit = (size.x / WIDTH).min(size.y / (HEIGHT + 41.0));
        let scale = self.effective_ui_scale().min(fit);
        let view = TransmogView {
            book: self.transmog.book.clone(),
            viewport_width: size.x / scale,
            race_id: match self.world.local_player_appearance() {
                Some(UnitAppearance::Player(player, _)) => player.race,
                _ => 0,
            },
        };
        if let Some(ui) = &mut self.transmog.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(view);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("TransmogUI");
        self.base_mut().add_child(&ui);
        let result = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_quest_window(view, transmog_screen))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        self.transmog.ui = Some(ui);
        Ok(())
    }

    fn sync_transmog_model(&mut self) -> Result<(), String> {
        let Some(UnitAppearance::Player(player, equipment)) =
            self.world.local_player_appearance().cloned()
        else {
            return Ok(());
        };
        let appearance =
            UnitAppearance::Player(player, self.transmog.book.preview_equipment(&equipment));
        if self.transmog.scene.is_none() {
            let host = self
                .transmog
                .ui
                .as_ref()
                .ok_or("Transmog UI missing")?
                .bind()
                .frame_control(MODEL_SCENE)
                .ok_or("Transmog model control missing")?;
            self.transmog.scene = Some(Scene::new(host));
        }
        self.request_transmog_model(appearance);
        self.attach_transmog_model()?;
        let scene = self.transmog.scene.as_mut().expect("preview created");
        scene.fit_viewport();
        scene.set_model_yaw(CHARACTER_VIEW_YAW + self.transmog.yaw);
        Ok(())
    }

    fn request_transmog_model(&mut self, appearance: UnitAppearance) {
        let current = self
            .transmog
            .pending
            .as_ref()
            .map(|(_, appearance)| appearance)
            .or(self.transmog.shown.as_ref());
        if current == Some(&appearance) {
            return;
        }
        if let Some((id, _)) = self.transmog.pending.take() {
            self.world.cancel_detached_visual(id);
        }
        let id = self.world.request_detached_visual(&appearance);
        self.transmog.pending = Some((id, appearance));
    }

    fn attach_transmog_model(&mut self) -> Result<(), String> {
        let Some((id, _)) = self.transmog.pending else {
            return Ok(());
        };
        let Some(loaded) = self.world.take_detached_visual(id) else {
            return Ok(());
        };
        let (_, appearance) = self.transmog.pending.take().expect("preview pending");
        let model = loaded?;
        if let Err(error) = self.world.place_player_weapons(
            &model,
            &appearance,
            transmog_sheath(self.transmog.sheathed),
        ) {
            model.free();
            return Err(error);
        }
        bind_sheet_light(&model);
        self.transmog
            .scene
            .as_mut()
            .ok_or("Transmog preview scene missing")?
            .replace_model(model);
        self.transmog.shown = Some(appearance);
        Ok(())
    }
}
