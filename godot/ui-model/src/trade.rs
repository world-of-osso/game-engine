//! Native trade session (docs/specs/trade-frame.md). The server's `TradeStateUpdate`
//! is the only authority: requests never change the snapshot, bags or money.
use crate::bag_data::{InventoryState, stack_slot};
use crate::bag_frame_component::parse_bag_slot_action;
use crate::bank_art::SlotItem;
use crate::cursor_item::CursorItem;
use crate::merchant_data::quality_color;
use crate::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use crate::trade_frame_component::{
    ACTION_CANCEL, ACTION_CLOSE, ACTION_PLAYER_SLOT_PREFIX, ACTION_TRADE, TRADE_SLOTS,
    TradeFrameState, TradeItemView,
};
use shared::protocol::{
    ItemStack, SetTradeItem, TradeItemSnapshot, TradePartySnapshot, TradePhase, TradeSnapshot,
    TradeStateUpdate,
};
use shared::trade::TRADE_SLOT_TRADED_COUNT;
use std::time::Duration;

/// `StaticPopupDialogs["TRADE"]` (GameDialogDefs.lua:1273).
pub const TRADE_POPUP: &str = "TRADE";
/// `StaticPopupTimeoutSec` (StaticPopup.lua:22).
const TRADE_POPUP_TIMEOUT: Duration = Duration::from_secs(60);

/// A request for the server (`TradeChannel`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TradeRequest {
    /// `InitiateTrade(unit)` with the unit's player name.
    Initiate(String),
    /// `TRADE` popup Yes (`BeginTrade`).
    Accept,
    /// `TRADE` popup No or its timeout (`CancelTrade`).
    Decline,
    /// `CancelTrade` / `CloseTrade`.
    Cancel,
    SetItem(SetTradeItem),
    ClearItem(u8),
    SetMoney(u64),
    /// `TradeFrameTradeButton` (`AcceptTrade`).
    Confirm,
    /// `CancelTradeAccept`.
    CancelAccept,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeTradeView {
    pub frame: TradeFrameState,
}

pub fn native_trade_screen(
    ctx: &ui_toolkit::screen::SharedContext,
) -> ui_toolkit::widget_def::Element {
    let view = ctx
        .get::<NativeTradeView>()
        .expect("NativeTradeView must be in SharedContext");
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(view.frame.clone());
    crate::trade_frame_component::trade_frame_screen(&shared)
}

/// Where the incoming request's `TRADE` popup is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Invitation {
    #[default]
    None,
    Asked,
    /// Yes/No/timeout already answered; the server's next phase ends the request.
    Answered,
}

#[derive(Default)]
pub struct TradeSession {
    pub snapshot: Option<TradeSnapshot>,
    invitation: Invitation,
    /// The player closed the window; it stays closed until the trade ends.
    cancelling: bool,
}

impl TradeSession {
    fn phase(&self) -> Option<TradePhase> {
        self.snapshot.as_ref().map(|snapshot| snapshot.phase)
    }

    pub fn window_open(&self) -> bool {
        self.phase() == Some(TradePhase::Open) && !self.cancelling
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Apply a server update; returns the error-frame texts. A refusal without a
    /// snapshot keeps the trade; any other update replaces it (`None` ends it).
    pub fn receive(&mut self, update: TradeStateUpdate) -> Vec<String> {
        let refusal = update.trade.is_none() && update.error.is_some();
        if !refusal {
            self.snapshot = update.trade;
            if self.snapshot.is_none() {
                self.cancelling = false;
            }
            if self.phase() != Some(TradePhase::PendingIncoming)
                && self.invitation == Invitation::Answered
            {
                self.invitation = Invitation::None;
            }
        }
        update.error.into_iter().chain(update.message).collect()
    }

    /// `TRADE_REQUEST` shows "Trade with %s?" once; the request ending hides it.
    pub fn sync_popup(&mut self, popups: &mut PopupStack) {
        let incoming = self.phase() == Some(TradePhase::PendingIncoming);
        match self.invitation {
            Invitation::None if incoming => {
                let name = self
                    .snapshot
                    .as_ref()
                    .map_or_else(String::new, |snapshot| snapshot.other.name.clone());
                popups.push(PopupSpec {
                    key: TRADE_POPUP.into(),
                    text: format!("Trade with {name}?"),
                    accept_label: "Yes".into(),
                    cancel_label: Some("No".into()),
                    timeout: Some(TRADE_POPUP_TIMEOUT),
                    confirm_text: None,
                });
                self.invitation = Invitation::Asked;
            }
            Invitation::Asked if !incoming => {
                popups.hide(TRADE_POPUP);
                self.invitation = Invitation::None;
            }
            Invitation::Answered if !incoming => self.invitation = Invitation::None,
            _ => {}
        }
    }

    /// Yes is `BeginTrade`; No and the timeout decline (GameDialogDefs.lua:1273-1284).
    /// Each shown popup answers at most once.
    pub fn popup_results(&mut self, results: &[PopupResult]) -> Vec<TradeRequest> {
        let Some(result) = results.iter().find(|result| result.key == TRADE_POPUP) else {
            return Vec::new();
        };
        if self.invitation != Invitation::Asked {
            return Vec::new();
        }
        self.invitation = Invitation::Answered;
        vec![match result.outcome {
            PopupOutcome::Accepted => TradeRequest::Accept,
            PopupOutcome::Cancelled | PopupOutcome::TimedOut => TradeRequest::Decline,
        }]
    }

    /// `TradeFrame_OnHide` → `CloseTrade`, sent once.
    pub fn close(&mut self) -> Option<TradeRequest> {
        if !self.window_open() {
            return None;
        }
        self.cancelling = true;
        Some(TradeRequest::Cancel)
    }

    fn player(&self) -> Option<&TradePartySnapshot> {
        self.snapshot
            .as_ref()
            .filter(|_| self.window_open())
            .map(|snapshot| &snapshot.player)
    }

    /// A frame click. `TradeFrameCancelButton_OnClick` withdraws an accept before it
    /// closes the trade (TF.lua:226-232); Trade is disabled once accepted (TF.lua:212).
    pub fn click(&mut self, action: &str) -> Option<TradeRequest> {
        let accepted = self.player()?.accepted;
        match action {
            ACTION_TRADE if !accepted => Some(TradeRequest::Confirm),
            ACTION_TRADE => None,
            ACTION_CANCEL if accepted => Some(TradeRequest::CancelAccept),
            ACTION_CANCEL | ACTION_CLOSE => self.close(),
            _ => {
                let slot: u8 = action
                    .strip_prefix(ACTION_PLAYER_SLOT_PREFIX)?
                    .parse()
                    .ok()?;
                self.player()?
                    .slots
                    .get(usize::from(slot))?
                    .as_ref()
                    .map(|_| TradeRequest::ClearItem(slot))
            }
        }
    }

    /// The player's offered money while the window is open.
    pub fn offered_money(&self) -> Option<u64> {
        self.player().map(|player| player.gold)
    }

    /// `TradeFrame_UpdateMoney`: the typed amount, only when it changed.
    pub fn money(&self, copper: u64) -> Option<TradeRequest> {
        (self.player()?.gold != copper).then_some(TradeRequest::SetMoney(copper))
    }

    /// Right-clicking a bag item offers its whole stack in the first free traded slot
    /// (never the "Will not be traded" slot); an offered item is not offered twice.
    pub fn offer(&self, bag_action: &str, inventory: &InventoryState) -> Option<TradeRequest> {
        let player = self.player()?;
        let (bag, slot) = parse_bag_slot_action(bag_action)?;
        let item = inventory.slot(bag, slot).filter(|item| !item.is_empty())?;
        let offered = player
            .slots
            .iter()
            .flatten()
            .any(|offer| offer.item_guid == item.item_guid);
        if offered {
            return None;
        }
        let free = (0..TRADE_SLOT_TRADED_COUNT)
            .find(|&index| player.slots.get(index).is_none_or(Option::is_none))?;
        Some(TradeRequest::SetItem(SetTradeItem {
            slot: free as u8,
            item_guid: item.item_guid,
            stack_count: u16::try_from(item.count).ok()?,
        }))
    }

    /// `ClickTradeButton(slot)` with an item on the cursor offers it there, the
    /// "Will not be traded" slot included (TradeFrame.xml:108-116); a split cursor
    /// offers its count.
    pub fn place(
        &self,
        slot: usize,
        cursor: &CursorItem,
        inventory: &InventoryState,
    ) -> Option<TradeRequest> {
        self.player()?;
        let CursorItem::Inventory { from, count, .. } = cursor else {
            return None;
        };
        if slot >= TRADE_SLOTS {
            return None;
        }
        let item = inventory.item_at(*from)?;
        Some(TradeRequest::SetItem(SetTradeItem {
            slot: u8::try_from(slot).ok()?,
            item_guid: item.item_guid,
            stack_count: u16::try_from(*count).ok()?,
        }))
    }

    pub fn view(&self) -> TradeFrameState {
        let Some(snapshot) = self.snapshot.as_ref().filter(|_| self.window_open()) else {
            return TradeFrameState::default();
        };
        TradeFrameState {
            visible: true,
            player_name: snapshot.player.name.clone(),
            recipient_name: snapshot.other.name.clone(),
            player_items: items(&snapshot.player),
            recipient_items: items(&snapshot.other),
            recipient_money: snapshot.other.gold,
            player_accepted: snapshot.player.accepted,
            recipient_accepted: snapshot.other.accepted,
        }
    }
}

fn items(party: &TradePartySnapshot) -> Vec<Option<TradeItemView>> {
    (0..TRADE_SLOTS)
        .map(|slot| {
            party
                .slots
                .get(slot)
                .and_then(Option::as_ref)
                .map(item_view)
        })
        .collect()
}

fn item_view(item: &TradeItemSnapshot) -> TradeItemView {
    let slot = stack_slot(&ItemStack {
        item_guid: item.item_guid,
        item_id: item.item_id,
        definition_source: item.definition_source,
        count: item.stack_count,
        durability: None,
        soulbound: false,
    });
    TradeItemView {
        item: SlotItem {
            icon_fdid: slot.icon_fdid,
            count: slot.count,
            quality_border: quality_color(item.quality).into(),
        },
        name: item.name.clone(),
        name_color: quality_color(item.quality).into(),
    }
}

/// `MoneyInputFrame_SetCopper`: gold blank when zero, silver and copper blank when
/// the whole amount is zero; `[gold, silver, copper]`.
pub fn money_texts(copper: u64) -> [String; 3] {
    let shown = |value: u64, shown: bool| {
        if shown {
            value.to_string()
        } else {
            String::new()
        }
    };
    let gold = copper / 10_000;
    [
        shown(gold, gold > 0),
        shown(copper / 100 % 100, copper > 0),
        shown(copper % 100, copper > 0),
    ]
}

/// `MoneyInputFrame_GetCopper` of `[gold, silver, copper]` box texts; blank is zero.
pub fn parse_money(texts: [&str; 3]) -> Option<u64> {
    let part = |text: &str| -> Option<u64> {
        let text = text.trim();
        if text.is_empty() {
            Some(0)
        } else {
            text.parse().ok()
        }
    };
    let [gold, silver, copper] = texts.map(part);
    gold?
        .checked_mul(10_000)?
        .checked_add(silver?.checked_mul(100)?)?
        .checked_add(copper?)
}
