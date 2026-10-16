//! Native owner professions, with DB2 catalogs and the existing ProfessionChannel.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_core::input_bindings_data::InputAction;
use game_engine_session::SessionScreen;
use game_engine_ui_model::item_catalog::item_catalog;
use game_engine_ui_model::professions::ProfessionBook;
use game_engine_ui_model::professions_catalog::RecipeCatalog;
use game_engine_ui_model::professions_frame::{
    ItemDisplay, ProfessionPage, ProfessionView, QUANTITY, SEARCH, professions_screen,
};
use godot::prelude::*;
use shared::casting::CastState;
use shared::protocol::ProfessionSnapshot;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{self, Receiver, TryRecvError};

#[path = "professions_reagents.rs"]
mod reagents;

#[derive(Default)]
pub(crate) struct Professions {
    pub(crate) book: ProfessionBook,
    catalog: Option<RecipeCatalog>,
    loading: Option<Receiver<Result<RecipeCatalog, String>>>,
    started: bool,
    pub(crate) ui: Option<Gd<RegistryUi>>,
    pub(crate) error: String,
    pub(crate) reagent_selection:
        game_engine_ui_model::professions_reagent_selection::ReagentSelection,
    collapsed: BTreeSet<String>,
    page: ProfessionPage,
    pub(crate) specializations:
        game_engine_ui_model::professions_specializations::SpecializationDraft,
    pub(crate) orders: game_engine_ui_model::professions_orders::CraftingOrders,
    order_search: String,
    order_form_initialized: bool,
}
impl Professions {
    pub(crate) fn recipe(
        &self,
        spell_id: u32,
    ) -> Option<&game_engine_ui_model::professions::Recipe> {
        self.catalog.as_ref()?.get(spell_id)
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
    pub(crate) fn primary_count(&self) -> usize {
        self.catalog.as_ref().map_or(0, |catalog| {
            self.book
                .snapshot
                .lines
                .iter()
                .filter(|line| catalog.primary_skills.contains(&line.skill_line))
                .count()
        })
    }
    pub(crate) fn skill_names(&self) -> BTreeMap<u32, String> {
        self.catalog
            .as_ref()
            .map(|catalog| catalog.skill_names.clone())
            .unwrap_or_default()
    }
    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
    }
    fn poll_catalog(&mut self, data_root: &std::path::Path) -> Result<(), String> {
        if !self.started {
            self.started = true;
            let path = data_root.join("db2/active");
            let (sender, receiver) = mpsc::channel();
            game_engine_core::worker_shutdown::track(std::thread::spawn(move || {
                let _ = sender.send(RecipeCatalog::load(&path));
            }));
            self.loading = Some(receiver);
        }
        let Some(receiver) = &self.loading else {
            return Ok(());
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.loading = None;
                self.catalog = Some(result?);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.loading = None;
                return Err("Profession DB2 loader disconnected".into());
            }
        }
        Ok(())
    }
}

impl GameClient {
    pub(super) fn receive_professions(&mut self, snapshot: ProfessionSnapshot) {
        self.professions.book.snapshot = snapshot;
    }
    pub(super) fn close_professions(&mut self) -> bool {
        let visible = self.professions.book.visible;
        self.professions.book.visible = false;
        self.professions.page = ProfessionPage::Recipes;
        self.professions.order_form_initialized = false;
        self.professions.orders.menu = None;
        self.professions.orders.npc = None;
        self.professions.orders.customer = false;
        self.professions.specializations.confirming_unlock = false;
        self.professions.specializations.purchases.clear();
        self.professions.free_ui();
        visible
    }
    pub(super) fn close_professions_window(&mut self) -> Result<bool, FrameError> {
        if let Some(npc) = self.professions.orders.npc {
            self.account.send_close_interaction(npc)?;
        }
        Ok(self.close_professions())
    }
    pub(super) fn open_profession_spell(
        &mut self,
        spell_id: u32,
    ) -> Result<bool, crate::frame_error::SessionError> {
        let opener = self
            .professions
            .catalog
            .as_ref()
            .is_some_and(|catalog| catalog.openers.contains_key(&spell_id));
        let known = self.professions.book.snapshot.spells.contains(&spell_id);
        if opener && known {
            if let Some(npc) = self.professions.orders.npc {
                self.account.send_close_interaction(npc)?;
            }
            self.close_professions();
            self.professions.book.visible = true;
            self.professions.book.quantity = 1;
            return Ok(true);
        }
        Ok(false)
    }
    pub(super) fn profession_spell_ids(&self) -> Vec<u32> {
        self.professions
            .catalog
            .as_ref()
            .map_or_else(Vec::new, |catalog| {
                catalog
                    .openers
                    .keys()
                    .filter(|spell| self.professions.book.snapshot.spells.contains(spell))
                    .copied()
                    .collect()
            })
    }
    pub(super) fn update_professions(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.close_professions();
            return Ok(());
        }
        self.professions.poll_catalog(&self.data_root)?;
        self.refresh_profession_recipes();
        self.apply_profession_key()?;
        if !self.professions.book.visible {
            return Ok(());
        }
        self.poll_profession_inputs()?;
        if self.professions.book.visible {
            self.sync_professions()?;
        }
        Ok(())
    }
    fn refresh_profession_recipes(&mut self) {
        let Some(catalog) = self.professions.catalog.as_ref() else {
            return;
        };
        let Some(spells) = self.spells.catalog() else {
            return;
        };
        self.professions.book.recipes = catalog
            .recipes
            .iter()
            .filter_map(|recipe| {
                if !self
                    .professions
                    .book
                    .snapshot
                    .spells
                    .contains(&recipe.spell_id)
                {
                    return None;
                }
                let mut recipe = recipe.clone();
                recipe.name = spells.get(recipe.spell_id)?.name.to_string();
                Some(recipe)
            })
            .collect();
        if self.professions.book.selected_recipe().is_none() {
            self.professions.book.selected = self
                .professions
                .book
                .recipes
                .first()
                .map(|recipe| recipe.spell_id);
        }
    }
    fn apply_profession_key(&mut self) -> Result<(), FrameError> {
        if self.game_menu_ui.is_some() || !self.account.session.gameplay_input_allowed() {
            return Ok(());
        }
        let input = self.physical_input.gameplay_state(self.keyboard_free());
        if self
            .client_options
            .bindings
            .is_just_pressed(InputAction::ToggleProfessions, &input)
        {
            if self.professions.book.visible {
                self.close_professions_window()?;
            } else {
                self.professions.book.visible = true;
                self.professions.book.quantity = 1;
            }
        }
        Ok(())
    }
    fn poll_profession_inputs(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.professions.ui.clone() else {
            return Ok(());
        };
        if self.professions.page == ProfessionPage::Recipes {
            self.professions.book.search = ui.bind_mut().frame_text(SEARCH.into()).to_string();
            self.professions.book.quantity = ui
                .bind_mut()
                .frame_text(QUANTITY.into())
                .to_string()
                .parse()
                .unwrap_or(0);
        }
        self.read_order_form(&mut ui)?;
        self.read_quality_quantities(&mut ui, false)?;
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if action == "profession:close" {
                self.close_professions_window()?;
                break;
            }
            if let Some(tab) = action.strip_prefix("profession:tab:") {
                self.professions.page = match tab {
                    "recipes" => ProfessionPage::Recipes,
                    "specializations" => ProfessionPage::Specializations,
                    "orders" => ProfessionPage::Orders,
                    _ => return Err(format!("Unknown profession tab: {tab}").into()),
                };
                if self.professions.page == ProfessionPage::Specializations {
                    self.query_profession_specializations()?;
                } else if self.professions.page == ProfessionPage::Orders {
                    self.professions.orders.customer = false;
                    self.query_crafting_orders()?;
                }
            } else if let Some(action) = action.strip_prefix("profession:order:") {
                self.apply_crafting_order_action(action)?;
            } else if let Some(action) = action.strip_prefix("profession:quality:") {
                self.apply_quality_reagent_action(action)?;
            } else if action == "profession:recraft" {
                self.professions.page = ProfessionPage::Recraft;
            } else if let Some(action) = action.strip_prefix("profession:spec:") {
                self.apply_specialization_action(action)?;
            } else if let Some(raw) = action.strip_prefix("profession:select:") {
                self.professions.book.selected = Some(
                    raw.parse()
                        .map_err(|_| format!("Invalid recipe selection: {action}"))?,
                );
                self.professions.error.clear();
            } else if let Some(category) = action.strip_prefix("profession:category:") {
                if !self.professions.collapsed.remove(category) {
                    self.professions.collapsed.insert(category.into());
                }
            } else if action == "profession:increase" || action == "profession:decrease" {
                let limit = self
                    .professions
                    .book
                    .craftable_count(&self.merchant.session.inventory)
                    .max(1);
                let quantity = self.professions.book.quantity;
                let next = if action == "profession:increase" {
                    quantity.saturating_add(1)
                } else {
                    quantity.saturating_sub(1)
                };
                self.professions.book.quantity = next.clamp(1, limit);
                ui.bind_mut()
                    .set_editbox_text(QUANTITY, &self.professions.book.quantity.to_string())?;
            } else if action == "profession:create" || action == "profession:all" {
                let all = action == "profession:all";
                if let Some(request) = self
                    .professions
                    .book
                    .craft_request(&self.merchant.session.inventory, all)
                {
                    self.professions.error.clear();
                    self.account.send_craft_recipe(request)?;
                }
            } else {
                return Err(format!("Unknown profession action: {action}").into());
            }
        }
        Ok(())
    }
    fn query_profession_specializations(&mut self) -> Result<(), FrameError> {
        let skill_line = self
            .professions
            .catalog
            .as_ref()
            .and_then(|catalog| catalog.specialization_skill_line(&self.professions.book));
        if let Some(skill_line) = skill_line {
            self.account.send_profession_specialization(
                shared::protocol::ProfessionSpecializationRequest::Query { skill_line },
            )?;
            self.professions.specializations.pending = true;
        } else {
            self.professions.specializations =
                game_engine_ui_model::professions_specializations::SpecializationDraft {
                    status: "No learned specialization tier for the selected profession.".into(),
                    ..Default::default()
                };
        }
        Ok(())
    }
    fn apply_specialization_action(&mut self, action: &str) -> Result<(), FrameError> {
        let state = &mut self.professions.specializations;
        if let Some(id) = action.strip_prefix("tree:") {
            state.selected_tree = Some(
                id.parse()
                    .map_err(|_| format!("Invalid specialization tree: {id}"))?,
            );
            state.selected_node = state.tree().map(|t| t.root_node_id);
        } else if let Some(id) = action.strip_prefix("node:") {
            state.selected_node = Some(
                id.parse()
                    .map_err(|_| format!("Invalid specialization node: {id}"))?,
            );
        } else {
            let request = match action {
                "spend" => {
                    state.stage_rank();
                    None
                }
                "undo" => {
                    state.undo();
                    None
                }
                "apply" => state.commit_request(),
                "unlock" => {
                    state.confirming_unlock = state.can_unlock();
                    None
                }
                "cancel-unlock" => {
                    state.confirming_unlock = false;
                    None
                }
                "confirm-unlock" => state.confirm_unlock(),
                _ => return Err(format!("Unknown specialization action: {action}").into()),
            };
            if let Some(request) = request {
                self.account.send_profession_specialization(request)?;
            }
        }
        Ok(())
    }
    fn profession_view(&mut self) -> ProfessionView {
        let book = self.professions.book.clone();
        let inventory = &self.merchant.session.inventory;
        let reagents = book.reagent_counts(inventory);
        let craftable = book.craftable_count(inventory);
        let casting = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
            .is_some_and(|unit| unit.has::<CastState>());
        self.professions
            .reagent_selection
            .select_recipe(book.selected);
        let reagent_slots = self.selected_quality_slots();
        let items = profession_items(&book, &reagent_slots);
        let crafting_quality = items
            .keys()
            .filter_map(|item| {
                let quality = self
                    .professions
                    .catalog
                    .as_ref()?
                    .crafting_quality
                    .get(item)?;
                Some((*item, quality.clone()))
            })
            .collect();
        let mut view = ProfessionView {
            page: self.professions.page,
            now_unix: godot::classes::Time::singleton().get_unix_time_from_system() as u64,
            specializations: self.professions.specializations.clone(),
            crafting_quality,
            can_create: !casting && book.craft_request(inventory, false).is_some(),
            can_all: !casting && book.craft_request(inventory, true).is_some(),
            skill_names: self
                .professions
                .catalog
                .as_ref()
                .map(|catalog| catalog.skill_names.clone())
                .unwrap_or_default(),
            child_skills: self
                .professions
                .catalog
                .as_ref()
                .map(|catalog| catalog.child_skills.clone())
                .unwrap_or_default(),
            status: self.quality_operation_status(),
            bag_counts: self.profession_reagent_counts(),
            reagent_selection: self.professions.reagent_selection.clone(),
            reagent_slots,
            recipe_craftable: recipe_craftable_counts(&book, inventory),
            collapsed: self.professions.collapsed.clone(),
            book,
            reagents,
            craftable,
            items,
            ..Default::default()
        };
        self.project_order_view(&mut view);
        self.cache_profession_icons(&mut view);
        view
    }
    fn cache_profession_icons(&mut self, view: &mut ProfessionView) {
        let profession = view.book.selected_recipe().map(|recipe| recipe.profession);
        let catalog = self.professions.catalog.as_ref();
        view.profession_name = catalog
            .and_then(|catalog| catalog.skill_names.get(&profession?))
            .cloned()
            .unwrap_or_default();
        let icon = catalog.and_then(|catalog| {
            let opener = catalog
                .openers
                .iter()
                .find(|(_, skill)| Some(**skill) == profession)?
                .0;
            Some(self.spells.catalog()?.get(*opener)?.icon_fdid)
        });
        if let Some(icon) = icon {
            view.profession_icon = self.drawable_fdid(icon);
        }
        if let Some(snapshot) = &mut view.specializations.snapshot {
            for tree in &mut snapshot.trees {
                for node in &mut tree.nodes {
                    node.icon = self.drawable_fdid(node.icon);
                }
            }
        }
        for item in view.items.values_mut() {
            item.icon_fdid = self.drawable_fdid(item.icon_fdid);
        }
    }
    fn sync_professions(&mut self) -> Result<(), String> {
        let view = self.profession_view();
        let scale = self.effective_ui_scale();
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            professions_screen,
        ));
        if let Some(ui) = &mut self.professions.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            ui.bind_mut().set_state(view)?;
            self.write_quality_quantities(&mut ui, false)?;
            return self.initialize_order_form();
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("ProfessionsUI");
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_professions(view))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        if self.professions.page == ProfessionPage::Recipes {
            ui.bind_mut().set_editbox_text(QUANTITY, "1")?;
        }
        self.professions.ui = Some(ui);
        self.initialize_order_form()
    }
}

#[path = "professions_orders.rs"]
mod orders;

fn recipe_craftable_counts(
    book: &ProfessionBook,
    inventory: &game_engine_ui_model::bag_data::InventoryState,
) -> BTreeMap<u32, u32> {
    let mut selected_book = book.clone();
    book.recipes
        .iter()
        .map(|recipe| {
            selected_book.selected = Some(recipe.spell_id);
            (recipe.spell_id, selected_book.craftable_count(inventory))
        })
        .collect()
}

fn profession_items(
    book: &ProfessionBook,
    slots: &[shared::crafting_reagents::ReagentSlot],
) -> BTreeMap<u32, ItemDisplay> {
    let Some(catalog) = item_catalog() else {
        return BTreeMap::new();
    };
    let Some(recipe) = book.selected_recipe() else {
        return BTreeMap::new();
    };
    recipe
        .output
        .into_iter()
        .map(|(item, _)| item)
        .chain(recipe.reagents.iter().map(|&(item, _)| item))
        .chain(
            slots
                .iter()
                .flat_map(|slot| slot.choices.iter().map(|choice| choice.item_id)),
        )
        .filter_map(|id| {
            let item = catalog.get(id)?;
            Some((
                id,
                ItemDisplay {
                    name: item.name.clone(),
                    icon_fdid: catalog.icon_fdid(id).unwrap_or(0),
                    quality: item.quality,
                },
            ))
        })
        .collect()
}
