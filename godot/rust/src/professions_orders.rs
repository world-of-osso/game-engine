//! Native fixed-output order requests, form input and catalog projection.
use super::*;
use game_engine_ui_model::professions_frame::orders::{NOTE, ORDER_MONEY, ORDER_SEARCH, RECIPIENT};
use game_engine_ui_model::professions_orders::owned_count;
use game_engine_ui_model::trade::{money_texts, parse_money};
use shared::protocol::{CraftingOrderKind, CraftingOrderRequest};

impl GameClient {
    pub(crate) fn receive_crafting_order_interaction(
        &mut self,
        message: &crate::account::NpcMessage,
    ) -> Result<bool, String> {
        use crate::account::NpcMessage;
        use shared::protocol::{InteractionKind, NpcRole};
        match message {
            NpcMessage::Opened(opened)
                if opened.kind == InteractionKind::Role(NpcRole::CraftingOrders) =>
            {
                self.professions.free_ui();
                self.professions.page = ProfessionPage::Orders;
                self.professions.book.visible = true;
                self.professions.order_form_initialized = false;
                let request = self.professions.orders.open_customer(opened.npc);
                self.merchant.session.close();
                self.mailbox.session.close();
                self.auction_interaction_closed_any();
                self.account
                    .send_crafting_order(request)
                    .map_err(|error| error.to_string())?;
                Ok(true)
            }
            NpcMessage::Closed(npc) if self.professions.orders.npc == Some(*npc) => {
                self.professions.orders.close_for(*npc);
                self.close_professions();
                Ok(false)
            }
            NpcMessage::Opened(_) if self.professions.orders.npc.is_some() => {
                self.close_professions();
                Ok(false)
            }
            _ => Ok(false),
        }
    }
    fn order_skill_line(&self) -> u32 {
        if self.professions.orders.customer {
            return 0;
        }
        self.professions
            .orders
            .selected_recipe()
            .map(|recipe| recipe.skill_line)
            .or_else(|| {
                self.professions
                    .book
                    .selected_recipe()
                    .map(|recipe| recipe.skill_line)
            })
            .unwrap_or(0)
    }
    pub(super) fn query_crafting_orders(&mut self) -> Result<(), FrameError> {
        let state = &self.professions.orders;
        let request = CraftingOrderRequest::Query {
            skill_line: self.order_skill_line(),
            kind: state.kind,
            mine: state.customer,
        };
        self.account.send_crafting_order(request)?;
        self.professions.orders.pending = true;
        Ok(())
    }
    pub(super) fn read_order_form(&mut self, ui: &mut Gd<RegistryUi>) -> Result<(), FrameError> {
        if self.professions.page != ProfessionPage::Orders {
            return Ok(());
        }
        let has_search = ui
            .bind()
            .registry()
            .is_some_and(|r| r.get_by_name(ORDER_SEARCH).is_some());
        if has_search {
            self.professions.order_search =
                ui.bind_mut().frame_text(ORDER_SEARCH.into()).to_string();
        }
        if !self.professions.order_form_initialized {
            return Ok(());
        }
        let has_form = ui
            .bind()
            .registry()
            .is_some_and(|r| r.get_by_name(NOTE).is_some());
        if !has_form {
            return Ok(());
        }
        let state = &mut self.professions.orders;
        state.note = ui.bind_mut().frame_text(NOTE.into()).to_string();
        if state.kind == CraftingOrderKind::Personal {
            state.recipient = ui.bind_mut().frame_text(RECIPIENT.into()).to_string();
        }
        let texts = ORDER_MONEY
            .all()
            .map(|name| ui.bind_mut().frame_text(name.into()).to_string());
        state.commission = parse_money([&texts[0], &texts[1], &texts[2]])
            .map(|money| money.to_string())
            .unwrap_or_else(|| "invalid".into());
        Ok(())
    }
    pub(super) fn initialize_order_form(&mut self) -> Result<(), String> {
        let state = &self.professions.orders;
        if self.professions.order_form_initialized
            || !state.customer
            || !state.creating
            || state.recipe.is_none()
        {
            return Ok(());
        }
        let Some(ui) = &mut self.professions.ui else {
            return Ok(());
        };
        if !ui
            .bind()
            .registry()
            .is_some_and(|r| r.get_by_name(NOTE).is_some())
        {
            return Ok(());
        }
        let mut bound = ui.bind_mut();
        bound.set_editbox_text(NOTE, &state.note)?;
        if state.kind == CraftingOrderKind::Personal {
            bound.set_editbox_text(RECIPIENT, &state.recipient)?;
        }
        let texts = money_texts(
            state
                .commission
                .parse()
                .map_err(|_| "Invalid stored crafting order commission")?,
        );
        for (name, text) in ORDER_MONEY.all().into_iter().zip(texts) {
            bound.set_editbox_text(name, &text)?;
        }
        self.professions.order_form_initialized = true;
        Ok(())
    }
    pub(super) fn apply_crafting_order_action(&mut self, action: &str) -> Result<(), FrameError> {
        if self.professions.orders.pending {
            return Ok(());
        }
        if !matches!(action, "recipient-menu" | "duration-menu") {
            self.professions.orders.menu = None;
        }
        if let Some(id) = action.strip_prefix("select:") {
            self.professions.orders.selected =
                Some(id.parse().map_err(|_| format!("Invalid order ID: {id}"))?);
        } else if let Some(id) = action.strip_prefix("recipe:") {
            let spell = id
                .parse()
                .map_err(|_| format!("Invalid order recipe: {id}"))?;
            self.professions.orders.recipe = Some(spell);
            self.professions.orders.supplied.clear();
            self.professions.order_form_initialized = false;
            if !self.professions.orders.customer {
                self.query_crafting_orders()?;
            }
        } else if let Some(kind) = action.strip_prefix("kind:") {
            self.professions.orders.kind = parse_kind(kind)?;
            self.professions.orders.selected = None;
            self.query_crafting_orders()?;
        } else if let Some(kind) = action.strip_prefix("recipient:") {
            self.professions.orders.kind = parse_kind(kind)?;
            self.professions.orders.menu = None;
            self.professions.order_form_initialized = false;
        } else if let Some(hours) = action.strip_prefix("duration:") {
            let hours = hours
                .parse::<u8>()
                .map_err(|_| format!("Invalid order duration: {hours}"))?;
            if ![12, 24, 48].contains(&hours) {
                return Err(format!("Unsupported order duration: {hours}").into());
            }
            self.professions.orders.duration_hours = hours;
            self.professions.orders.menu = None;
        } else if let Some(item) = action.strip_prefix("supply:") {
            self.toggle_order_supply(item)?;
        } else {
            self.apply_order_control(action)?;
        }
        Ok(())
    }
    fn toggle_order_supply(&mut self, item: &str) -> Result<(), FrameError> {
        let item: u32 = item
            .parse()
            .map_err(|_| format!("Invalid supplied reagent: {item}"))?;
        let Some(recipe) = self.professions.orders.selected_recipe() else {
            return Ok(());
        };
        let Some(requirement) = recipe
            .reagents
            .iter()
            .find(|reagent| reagent.item_id == item)
        else {
            return Err(format!("Reagent {item} is not required by this recipe").into());
        };
        let supplied = self
            .professions
            .orders
            .supplied
            .get(&item)
            .copied()
            .unwrap_or(0);
        let next = if supplied > 0 {
            0
        } else {
            requirement
                .quantity
                .min(owned_count(&self.merchant.session.inventory, item))
        };
        self.professions.orders.supplied.insert(item, next);
        Ok(())
    }
    fn apply_order_control(&mut self, action: &str) -> Result<(), FrameError> {
        match action {
            "browse" => {
                self.professions.orders.creating = true;
                self.professions.orders.selected = None;
                self.professions.orders.recipe = None;
                self.professions.order_form_initialized = false;
            }
            "mine" => {
                self.professions.orders.creating = false;
                self.professions.orders.selected = None;
                self.query_crafting_orders()?;
            }
            "back" => self.professions.orders.selected = None,
            "refresh" => self.query_crafting_orders()?,
            "recipient-menu" => self
                .toggle_order_menu(game_engine_ui_model::professions_orders::OrderMenu::Recipient),
            "duration-menu" => self
                .toggle_order_menu(game_engine_ui_model::professions_orders::OrderMenu::Duration),
            "place" => self.place_crafting_order()?,
            "claim" | "release" | "fulfill" | "cancel" => {
                if let Some(request) = self.professions.orders.action_request(action) {
                    self.account.send_crafting_order(request)?;
                }
            }
            _ => return Err(format!("Unknown crafting order action: {action}").into()),
        }
        Ok(())
    }
    fn toggle_order_menu(&mut self, menu: game_engine_ui_model::professions_orders::OrderMenu) {
        self.professions.orders.menu = if self.professions.orders.menu == Some(menu) {
            None
        } else {
            Some(menu)
        };
    }
    fn place_crafting_order(&mut self) -> Result<(), FrameError> {
        let recipe = self.professions.orders.selected_recipe().cloned();
        let Some(recipe) = recipe else {
            return Ok(());
        };
        match self
            .professions
            .orders
            .place_request(&recipe, &self.merchant.session.inventory)
        {
            Ok(request) => self.account.send_crafting_order(request)?,
            Err(error) => self.professions.orders.status = error,
        }
        Ok(())
    }
    pub(super) fn project_order_view(&self, view: &mut ProfessionView) {
        view.orders = self.professions.orders.clone();
        view.order_recipes = view
            .orders
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.recipes.clone())
            .unwrap_or_default();
        let search = self.professions.order_search.to_lowercase();
        view.order_recipes
            .retain(|r| r.name.to_lowercase().contains(&search));
        if let Some(recipe) = view.orders.selected_recipe() {
            view.order_can_place = view
                .orders
                .validate_place(recipe, &self.merchant.session.inventory)
                .is_ok();
            for reagent in &recipe.reagents {
                view.owned_reagents.insert(
                    reagent.item_id,
                    owned_count(&self.merchant.session.inventory, reagent.item_id),
                );
            }
            insert_order_items(
                &mut view.items,
                std::iter::once(recipe.item_id).chain(recipe.reagents.iter().map(|r| r.item_id)),
            );
        }
        if let Some(snapshot) = &view.orders.snapshot {
            insert_order_items(
                &mut view.items,
                snapshot.orders.iter().flat_map(|o| {
                    std::iter::once(o.item_id).chain(o.reagents.iter().map(|r| r.item_id))
                }),
            );
        }
    }
}
fn parse_kind(value: &str) -> Result<CraftingOrderKind, String> {
    match value {
        "public" => Ok(CraftingOrderKind::Public),
        "guild" => Ok(CraftingOrderKind::Guild),
        "personal" => Ok(CraftingOrderKind::Personal),
        _ => Err(format!("Unknown order kind: {value}")),
    }
}
fn insert_order_items(items: &mut BTreeMap<u32, ItemDisplay>, ids: impl Iterator<Item = u32>) {
    let Some(catalog) = item_catalog() else {
        return;
    };
    for id in ids {
        if let Some(item) = catalog.get(id) {
            items.insert(
                id,
                ItemDisplay {
                    name: item.name.clone(),
                    icon_fdid: catalog.icon_fdid(id).unwrap_or(0),
                    quality: item.quality,
                },
            );
        }
    }
}
