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
    transmog::{TransmogBook, TransmogSend, WardrobeTab},
    transmog_frame::{HEIGHT, MODEL_SCENE, NAME_POPUP_EDITBOX, TransmogView, WIDTH},
};
use godot::prelude::*;
use shared::components::{Gold, SheathState};
use shared::protocol::{
    EquipmentSlot, TransmogChoice, TransmogError, TransmogFailed, TransmogState,
};

// The frontal body camera looks along drawn blades. A three-quarter view exposes them.
const CHARACTER_VIEW_YAW: f32 = std::f32::consts::FRAC_PI_4;
const ROTATE_STEP: f32 = std::f32::consts::FRAC_PI_8;

fn choice_id(choice: TransmogChoice) -> i64 {
    match choice {
        TransmogChoice::Original => 0,
        TransmogChoice::Hidden => -1,
        TransmogChoice::Source(id) => i64::from(id),
    }
}

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
    collection: std::collections::BTreeMap<String, collection::CollectionPreview>,
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
    pub(super) fn receive_transmog_state(
        &mut self,
        state: TransmogState,
    ) -> Result<(), FrameError> {
        self.merchant.session.close();
        self.auction_interaction_closed_any();
        self.transmog.book.receive(state);
        // CommitAndApplyAllPending: the outfit commit follows a confirmed paid Apply.
        if let Some(save) = self.transmog.book.take_queued_save() {
            self.send_transmog(TransmogSend::SaveOutfit(save))?;
        }
        Ok(())
    }

    fn send_transmog(&mut self, request: TransmogSend) -> Result<(), FrameError> {
        if let Err(error) = self.account.send_transmog(request) {
            let book = &mut self.transmog.book;
            book.applying = false;
            book.saving = None;
            book.queued_save = None;
            book.error = error.to_string();
            return Err(error.into());
        }
        Ok(())
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

    pub(super) fn transmog_snapshot(&self) -> VarDictionary {
        let book = &self.transmog.book;
        let mut state = VarDictionary::new();
        state.set("open", book.state.is_some());
        state.set("tab", format!("{:?}", book.tab).as_str());
        state.set("money", book.money as i64);
        state.set("pending", book.pending_count() as i64);
        state.set("busy", book.busy());
        state.set("applying", book.applying);
        state.set("saving", format!("{:?}", book.saving).as_str());
        state.set("queued_save", book.queued_save.is_some());
        state.set("error", book.error.as_str());
        state.set("viewed_outfit", book.viewed_outfit.map_or(-1, i64::from));
        let mut outfits = VarDictionary::new();
        let mut icons = VarDictionary::new();
        for row in book.outfit_rows() {
            outfits.set(i64::from(row.id), row.name.as_str());
            icons.set(i64::from(row.id), i64::from(row.icon_fdid));
        }
        state.set("outfits", &outfits);
        state.set("outfit_icons", &icons);
        let mut sets = VarDictionary::new();
        for (set, progress) in book.available_sets() {
            sets.set(
                i64::from(set.set_id),
                format!("{} {}/{}", set.name, progress.collected, progress.total).as_str(),
            );
        }
        state.set("sets", &sets);
        let mut custom = VarDictionary::new();
        for set in book.custom_sets() {
            custom.set(set.id as i64, set.name.as_str());
        }
        state.set("custom_sets", &custom);
        let mut illusions = VarDictionary::new();
        // Source ID, 0 for the equipped item's own appearance, -1 for hidden.
        let mut applied = VarDictionary::new();
        let mut effective = VarDictionary::new();
        if let Some(state) = &book.state {
            for slot in &state.slots {
                let inv = i64::from(slot.slot.inv_slot_id());
                if let Some(id) = book.effective_illusion(slot.slot) {
                    illusions.set(inv, i64::from(id));
                }
                applied.set(inv, choice_id(slot.applied));
                effective.set(inv, choice_id(book.effective_choice(slot)));
            }
        }
        state.set("illusions", &illusions);
        state.set("applied", &applied);
        state.set("effective", &effective);
        state
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
        let search = ui
            .bind_mut()
            .frame_text(game_engine_ui_model::transmog_frame::SEARCH.into())
            .to_string();
        if search != self.transmog.book.search {
            self.transmog.book.set_search(search);
        }
        if let Some(draft) = &mut self.transmog.book.outfit_info {
            draft.name = ui
                .bind_mut()
                .frame_text(game_engine_ui_model::transmog_frame::OUTFIT_ICON_EDITBOX.into())
                .to_string();
        }
        while let Some((action, right, _)) = ui.bind_mut().pop_alt_click() {
            if right && let Some(id) = action.strip_prefix("transmog:outfit:") {
                self.transmog.book.outfit_menu = Some(
                    id.parse()
                        .map_err(|error| format!("Invalid outfit menu: {error}"))?,
                );
            }
        }
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
                self.send_transmog(request)?;
            }
        }
        Ok(())
    }

    fn apply_transmog_action(&mut self, action: &str) -> Result<Option<TransmogSend>, String> {
        if self.apply_transmog_preview_action(action)? {
            return Ok(None);
        }
        if action == "transmog:name-accept" {
            let mut ui = self.transmog.ui.clone().ok_or("Transmog UI missing")?;
            let name = ui
                .bind_mut()
                .frame_text(NAME_POPUP_EDITBOX.into())
                .to_string();
            return Ok(self.transmog.book.create_custom_set(&name));
        }
        let book = &mut self.transmog.book;
        match action {
            "transmog:save-outfit" => return Ok(book.save_outfit()),
            "transmog:icon-okay" => return Ok(book.commit_outfit_info()),
            "transmog:icon-cancel" => book.outfit_info = None,
            "transmog:icon-previous" => {
                if let Some(draft) = &mut book.outfit_info {
                    draft.page = draft.page.saturating_sub(1);
                }
            }
            "transmog:icon-next" => {
                let pages = book.outfit_icons().len().div_ceil(80);
                if let Some(draft) = &mut book.outfit_info {
                    draft.page = (draft.page + 1).min(pages.saturating_sub(1));
                }
            }
            "transmog:undo" => book.undo(),
            "transmog:clear-search" => book.set_search(String::new()),
            "transmog:filters" => book.filters_open = !book.filters_open,
            "transmog:filter:all" => book.set_all_source_filters(true),
            "transmog:filter:none" => book.set_all_source_filters(false),
            "transmog:filter:collected" => book.toggle_collected_sets(),
            "transmog:filter:uncollected" => book.toggle_uncollected_sets(),
            "transmog:original" if book.illusion_selected => {
                book.stage_illusion(None);
            }
            "transmog:original" => {
                book.stage(TransmogChoice::Original);
            }
            "transmog:hide" => {
                book.stage(TransmogChoice::Hidden);
            }
            "transmog:previous" => book.page = book.page.saturating_sub(1),
            "transmog:next" => book.page = (book.page + 1).min(book.pages() - 1),
            "transmog:tab:items" => book.set_tab(WardrobeTab::Items),
            "transmog:tab:sets" => book.set_tab(WardrobeTab::Sets),
            "transmog:tab:custom-sets" => book.set_tab(WardrobeTab::CustomSets),
            "transmog:new-custom-set" => book.open_custom_set_name(),
            "transmog:name-cancel" => book.naming_custom_set = false,
            _ => apply_transmog_id_action(book, action)?,
        }
        Ok(None)
    }

    fn apply_transmog_preview_action(&mut self, action: &str) -> Result<bool, String> {
        match action {
            "transmog:rotate-left" => self.transmog.yaw -= ROTATE_STEP,
            "transmog:rotate-right" => self.transmog.yaw += ROTATE_STEP,
            "transmog:reset-rotation" => self.transmog.yaw = 0.0,
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
            sheathed: self.transmog.sheathed,
            race_id: match self.world.local_player_appearance() {
                Some(UnitAppearance::Player(player, _, _)) => player.race,
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
                .and_then(|()| host.show_transmog(view))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        self.transmog.ui = Some(ui);
        Ok(())
    }

    fn sync_transmog_model(&mut self) -> Result<(), String> {
        let Some(UnitAppearance::Player(player, equipment, tabard)) =
            self.world.local_player_appearance().cloned()
        else {
            return Ok(());
        };
        let appearance = UnitAppearance::Player(
            player,
            self.transmog.book.preview_equipment(&equipment),
            tabard,
        );
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

/// Actions carrying an equipment slot, appearance, illusion, outfit or set identity.
fn apply_transmog_id_action(book: &mut TransmogBook, action: &str) -> Result<(), String> {
    fn id<T: std::str::FromStr>(value: &str, what: &str) -> Result<T, String>
    where
        T::Err: std::fmt::Display,
    {
        value
            .parse::<T>()
            .map_err(|error| format!("Invalid transmog {what}: {error}"))
    }
    fn slot(value: &str) -> Result<EquipmentSlot, String> {
        let inv = id::<u8>(value, "slot")?;
        EquipmentSlot::ALL
            .into_iter()
            .find(|slot| slot.inv_slot_id() == inv)
            .ok_or_else(|| format!("Unknown transmog slot {inv}"))
    }
    let (kind, value) = action
        .strip_prefix("transmog:")
        .and_then(|rest| rest.rsplit_once(':'))
        .ok_or_else(|| format!("Unknown transmog action {action}"))?;
    let staged = match kind {
        "edit-outfit" => book.edit_outfit_info(id(value, "outfit")?),
        "outfit-icon" => book.select_outfit_icon(id(value, "icon")?),
        "slot" => {
            book.select(slot(value)?);
            true
        }
        "illusion-slot" => {
            book.select_illusion(slot(value)?);
            true
        }
        "filter-source" => book.toggle_source_filter(id(value, "source filter")?),
        "source" => book.stage(TransmogChoice::Source(id(value, "source")?)),
        "illusion" => book.stage_illusion(Some(id(value, "illusion")?)),
        "outfit" => book.view_outfit(id(value, "outfit")?),
        "set" => book.use_set(id(value, "set")?),
        "custom-set" => book.use_custom_set(id(value, "custom set")?),
        _ => return Err(format!("Unknown transmog action {action}")),
    };
    if !staged {
        book.error = "This appearance is not available for the selected slot.".into();
    }
    Ok(())
}
