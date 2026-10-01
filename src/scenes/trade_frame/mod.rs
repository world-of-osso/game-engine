//! TradeFrame scene: builds the Retail frame from [`TradeClientState`]; an open trade
//! opens its window with the backpack, and closing the window cancels the trade.
//! An incoming request asks `TRADE` ("Trade with %s?", StaticPopup) first. Clicks
//! and the money entry become [`TradeAction`]s; right-clicking a bag item offers it
//! (`scenes::bag_frame`).

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::bag_data::stack_slot;
use game_engine::merchant_data::quality_color;
use game_engine::trade::{TradeAction, TradeClientState};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use game_engine::ui::screens::bank_art::SlotItem;
use game_engine::ui::screens::trade_frame_component::{
    ACTION_CANCEL, ACTION_CLOSE, ACTION_PLAYER_SLOT_PREFIX, ACTION_TRADE, MONEY_BOXES, TRADE_SLOTS,
    TradeFrameState, TradeItemView, trade_frame_screen,
};
use shared::protocol::{ItemStack, TradeItemSnapshot, TradePartySnapshot, TradePhase};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::scenes::frame_input::{self, Pointer};
use crate::scenes::static_popup::StaticPopupSystems;
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};

/// `StaticPopupDialogs["TRADE"]` (GameDialogDefs.lua:1273).
pub const TRADE_POPUP: &str = "TRADE";
/// `StaticPopupTimeoutSec` (StaticPopup.lua:22).
const TRADE_POPUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

struct TradeFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for TradeFrameRes {}
unsafe impl Sync for TradeFrameRes {}

#[derive(Resource)]
struct TradeFrameWrap(TradeFrameRes, TradeFrameState);

pub struct TradeFramePlugin;

impl Plugin for TradeFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TradeClientState>()
            .init_resource::<PopupStack>()
            .add_message::<PopupResult>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_trade_frame.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_trade_frame);
        app.add_systems(
            Update,
            (
                ask_trade_requests,
                answer_trade_popup,
                handle_trade_clicks,
                handle_trade_keyboard,
                sync_trade_window,
                sync_trade_money,
                sync_trade_frame,
            )
                .chain()
                .after(StaticPopupSystems)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn item_view(item: &TradeItemSnapshot) -> TradeItemView {
    let slot = stack_slot(&ItemStack {
        item_guid: item.item_guid,
        item_id: item.item_id,
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

pub fn trade_frame_state(trade: &TradeClientState, window_open: bool) -> TradeFrameState {
    let Some(snapshot) = trade.snapshot.as_ref().filter(|_| trade.is_open()) else {
        return TradeFrameState::default();
    };
    TradeFrameState {
        visible: window_open,
        player_name: snapshot.player.name.clone(),
        recipient_name: snapshot.other.name.clone(),
        player_items: items(&snapshot.player),
        recipient_items: items(&snapshot.other),
        recipient_money: snapshot.other.gold,
        player_accepted: snapshot.player.accepted,
        recipient_accepted: snapshot.other.accepted,
    }
}

fn build_trade_frame(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    trade: Res<TradeClientState>,
    manager: Res<WindowManager>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = trade_frame_state(&trade, manager.is_open(WindowId::Trade));
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(trade_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(TradeFrameWrap(TradeFrameRes { screen, shared }, state));
}

fn teardown_trade_frame(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    wrap: Option<ResMut<TradeFrameWrap>>,
) {
    if let Some(mut wrap) = wrap {
        wrap.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<TradeFrameWrap>();
}

fn sync_trade_frame(
    mut ui: ResMut<UiState>,
    wrap: Option<ResMut<TradeFrameWrap>>,
    trade: Res<TradeClientState>,
    manager: Res<WindowManager>,
) {
    let Some(mut wrap) = wrap else { return };
    let state = trade_frame_state(&trade, manager.is_open(WindowId::Trade));
    if wrap.1 == state {
        return;
    }
    wrap.1 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

/// `TRADE_REQUEST` shows the `TRADE` popup while the request is pending; the
/// request ending (the other side cancelled) hides it.
fn ask_trade_requests(
    trade: Res<TradeClientState>,
    mut popups: ResMut<PopupStack>,
    mut asked: Local<bool>,
) {
    let incoming = trade.phase() == Some(TradePhase::PendingIncoming);
    if incoming && !*asked {
        let name = trade
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
    } else if !incoming && *asked {
        popups.hide(TRADE_POPUP);
    }
    *asked = incoming;
}

/// Yes is `BeginTrade`; No and the timeout are `CancelTrade` (GameDialogDefs.lua:1273-1284).
fn answer_trade_popup(
    mut results: MessageReader<PopupResult>,
    mut trade: ResMut<TradeClientState>,
) {
    for result in results.read().filter(|result| result.key == TRADE_POPUP) {
        if trade.phase() != Some(TradePhase::PendingIncoming) {
            continue;
        }
        trade.queue(match result.outcome {
            PopupOutcome::Accepted => TradeAction::Accept,
            PopupOutcome::Cancelled | PopupOutcome::TimedOut => TradeAction::Decline,
        });
    }
}

/// The frame's action for a click: `TradeFrameCancelButton_OnClick` withdraws an
/// accept before it closes the trade (TF.lua:226-232).
pub fn trade_click(action: &str, trade: &TradeClientState) -> Option<TradeAction> {
    let player_accepted = trade
        .snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.player.accepted);
    match action {
        ACTION_TRADE => Some(TradeAction::Confirm),
        ACTION_CANCEL if player_accepted => Some(TradeAction::CancelAccept),
        ACTION_CANCEL | ACTION_CLOSE => Some(TradeAction::Cancel),
        _ => {
            let slot: u8 = action
                .strip_prefix(ACTION_PLAYER_SLOT_PREFIX)?
                .parse()
                .ok()?;
            // An empty slot without a cursor item does nothing.
            let player = &trade.snapshot.as_ref()?.player;
            player
                .slots
                .get(usize::from(slot))?
                .as_ref()
                .map(|_| TradeAction::ClearItem(slot))
        }
    }
}

fn handle_trade_clicks(
    pointer: Pointer,
    mut ui: ResMut<UiState>,
    mut trade: ResMut<TradeClientState>,
) {
    if !trade.is_open() {
        return;
    }
    let Some((_, frame_id)) = pointer.click(&ui) else {
        return;
    };
    let specs = frame_input::money_specs(MONEY_BOXES);
    if frame_input::spec_of(&ui.registry, &specs, frame_id).is_some() {
        frame_input::set_focus(&mut ui, Some(frame_id));
        return;
    }
    if frame_input::focused_spec(&ui, &specs).is_some() {
        frame_input::set_focus(&mut ui, None);
        submit_money(&ui, &mut trade);
    }
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    if let Some(action) = trade_click(&action, &trade) {
        trade.queue(action);
    }
}

/// `TradeFrame_UpdateMoney`: the typed amount becomes the offer when it changed.
fn submit_money(ui: &UiState, trade: &mut TradeClientState) {
    let copper = frame_input::money(&ui.registry, MONEY_BOXES);
    let offered = trade.snapshot.as_ref().map(|snapshot| snapshot.player.gold);
    if offered != Some(copper) {
        trade.queue(TradeAction::SetMoney(copper));
    }
}

fn handle_trade_keyboard(
    mut key_events: MessageReader<KeyboardInput>,
    mut ui: ResMut<UiState>,
    mut trade: ResMut<TradeClientState>,
) {
    let specs = frame_input::money_specs(MONEY_BOXES);
    if frame_input::type_into(&mut key_events, &mut ui, &specs).is_some() {
        submit_money(&ui, &mut trade);
    }
}

/// `PLAYER_TRADE_MONEY`: the entry shows the offered money unless it is being typed.
fn sync_trade_money(
    mut ui: ResMut<UiState>,
    trade: Res<TradeClientState>,
    mut shown: Local<Option<u64>>,
) {
    let offered = trade
        .snapshot
        .as_ref()
        .filter(|_| trade.is_open())
        .map(|snapshot| snapshot.player.gold);
    let specs = frame_input::money_specs(MONEY_BOXES);
    if offered == *shown || frame_input::focused_spec(&ui, &specs).is_some() {
        return;
    }
    frame_input::set_money(&mut ui.registry, MONEY_BOXES, offered.unwrap_or(0));
    *shown = offered;
}

/// Where the trade window is in its life.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TradeWindow {
    #[default]
    Closed,
    Open,
    /// The player closed the window; the cancel is on its way.
    Cancelling,
}

/// An open trade opens the window and the backpack; closing the window cancels the
/// trade (`TradeFrame_OnHide` → `CloseTrade`); the trade ending closes both.
pub fn step_trade_window(
    window: TradeWindow,
    trading: bool,
    window_open: bool,
    manager: &mut WindowManager,
) -> (TradeWindow, Option<TradeAction>) {
    match window {
        TradeWindow::Closed if trading => {
            manager.open(WindowId::Trade);
            manager.open(WindowId::Bag(0));
            (TradeWindow::Open, None)
        }
        TradeWindow::Open if trading && !window_open => {
            manager.close(WindowId::Bag(0));
            (TradeWindow::Cancelling, Some(TradeAction::Cancel))
        }
        TradeWindow::Open | TradeWindow::Cancelling if !trading => {
            manager.close(WindowId::Trade);
            manager.close(WindowId::Bag(0));
            (TradeWindow::Closed, None)
        }
        unchanged => (unchanged, None),
    }
}

fn sync_trade_window(
    mut manager: ResMut<WindowManager>,
    mut trade: ResMut<TradeClientState>,
    mut window: Local<TradeWindow>,
) {
    let trading = trade.is_open();
    let window_open = manager.is_open(WindowId::Trade);
    let (next, action) = step_trade_window(*window, trading, window_open, &mut manager);
    *window = next;
    if let Some(action) = action {
        trade.queue(action);
    }
}

#[cfg(test)]
mod tests;
