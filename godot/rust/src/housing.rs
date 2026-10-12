//! Native Housing Dashboard lifecycle and authoritative account storage; no default grants.
use crate::replicated::UnitFields;
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_core::housing::HousingCatalog;
use game_engine_session::SessionScreen;
use game_engine_ui_model::housing::{
    DashboardTab, HousingAuthority, HousingDashboard, HousingSurface, SEARCH_FIELD,
};
use game_engine_ui_model::housing_component::{HousingView, housing_screen};
use game_engine_ui_model::popup::{PopupOutcome, PopupResult, PopupSpec};
use godot::prelude::*;
use std::sync::Arc;
const PURCHASE_POPUP: &str = "HOUSING_BUY_PLOT";

#[derive(Default)]
pub(crate) struct HousingHud {
    pub model: Option<HousingDashboard>,
    pub ui: Option<Gd<RegistryUi>>,
    pub authority: HousingAuthority,
    pub(super) decor: crate::housing_decor::HousingDecor,
    next_request_id: u64,
    confirming_plot: Option<u32>,
}

impl GameClient {
    pub(super) fn toggle_housing(&mut self) -> Result<(), String> {
        if self.housing.ui.is_some() {
            self.close_housing();
            return Ok(());
        }
        self.ensure_housing_model()?;
        if let Some(model) = &mut self.housing.model {
            model.surface = HousingSurface::Dashboard;
        }
        self.mount_housing_dashboard()?;
        let neighborhood_id = self
            .housing
            .model
            .as_ref()
            .ok_or("Housing model is unavailable.")?
            .neighborhood;
        // Browsing an authored neighborhood is not an ownership grant or entry.
        self.request_housing(shared::protocol::HousingCommand::ListPlots { neighborhood_id })
    }

    pub(super) fn ensure_housing_model(&mut self) -> Result<(), String> {
        if self.housing.model.is_none() {
            let catalog = HousingCatalog::load(&self.data_root.join("cache/housing.sqlite"))?;
            self.housing.model = Some(HousingDashboard::new(Arc::new(catalog)));
        }
        Ok(())
    }

    fn mount_housing_dashboard(&mut self) -> Result<(), String> {
        let view = self.housing_view()?;
        self.cache_housing_textures(&view);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("HousingDashboardUI");
        ui.set_layer(6);
        self.base_mut().add_child(&ui);
        let mounted = ui.bind_mut().show_housing(view);
        if let Err(error) = mounted {
            ui.free();
            return Err(error);
        }
        self.housing.ui = Some(ui);
        Ok(())
    }

    pub(super) fn receive_housing_reply(
        &mut self,
        reply: shared::protocol::HousingReply,
    ) -> Result<(), String> {
        let opened = reply.state.opened_cornerstone;
        let answered = reply.request_id == self.housing.next_request_id;
        let accepted = self.housing.authority.receive(reply);
        if answered && let Some(model) = &mut self.housing.model {
            model.pending = false;
        }
        if accepted.is_ok()
            && let Some(plot) = opened
        {
            self.ensure_housing_model()?;
            if let Some(model) = &mut self.housing.model {
                model.surface = HousingSurface::Cornerstone(plot);
            }
            if self.housing.ui.is_none() {
                self.mount_housing_dashboard()?;
            }
        }
        let message = match accepted {
            Ok(()) => self.housing.authority.error.clone(),
            Err(message) => Some(message),
        };
        if let Some(message) = message {
            self.add_world_error(&message)?;
        }
        Ok(())
    }

    pub(super) fn request_housing(
        &mut self,
        command: shared::protocol::HousingCommand,
    ) -> Result<(), String> {
        let request_id = self
            .housing
            .next_request_id
            .checked_add(1)
            .ok_or("Housing request counter exhausted.")?;
        self.account
            .send_housing(shared::protocol::HousingRequest {
                request_id,
                command,
            })
            .map_err(|error| error.to_string())?;
        self.housing.next_request_id = request_id;
        if let Some(model) = &mut self.housing.model {
            model.pending = true;
        }
        Ok(())
    }

    pub(super) fn close_housing(&mut self) {
        self.housing.decor.clear_preview();
        self.group_frames.popups.hide(PURCHASE_POPUP);
        self.housing.confirming_plot = None;
        if let Some(ui) = self.housing.ui.take() {
            ui.free();
        }
    }

    fn housing_view(&self) -> Result<HousingView, String> {
        let size = self
            .base()
            .get_viewport()
            .ok_or("Housing has no viewport")?
            .get_visible_rect()
            .size;
        let scale = self.effective_ui_scale();
        let model = self
            .housing
            .model
            .as_ref()
            .ok_or("Housing catalog is not loaded")?
            .clone();
        let missing_thumbnails = model
            .visible()
            .into_iter()
            .chain(model.selected())
            .filter_map(|decor| {
                let fdid = decor.thumbnail_fdid;
                (!self
                    .data_root
                    .join(format!("textures/{fdid}.blp"))
                    .is_file())
                .then_some(fdid)
            })
            .collect();
        Ok(HousingView {
            model,
            authority: self.housing.authority.clone(),
            viewport: [size.x / scale, size.y / scale],
            money: self
                .world
                .local_player_id()
                .and_then(|id| self.replica.unit(id)?.gold()),
            missing_thumbnails,
        })
    }

    fn cache_housing_textures(&mut self, view: &HousingView) {
        for fdid in crate::quests::screen_texture_fdids(view.clone(), housing_screen) {
            self.drawable_fdid(fdid);
        }
    }

    pub(super) fn update_housing(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_housing();
            self.housing.authority.clear();
            self.housing.decor.clear();
            return Ok(());
        }
        self.sync_housing_layout()?;
        let Some(ui) = &mut self.housing.ui else {
            return Ok(());
        };
        let mut host = ui.bind_mut();
        let error = host.sync_input();
        if !error.is_empty() {
            return Err(error.to_string().into());
        }
        if let Some(model) = &mut self.housing.model
            && model.tab == DashboardTab::Catalog
        {
            let search = host.frame_text(SEARCH_FIELD.into()).to_string();
            if model.search != search {
                model.set_search(search);
            }
        }
        drop(host);
        let view = self.housing_view()?;
        self.cache_housing_textures(&view);
        let scale = self.effective_ui_scale();
        let ui = self
            .housing
            .ui
            .as_mut()
            .ok_or("Housing UI disappeared during update")?;
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(view)?;
        Ok(())
    }

    pub(crate) fn housing_cursor_press(
        &mut self,
        owner: i64,
        action: &str,
    ) -> Result<bool, FrameError> {
        if !self
            .housing
            .ui
            .as_ref()
            .is_some_and(|ui| ui.instance_id().to_i64() == owner)
        {
            return Ok(false);
        }
        if action == "housing:close" {
            self.close_housing();
            return Ok(true);
        }
        if action == "housing:purchase" {
            self.confirm_housing_purchase()?;
            return Ok(true);
        }
        if self.dispatch_housing_editor_action(action)? {
            return Ok(true);
        }
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("Housing UI has no catalog model")?;
        if model.pending {
            return Ok(true);
        }
        if let Some(request) = model.navigation_request(action)? {
            self.request_housing(request)?;
            return Ok(true);
        }
        match action {
            "housing:info" => model.tab = DashboardTab::HouseInfo,
            "housing:catalog" => model.tab = DashboardTab::Catalog,
            "housing:collection" => model.tab = DashboardTab::Collection,
            "housing:filters" => model.filters_open = !model.filters_open,
            "housing:subcategory:all" => {
                model.set_subcategory(None);
                model.filters_open = false;
            }
            _ => apply_catalog_action(model, action)?,
        }
        Ok(true)
    }

    fn confirm_housing_purchase(&mut self) -> Result<(), String> {
        let Some(model) = &self.housing.model else {
            return Err("Housing model is unavailable.".into());
        };
        if model.pending {
            return Err("Finish the current housing request first.".into());
        }
        let HousingSurface::Cornerstone(plot_id) = model.surface else {
            return Err("Interact with a cornerstone before purchasing.".into());
        };
        let plot = self
            .housing
            .authority
            .state
            .as_ref()
            .and_then(|state| state.plots.iter().find(|plot| plot.plot_id == plot_id))
            .ok_or("This plot has not been confirmed by the server.")?;
        if plot.house_id.is_some() {
            return Err("This plot is already owned.".into());
        }
        let money = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.gold());
        if !money.is_some_and(|money| money >= plot.cost) {
            return Err("Money is unconfirmed or insufficient for this plot.".into());
        }
        let price = game_engine_ui_model::housing_navigation_component::format_money(plot.cost);
        self.housing.confirming_plot = Some(plot_id);
        self.group_frames.popups.push(PopupSpec {
            key: PURCHASE_POPUP.into(),
            text: format!("Purchase Plot {} for {price}?", plot.plot_index),
            accept_label: "Buy".into(),
            cancel_label: Some("Cancel".into()),
            timeout: None,
            confirm_text: None,
        });
        self.physical_input.clear_gameplay();
        Ok(())
    }

    pub(super) fn dispatch_housing_popup_results(
        &mut self,
        results: &[PopupResult],
    ) -> Result<(), FrameError> {
        for result in results.iter().filter(|result| result.key == PURCHASE_POPUP) {
            let Some(plot_id) = self.housing.confirming_plot.take() else {
                continue;
            };
            if result.outcome == PopupOutcome::Accepted {
                self.request_housing(shared::protocol::HousingCommand::PurchasePlot { plot_id })?;
            }
        }
        Ok(())
    }

    pub(super) fn housing_pointer(&mut self, event: &Gd<godot::classes::InputEvent>) -> bool {
        match self.housing_editor_pointer(event) {
            Ok(true) => return true,
            Ok(false) => {}
            Err(error) => {
                godot_error!("House editor: {error}");
                if let Err(message) = self.add_world_error(&error) {
                    godot_error!("House editor feedback: {message}");
                }
                return true;
            }
        }
        let Ok(button) = event
            .clone()
            .try_cast::<godot::classes::InputEventMouseButton>()
        else {
            return false;
        };
        let rows = match button.get_button_index() {
            godot::global::MouseButton::WHEEL_UP => -1,
            godot::global::MouseButton::WHEEL_DOWN => 1,
            _ => return false,
        };
        let Some(rect) = self.housing.ui.as_ref().and_then(|ui| {
            ui.bind()
                .frame_viewport_rect("HousingCatalogOptionsContainer")
        }) else {
            return false;
        };
        let bounds = Rect2::new(
            Vector2::new(rect[0], rect[1]),
            Vector2::new(rect[2], rect[3]),
        );
        if !bounds.contains_point(button.get_position()) {
            return false;
        }
        if button.is_pressed()
            && let Some(model) = &mut self.housing.model
        {
            model.scroll(rows);
        }
        true
    }
}

fn apply_catalog_action(model: &mut HousingDashboard, action: &str) -> Result<(), String> {
    let (kind, id) = action
        .rsplit_once(':')
        .ok_or_else(|| format!("Invalid housing action: {action}"))?;
    let id = id
        .parse::<u32>()
        .map_err(|_| format!("Invalid housing ID: {action}"))?;
    match kind {
        "housing:category" => model.set_category(if model.category == Some(id) {
            None
        } else {
            Some(id)
        }),
        "housing:subcategory" => {
            model.set_subcategory(Some(id));
            model.filters_open = false;
        }
        "housing:select" => {
            if !model.select(id) {
                return Err(format!("Decor {id} is not in the filtered housing catalog"));
            }
        }
        _ => return Err(format!("Unknown housing action: {action}")),
    }
    Ok(())
}
