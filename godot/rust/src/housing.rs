//! Native Housing Dashboard lifecycle and authoritative account storage; no default grants.
use crate::replicated::UnitFields;
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_core::housing::HousingCatalog;
use game_engine_session::SessionScreen;
use game_engine_ui_housing::housing::{
    BulletinBoard, CollectionFilter, DashboardTab, HousingAuthority, HousingDashboard,
    HousingSurface, SEARCH_FIELD, STORAGE_SEARCH_FIELD,
};
use game_engine_ui_housing::housing_component::{HousingView, housing_screen};
use game_engine_ui_model::popup::{PopupOutcome, PopupResult, PopupSpec};
use godot::prelude::*;
use std::sync::Arc;
const PURCHASE_POPUP: &str = "HOUSING_BUY_PLOT";
const ATLAS_ELEMENTS: &str = "db2/active/UiTextureAtlasElement.csv";
const TEXTURE_KITS: &str = "db2/active/UiTextureKit.csv";

#[cfg(test)]
#[path = "housing_catalog_filter_tests.rs"]
mod catalog_filter_tests;

#[derive(Default)]
pub(crate) struct HousingHud {
    pub model: Option<HousingDashboard>,
    pub ui: Option<Gd<RegistryUi>>,
    pub authority: HousingAuthority,
    pub(super) decor: crate::housing_decor::HousingDecor,
    pub(super) catalog_preview: crate::housing_preview::CatalogPreview,
    pub(super) expert_input: crate::input::PhysicalInput,
    pub(super) mouse_pointer: Option<crate::housing_mouse::ExpertPointer>,
    pub(super) storage_resize: Option<crate::housing_storage::StorageResize>,
    pub(super) placed_list_drag: Option<crate::housing_placed_list::PlacedListDrag>,
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
            let mut catalog = HousingCatalog::load(&self.data_root.join("cache/housing.sqlite"))?;
            catalog.resolve_icons(&self.data_root.join(ATLAS_ELEMENTS))?;
            catalog.resolve_texture_kits(&self.data_root.join(TEXTURE_KITS))?;
            let mut model = HousingDashboard::new(Arc::new(catalog));
            model.maps = Some(Arc::new(
                game_engine_ui_model::ui_map_data::UiMapCatalog::load(
                    &self.data_root.join("db2/active"),
                )?,
            ));
            self.housing.model = Some(model);
        }
        Ok(())
    }

    fn mount_housing_dashboard(&mut self) -> Result<(), String> {
        let panel = game_engine_core::ui_layout_data::load_housing_storage_panel(
            &self.account.hud_layout_path()?,
        )?;
        if let Some(model) = &mut self.housing.model {
            model.storage_panel = panel;
            model.storage_transition = None;
        }
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
        let board = reply.state.bulletin_board.clone();
        let answered = reply.request_id == self.housing.next_request_id;
        let accepted = self.housing.authority.receive(reply);
        if answered && let Some(model) = &mut self.housing.model {
            model.pending = false;
        }
        if accepted.is_ok() && (opened.is_some() || board.is_some()) {
            self.ensure_housing_model()?;
            if let Some(model) = &mut self.housing.model {
                if let Some(plot) = opened {
                    model.surface = HousingSurface::Cornerstone(plot);
                } else if let Some(roster) = board {
                    model.surface = HousingSurface::BulletinBoard;
                    model.bulletin_board = Some(BulletinBoard::new(roster));
                }
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
        self.housing.catalog_preview.reset();
        self.housing.decor.clear_preview();
        self.housing.decor.highlight(None);
        self.housing.expert_input.clear();
        self.housing.mouse_pointer = None;
        self.housing.storage_resize = None;
        if let Some(model) = &mut self.housing.model {
            model.storage_transition = None;
            model.surface = HousingSurface::Dashboard;
            model.editor = Default::default();
            model.settings = None;
        }
        self.housing.decor.restore_saved();
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
        let mut model = self
            .housing
            .model
            .as_ref()
            .ok_or("Housing catalog is not loaded")?
            .clone();
        model.editor_bindings = self.client_options.bindings.clone();
        model.editor.mouse_control_anchor =
            self.project_housing_control_anchor(model.editor.expert_transform(), scale);
        let authority = &self.housing.authority;
        let missing_thumbnails = model
            .visible(authority)
            .into_iter()
            .chain(model.selected(authority))
            .filter_map(|decor| {
                let fdid = decor.thumbnail_fdid;
                (!self
                    .data_root
                    .join(format!("textures/{fdid}.blp"))
                    .is_file())
                .then_some(fdid)
            })
            .collect();
        let vendor_names = model
            .selected(authority)
            .into_iter()
            .flat_map(|decor| &decor.sources)
            .filter_map(|source| match source {
                game_engine_core::housing::DecorSource::Vendor { creature_id, .. } => {
                    let name = &self.tooltips.creature(*creature_id)?.name;
                    Some((*creature_id, name.clone()))
                }
                game_engine_core::housing::DecorSource::Encounter { .. } => None,
            })
            .collect();
        Ok(HousingView {
            model,
            authority: self.housing.authority.clone(),
            vendor_names,
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
        self.advance_housing_precision()?;
        let delta = self.base().get_process_delta_time() as f32;
        if let Some(model) = &mut self.housing.model {
            model.advance_storage_transition(delta)?;
        }
        self.advance_housing_storage_menu(delta);
        let Some(ui) = &mut self.housing.ui else {
            return Ok(());
        };
        let mut host = ui.bind_mut();
        let error = host.sync_input();
        if !error.is_empty() {
            return Err(error.to_string().into());
        }
        // A tab click changes the desired model before the registry mounts its new
        // controls. An absent search field is not a user's empty-string edit.
        if let Some(model) = &mut self.housing.model
            && model.tab == DashboardTab::Catalog
            && host.frame_control(SEARCH_FIELD).is_some()
        {
            model.search_focused = host
                .frame_control(SEARCH_FIELD)
                .is_some_and(|control| control.has_focus());
            let search = host.frame_text(SEARCH_FIELD.into()).to_string();
            if model.search != search {
                model.set_search(search, &self.housing.authority);
            }
        }
        if let Some(model) = &mut self.housing.model {
            let search_control = host.frame_control(STORAGE_SEARCH_FIELD);
            model.storage_search_focused = search_control
                .as_ref()
                .is_some_and(|control| control.has_focus());
            if model.surface == HousingSurface::Editor && search_control.is_some() {
                let search = host.frame_text(STORAGE_SEARCH_FIELD.into()).to_string();
                if model.storage_search != search {
                    model.set_storage_search(search);
                }
            }
        }
        drop(host);
        if let Some(model) = self.housing.model.as_mut() {
            model.reconcile_placed_list(&self.housing.authority);
        }
        self.query_housing_vendor_names()?;
        self.clear_inactive_housing_preview();
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
        self.configure_housing_storage_cursor();
        self.sync_housing_search_clear_visuals()?;
        self.sync_housing_catalog_preview()?;
        Ok(())
    }

    /// Retail resolves decor vendor names through the creature query; ask once per
    /// vendor of the previewed decor.
    fn query_housing_vendor_names(&mut self) -> Result<(), FrameError> {
        let Some(model) = &self.housing.model else {
            return Ok(());
        };
        if model.tab != DashboardTab::Catalog {
            return Ok(());
        }
        let Some(decor) = model.selected(&self.housing.authority) else {
            return Ok(());
        };
        let vendors: Vec<u32> = decor
            .sources
            .iter()
            .filter_map(|source| match source {
                game_engine_core::housing::DecorSource::Vendor { creature_id, .. } => {
                    Some(*creature_id)
                }
                game_engine_core::housing::DecorSource::Encounter { .. } => None,
            })
            .collect();
        for entry in vendors {
            if self.tooltips.creature(entry).is_none() && self.tooltips.take_unasked(entry) {
                self.account.send_creature_tooltip_query(entry)?;
            }
        }
        Ok(())
    }

    fn dispatch_housing_catalog_focus_action(&mut self, action: &str) -> Result<bool, String> {
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("Housing Catalog model unavailable.")?;
        let focus_before = (model.category, model.subcategory);
        let clearing = action == "housing:search:clear";
        match action {
            "housing:search:clear" => {
                model.set_search(String::new(), &self.housing.authority);
                model.search_focused = false;
            }
            "housing:category:back" => model.set_category(None, &self.housing.authority),
            "housing:subcategory:all" => model.set_subcategory(None, &self.housing.authority),
            value
                if value.starts_with("housing:category:")
                    || value.starts_with("housing:subcategory:") =>
            {
                apply_catalog_action(model, &self.housing.authority, value)?;
            }
            _ => return Ok(false),
        }
        let focus_changed = focus_before != (model.category, model.subcategory);
        if clearing || (focus_changed && model.category.is_some()) {
            self.clear_housing_search_input(SEARCH_FIELD)?;
        }
        Ok(true)
    }

    fn clear_housing_search_input(&mut self, field: &str) -> Result<(), String> {
        if let Some(ui) = &mut self.housing.ui {
            let mut host = ui.bind_mut();
            if host.frame_control(field).is_some() {
                host.clear_editbox_named(field)?;
                if let Some(mut control) = host.frame_control(field) {
                    control.release_focus();
                }
            }
        }
        Ok(())
    }

    fn dispatch_housing_storage_action(&mut self, action: &str) -> Result<bool, String> {
        if self.dispatch_housing_drawer_action(action)? {
            return Ok(true);
        }
        let model = self
            .housing
            .model
            .as_mut()
            .ok_or("Housing UI has no storage model.")?;
        if model.storage_filter_action(action)? {
            return Ok(true);
        }
        let focus_before = (model.storage_category, model.storage_subcategory);
        let clearing_search = action == "housing:storage:search:clear";
        if clearing_search {
            model.set_storage_search(String::new());
            model.storage_search_focused = false;
        } else if !model.storage_focus_action(action, &self.housing.authority)? {
            return Ok(false);
        }
        let focus_changed = focus_before != (model.storage_category, model.storage_subcategory);
        let clear_search = clearing_search || (focus_changed && model.storage_category.is_some());
        if clear_search {
            self.clear_housing_search_input(STORAGE_SEARCH_FIELD)?;
        }
        Ok(true)
    }

    pub(crate) fn housing_cursor_press(
        &mut self,
        owner: i64,
        action: &str,
        click: game_engine_ui_model::merchant::Click,
        at: Option<Vector2>,
    ) -> Result<bool, FrameError> {
        if !self
            .housing
            .ui
            .as_ref()
            .is_some_and(|ui| ui.instance_id().to_i64() == owner)
        {
            return Ok(false);
        }
        if click.right && action.starts_with("housing:placed-list:select:") {
            self.open_housing_placed_list_context(
                action,
                at.ok_or("Placed-decor context click has no pointer coordinates.")?,
            )?;
            return Ok(true);
        }
        if action == "housing:close" {
            self.close_housing();
            return Ok(true);
        }
        if action == "housing:purchase" {
            self.confirm_housing_purchase()?;
            return Ok(true);
        }
        if self.dispatch_housing_storage_action(action)? {
            return Ok(true);
        }
        if self.dispatch_housing_catalog_focus_action(action)? {
            return Ok(true);
        }
        if self.dispatch_housing_editor_action(action)? {
            return Ok(true);
        }
        let HousingHud {
            model, authority, ..
        } = &mut self.housing;
        let model = model.as_mut().ok_or("Housing UI has no catalog model")?;
        if model.pending {
            return Ok(true);
        }
        if action.starts_with("housing:settings:") {
            if let Some(request) = model.settings_request(authority, action)? {
                self.request_housing(request)?;
                self.close_housing();
            }
            return Ok(true);
        }
        if let Some(value) = action.strip_prefix("housing:finder-plot:") {
            let id = value.parse::<u32>().map_err(|_| "Invalid plot identity.")?;
            model.select_finder_plot(authority, id)?;
            return Ok(true);
        }
        if let Some(request) = model.navigation_request(action)? {
            self.request_housing(request)?;
            return Ok(true);
        }
        match action {
            // navigation_request applied this local sort without sending a request.
            action if action.starts_with("housing:roster-sort:") => {}
            "housing:finder-back" => {}
            "housing:info" => model.tab = DashboardTab::HouseInfo,
            "housing:catalog" => model.tab = DashboardTab::Catalog,
            "housing:collection" => model.tab = DashboardTab::Collection,
            "housing:filters" => model.filters_open = !model.filters_open,
            "housing:filter:dyeable" => model.toggle_dyeable(authority),
            "housing:filter:collection:all" => {
                model.set_collection(CollectionFilter::All, authority)
            }
            "housing:filter:collection:collected" => {
                model.set_collection(CollectionFilter::Collected, authority)
            }
            "housing:filter:collection:uncollected" => {
                model.set_collection(CollectionFilter::Uncollected, authority)
            }
            _ => apply_catalog_action(model, authority, action)?,
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
        let price = game_engine_ui_housing::housing_navigation_component::format_money(plot.cost);
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
        if self.housing_catalog_pointer(event) {
            self.mark_viewport_input_handled();
            return true;
        }
        match self.housing_editor_pointer(event) {
            Ok(true) => {
                self.mark_viewport_input_handled();
                return true;
            }
            Ok(false) => {}
            Err(error) => {
                self.mark_viewport_input_handled();
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
            model.scroll(rows, &self.housing.authority);
        }
        true
    }
}

fn apply_catalog_action(
    model: &mut HousingDashboard,
    authority: &HousingAuthority,
    action: &str,
) -> Result<(), String> {
    if action == "housing:filter:reset" {
        model.reset_catalog_filters(authority);
        return Ok(());
    }
    let (kind, id) = action
        .rsplit_once(':')
        .ok_or_else(|| format!("Invalid housing action: {action}"))?;
    let id = id
        .parse::<u32>()
        .map_err(|_| format!("Invalid housing ID: {action}"))?;
    match kind {
        // Retail OnCategoryClicked focuses; BACK or the All category returns to the top level.
        "housing:category" => model.set_category(Some(id), authority),
        "housing:subcategory" => model.set_subcategory(Some(id), authority),
        "housing:select" => {
            if !model.select(id, authority) {
                return Err(format!("Decor {id} is not in the filtered housing catalog"));
            }
        }
        _ => return Err(format!("Unknown housing action: {action}")),
    }
    Ok(())
}
