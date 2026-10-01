//! Native player trade host (docs/specs/trade-frame.md). The server's
//! `TradeStateUpdate` opens, fills and ends the authored TradeFrame; frame clicks, the
//! money entry, bag right-clicks, `/trade` and the unit menu only send requests.
use game_engine_session::SessionScreen;
use game_engine_ui_model::trade::{
    NativeTradeView, TradeRequest, TradeSession, money_texts, parse_money,
};
use game_engine_ui_model::trade_frame_component::MONEY_BOXES;
use godot::prelude::*;
use shared::components::Player;
use shared::protocol::{TradePartySnapshot, TradeStateUpdate};
use ui_toolkit::frame::WidgetData;

use crate::GameClient;
use crate::frame_error::{FrameError, SessionError};
use crate::ui::RegistryUi;

/// Enter in a money box (`MoneyInputFrame` `OnEnterPressed`).
pub(crate) const ACTION_MONEY_SUBMIT: &str = "trade_money_submit";
const MONEY_NAMES: [&str; 3] = [MONEY_BOXES.gold, MONEY_BOXES.silver, MONEY_BOXES.copper];

#[derive(Default)]
pub(crate) struct Trade {
    pub session: TradeSession,
    pub ui: Option<Gd<RegistryUi>>,
    /// The money boxes had focus last frame: losing it submits the amount.
    money_focused: bool,
}

impl Trade {
    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.money_focused = false;
    }

    pub fn reset(&mut self) {
        self.session.reset();
        self.free_ui();
    }
}

/// Typed money box texts, `[gold, silver, copper]`, and whether one has focus.
fn read_money(ui: &Gd<RegistryUi>) -> Option<([String; 3], bool)> {
    let bound = ui.bind();
    let registry = bound.registry()?;
    let mut focused = false;
    let mut texts: [String; 3] = Default::default();
    for (text, name) in texts.iter_mut().zip(MONEY_NAMES) {
        let id = registry.get_by_name(name)?;
        focused |= registry.focused_frame == Some(id);
        if let Some(WidgetData::EditBox(edit)) = registry.get(id)?.widget_data.as_ref() {
            *text = edit.text.clone();
        }
    }
    Some((texts, focused))
}

impl GameClient {
    pub(crate) fn receive_trade(&mut self, update: TradeStateUpdate) -> Result<(), String> {
        for text in self.trade.session.receive(update) {
            self.add_world_error(&text)?;
        }
        Ok(())
    }

    pub(crate) fn send_trade(&mut self, request: Option<TradeRequest>) -> Result<(), SessionError> {
        match request {
            Some(request) => self.account.send_trade(request),
            None => Ok(()),
        }
    }

    /// `UnitPopupTradeButtonMixin:OnClick` / `/trade`: `InitiateTrade(unit)`.
    pub(crate) fn initiate_trade(&mut self, name: String) -> Result<(), SessionError> {
        self.account.send_trade(TradeRequest::Initiate(name))
    }

    /// `/trade`: `InitiateTrade("target")` with the targeted player; without one,
    /// `ERR_NO_TARGET_OR_NAME`.
    pub(crate) fn trade_with_target(&mut self) -> Result<(), String> {
        let name = self
            .targeting_target()
            .and_then(|id| self.replica.unit(id))
            .and_then(|unit| unit.get::<Player>())
            .map(|player| player.name.clone());
        match name {
            Some(name) => self.initiate_trade(name).map_err(|error| error.0),
            None => self.add_world_error("No target or name specified."),
        }
    }

    /// A bag right-click while the window is open; `false` leaves it to the bags.
    pub(crate) fn offer_trade_item(&mut self, bag_action: &str) -> Result<bool, SessionError> {
        if !self.trade.session.window_open() {
            return Ok(false);
        }
        let request = self
            .trade
            .session
            .offer(bag_action, &self.merchant.session.inventory);
        self.send_trade(request)?;
        Ok(true)
    }

    /// Escape's `CloseAllWindows` hides TradeFrame; `TradeFrame_OnHide` cancels.
    pub(super) fn close_trade_window(&mut self) -> Result<bool, SessionError> {
        let request = self.trade.session.close();
        let closed = request.is_some();
        self.send_trade(request)?;
        Ok(closed)
    }

    pub(super) fn update_trade(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.trade.reset();
            return Ok(());
        }
        if !self.trade.session.window_open() {
            self.trade.free_ui();
            return Ok(());
        }
        if self.trade.ui.is_some()
            && self.game_menu_ui.is_none()
            && self.account.session.gameplay_input_allowed()
        {
            self.poll_trade_input()?;
        }
        if !self.trade.session.window_open() {
            self.trade.free_ui();
            return Ok(());
        }
        self.show_trade()?;
        Ok(())
    }

    fn show_trade(&mut self) -> Result<(), String> {
        let view = NativeTradeView {
            frame: self.trade.session.view(),
        };
        let scale = self.effective_ui_scale();
        if self.trade.ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("TradeUI");
            self.base_mut().add_child(&ui);
            let shown = ui.bind_mut().show_trade(view.clone());
            if let Err(error) = shown {
                ui.free();
                return Err(error);
            }
            self.trade.ui = Some(ui);
        }
        let mut ui = self.trade.ui.clone().ok_or("Trade UI missing")?;
        let typed = read_money(&ui);
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(view)?;
        // `PLAYER_TRADE_MONEY`: the entry shows the offer unless it is being typed.
        let shown = match typed {
            Some((texts, true)) => texts,
            _ => money_texts(self.trade.session.offered_money().unwrap_or(0)),
        };
        let current = read_money(&ui).map(|(texts, _)| texts);
        if current.as_ref() != Some(&shown) {
            for (name, text) in MONEY_NAMES.into_iter().zip(&shown) {
                ui.bind_mut().set_editbox_text(name, text)?;
            }
        }
        Ok(())
    }

    fn poll_trade_input(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.trade.ui.clone() else {
            return Ok(());
        };
        let error = ui.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string().into());
        }
        let (texts, focused) = read_money(&ui).ok_or("Trade money boxes missing")?;
        let blurred = self.trade.money_focused && !focused;
        self.trade.money_focused = focused;
        let mut submit = blurred;
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if action == ACTION_MONEY_SUBMIT {
                submit = true;
                continue;
            }
            let request = self.trade.session.click(&action);
            self.send_trade(request)?;
        }
        if submit {
            self.submit_trade_money(texts)?;
        }
        Ok(())
    }

    /// `TradeFrame_UpdateMoney`: the typed amount becomes the offer when it changed;
    /// an amount that does not parse shows the current offer again.
    fn submit_trade_money(&mut self, texts: [String; 3]) -> Result<(), SessionError> {
        let [gold, silver, copper] = &texts;
        let Some(copper) = parse_money([gold, silver, copper]) else {
            return Ok(());
        };
        let request = self.trade.session.money(copper);
        self.send_trade(request)
    }
}

fn party_snapshot(party: &TradePartySnapshot) -> VarDictionary {
    let mut state = VarDictionary::new();
    state.set("name", party.name.as_str());
    state.set("accepted", party.accepted);
    state.set("gold", party.gold as i64);
    let mut slots = VarArray::new();
    for slot in &party.slots {
        let Some(item) = slot else {
            slots.push(&Variant::nil());
            continue;
        };
        let mut row = VarDictionary::new();
        row.set("guid", item.item_guid as i64);
        row.set("item_id", i64::from(item.item_id));
        row.set("name", item.name.as_str());
        row.set("count", i64::from(item.stack_count));
        slots.push(&row.to_variant());
    }
    state.set("slots", &slots);
    state
}

impl GameClient {
    pub(super) fn trade_snapshot(&self) -> VarDictionary {
        let session = &self.trade.session;
        let mut state = VarDictionary::new();
        state.set("window_open", session.window_open());
        // Other replicated players a trade can be started with, by name.
        let local = self.world.local_player_id();
        let mut players = VarDictionary::new();
        for unit in self.replica.units() {
            if let Some(player) = unit.get::<Player>()
                && Some(unit.server_id) != local
            {
                players.set(player.name.as_str(), unit.server_id as i64);
            }
        }
        state.set("players", &players);
        let Some(snapshot) = &session.snapshot else {
            return state;
        };
        state.set("phase", format!("{:?}", snapshot.phase).as_str());
        state.set("player", &party_snapshot(&snapshot.player));
        state.set("other", &party_snapshot(&snapshot.other));
        state
    }
}
