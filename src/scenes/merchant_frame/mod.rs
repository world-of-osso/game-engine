//! MerchantFrame scene: builds the Retail frame from [`MerchantState`], the player's
//! money and repair cost; opens and closes its window with the vendor interaction;
//! turns clicks into [`MerchantRequest`]s. Retail clicks (MerchantFrame.lua:632-669):
//! right-click an item buys one purchase, a click on a buyback item buys it back;
//! a left click picks a vendor item up onto the cursor (`scenes::cursor_item`).
//! Sell All Junk asks first (`SELL_ALL_JUNK_ITEMS_POPUP`).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::bag_data::{InventoryState, ItemQuality};
use game_engine::item_icons::item_icon_fdid;
use game_engine::merchant_data::{
    BUYBACK_ITEMS_PER_PAGE, MerchantRequest, MerchantState, MerchantTab, quality_color,
};
use game_engine::status::{CharacterStatsSnapshot, DurabilityStatusSnapshot};
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use game_engine::ui::screens::merchant_frame_component::{
    ACTION_BUYBACK_LAST, ACTION_CLOSE, ACTION_ITEM_PREFIX, ACTION_PAGE_NEXT, ACTION_PAGE_PREV,
    ACTION_REPAIR_ALL, ACTION_SELL_ALL_JUNK, ACTION_TAB_BUYBACK, ACTION_TAB_MERCHANT, CellTint,
    MerchantCell, MerchantFrameState, merchant_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking_quests::NpcInteractionRequest;
use crate::scenes::static_popup::StaticPopupSystems;
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};

/// `INV_Misc_QuestionMark`, Retail's icon for an item without one.
const UNKNOWN_ICON_FDID: u32 = 134_400;
/// `StaticPopup_ShowCustomGenericConfirmation` for Sell All Junk (MF.lua:1054-1063).
const SELL_ALL_JUNK_POPUP: &str = "SELL_ALL_JUNK_ITEMS";

struct MerchantFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for MerchantFrameRes {}
unsafe impl Sync for MerchantFrameRes {}

#[derive(Resource)]
struct MerchantFrameWrap(MerchantFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct MerchantFrameModel(MerchantFrameState);

pub struct MerchantFramePlugin;

impl Plugin for MerchantFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MerchantState>()
            .init_resource::<InventoryState>()
            .init_resource::<PopupStack>()
            .add_message::<MerchantRequest>()
            .add_message::<PopupResult>()
            .add_message::<NpcInteractionRequest>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_merchant_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_merchant_frame_ui);
        app.add_systems(
            Update,
            (
                handle_merchant_frame_input,
                sell_junk_on_confirm.after(StaticPopupSystems),
                sync_merchant_window,
                sync_merchant_frame_state,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

/// Everything the frame shows.
#[derive(SystemParam)]
struct MerchantView<'w> {
    merchant: Res<'w, MerchantState>,
    manager: Res<'w, WindowManager>,
    stats: Option<Res<'w, CharacterStatsSnapshot>>,
    durability: Option<Res<'w, DurabilityStatusSnapshot>>,
    inventory: Res<'w, InventoryState>,
}

impl MerchantView<'_> {
    fn state(&self) -> MerchantFrameState {
        let money = self.stats.as_ref().map_or(0, |stats| stats.gold);
        let repair_cost = self
            .durability
            .as_ref()
            .map_or(0, |durability| durability.total_repair_cost);
        MerchantFrameState {
            has_junk: has_junk(&self.inventory),
            ..build_state(
                &self.merchant,
                self.manager.is_open(WindowId::Merchant),
                money,
                u64::from(repair_cost),
            )
        }
    }
}

/// `C_MerchantFrame.GetNumJunkItems() > 0`: a poor-quality bag item a vendor buys
/// (the items `SellAllJunkItems` sells).
fn has_junk(inventory: &InventoryState) -> bool {
    inventory.slots.iter().flatten().any(|item| {
        item.quality == ItemQuality::Poor
            && game_engine::item_catalog::item_catalog_entry(item.item_id)
                .is_some_and(|entry| entry.sell_price > 0)
    })
}

fn build_merchant_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    view: MerchantView,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = view.state();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(merchant_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(MerchantFrameWrap(MerchantFrameRes { screen, shared }));
    commands.insert_resource(MerchantFrameModel(state));
}

fn teardown_merchant_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<MerchantFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<MerchantFrameWrap>();
    commands.remove_resource::<MerchantFrameModel>();
}

fn sync_merchant_frame_state(
    mut ui: ResMut<UiState>,
    wrap: Option<ResMut<MerchantFrameWrap>>,
    last_model: Option<ResMut<MerchantFrameModel>>,
    view: MerchantView,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap, last_model) else {
        return;
    };
    let state = view.state();
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

/// A vendor list opens the Merchant window and the backpack (`OpenAllBags`,
/// MerchantFrame.lua OnShow); the window manager closing it (close button, Escape,
/// eviction) ends the interaction with `CloseInteraction`; the server ending it
/// closes the window. Either way the backpack closes (`CloseAllBags`, OnHide).
fn sync_merchant_window(
    mut manager: ResMut<WindowManager>,
    mut merchant: ResMut<MerchantState>,
    mut requests: MessageWriter<NpcInteractionRequest>,
    mut open_npc: Local<Option<u64>>,
) {
    let window_open = manager.is_open(WindowId::Merchant);
    if merchant.npc.is_some() && *open_npc != merchant.npc {
        manager.open(WindowId::Merchant);
        manager.open(WindowId::Bag(0));
    } else if let Some(npc) = merchant.npc.filter(|_| !window_open) {
        merchant.close();
        requests.write(NpcInteractionRequest::Close { npc });
        manager.close(WindowId::Bag(0));
    } else if merchant.npc.is_none() && window_open {
        manager.close(WindowId::Merchant);
        manager.close(WindowId::Bag(0));
    }
    *open_npc = merchant.npc;
}

fn build_state(
    merchant: &MerchantState,
    window_open: bool,
    money: u64,
    repair_cost: u64,
) -> MerchantFrameState {
    let buyback_tab = merchant.tab == MerchantTab::Buyback;
    let cells = if buyback_tab {
        buyback_cells(merchant, money)
    } else {
        merchant_cells(merchant, money)
    };
    let paged = !buyback_tab && merchant.page_count() > 1;
    MerchantFrameState {
        visible: window_open && merchant.is_open(),
        title: if buyback_tab {
            "Merchant Buyback".into() // MERCHANT_BUYBACK
        } else {
            merchant.vendor_name.clone()
        },
        buyback_tab,
        cells,
        // MERCHANT_PAGE_NUMBER "Page %s of %s".
        page_text: paged
            .then(|| format!("Page {} of {}", merchant.page + 1, merchant.page_count())),
        prev_enabled: merchant.page > 0,
        next_enabled: merchant.page + 1 < merchant.page_count(),
        // `GetRepairAllCost()` enables Repair All while anything is damaged.
        repair: merchant.can_repair.then_some(repair_cost > 0),
        repair_mode: false,
        last_buyback: merchant.last_buyback().map(|item| MerchantCell {
            action: ACTION_BUYBACK_LAST.into(),
            ..buyback_cell(item, money, 0)
        }),
        money,
        has_junk: false,
    }
}

fn merchant_cells(merchant: &MerchantState, money: u64) -> Vec<MerchantCell> {
    merchant
        .page_items()
        .iter()
        .enumerate()
        .map(|(index, item)| MerchantCell {
            name: item.name.clone(),
            name_color: quality_color(item.quality),
            icon_fdid: item_icon_fdid(item.item_id).unwrap_or(UNKNOWN_ICON_FDID),
            count: item.stack_count,
            stock: item.num_available,
            price: u64::from(item.price),
            price_gray: money < u64::from(item.price),
            tint: if item.usable {
                CellTint::Normal
            } else {
                CellTint::Unusable
            },
            action: format!("{ACTION_ITEM_PREFIX}{index}"),
        })
        .collect()
}

fn buyback_cells(merchant: &MerchantState, money: u64) -> Vec<MerchantCell> {
    merchant
        .buyback
        .iter()
        .take(BUYBACK_ITEMS_PER_PAGE)
        .enumerate()
        .map(|(index, item)| buyback_cell(item, money, index))
        .collect()
}

fn buyback_cell(item: &shared::protocol::BuybackItem, money: u64, index: usize) -> MerchantCell {
    MerchantCell {
        name: item.name.clone(),
        name_color: quality_color(item.quality),
        icon_fdid: item_icon_fdid(item.item_id).unwrap_or(UNKNOWN_ICON_FDID),
        count: item.count,
        stock: None,
        price: u64::from(item.price),
        price_gray: money < u64::from(item.price),
        tint: CellTint::Normal,
        action: format!("{ACTION_ITEM_PREFIX}{index}"),
    }
}

#[derive(SystemParam)]
struct Pointer<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    reconnect: Option<Res<'w, crate::networking::ReconnectState>>,
    modal_open: Option<Res<'w, crate::scenes::game_menu::UiModalOpen>>,
}

impl Pointer<'_, '_> {
    /// The button pressed this frame and the `onclick` action under the cursor.
    fn click(self, ui: &UiState) -> Option<(MouseButton, String)> {
        let mouse = self.mouse.as_ref()?;
        let button = [MouseButton::Left, MouseButton::Right]
            .into_iter()
            .find(|button| mouse.just_pressed(*button))?;
        if self.modal_open.is_some() || !crate::networking::gameplay_input_allowed(self.reconnect) {
            return None;
        }
        let window = self.windows.single().ok()?;
        let cursor = ui_cursor_position(&ui.registry, window)?;
        let frame_id = find_frame_at(&ui.registry, cursor.x, cursor.y)?;
        Some((button, walk_up_for_onclick(&ui.registry, frame_id)?))
    }
}

fn handle_merchant_frame_input(
    pointer: Pointer,
    ui: Res<UiState>,
    mut merchant: ResMut<MerchantState>,
    mut manager: ResMut<WindowManager>,
    mut requests: MessageWriter<MerchantRequest>,
    mut popups: ResMut<PopupStack>,
) {
    if !merchant.is_open() {
        return;
    }
    let Some((button, action)) = pointer.click(&ui) else {
        return;
    };
    if action == ACTION_SELL_ALL_JUNK {
        popups.push(sell_all_junk_popup());
        return;
    }
    if let Some(request) = dispatch_action(&action, button, &mut merchant, &mut manager) {
        requests.write(request);
    }
}

/// `SELL_ALL_JUNK_ITEMS_POPUP` with the generic confirmation's Yes / No.
fn sell_all_junk_popup() -> PopupSpec {
    PopupSpec {
        key: SELL_ALL_JUNK_POPUP.into(),
        text: "You are about to sell all junk items and will not be able to buy them back.\nAre you sure you want to proceed?".into(),
        accept_label: "Yes".into(),
        cancel_label: Some("No".into()),
        timeout: None,
        confirm_text: None,
    }
}

/// `MerchantFrame_OnSellAllJunkButtonConfirmed` → `C_MerchantFrame.SellAllJunkItems`.
fn sell_junk_on_confirm(
    mut results: MessageReader<PopupResult>,
    merchant: Res<MerchantState>,
    mut requests: MessageWriter<MerchantRequest>,
) {
    for result in results.read() {
        if result.key == SELL_ALL_JUNK_POPUP
            && result.outcome == PopupOutcome::Accepted
            && merchant.is_open()
        {
            requests.write(MerchantRequest::SellAllJunk);
        }
    }
}

/// Apply a frame click; returns the request it sends to the server.
fn dispatch_action(
    action: &str,
    button: MouseButton,
    merchant: &mut MerchantState,
    manager: &mut WindowManager,
) -> Option<MerchantRequest> {
    match action {
        ACTION_CLOSE => {
            manager.close(WindowId::Merchant);
        }
        ACTION_PAGE_PREV => merchant.prev_page(),
        ACTION_PAGE_NEXT => merchant.next_page(),
        ACTION_TAB_MERCHANT => merchant.set_tab(MerchantTab::Merchant),
        ACTION_TAB_BUYBACK => merchant.set_tab(MerchantTab::Buyback),
        ACTION_REPAIR_ALL => return Some(MerchantRequest::Repair { item_guid: None }),
        ACTION_BUYBACK_LAST => {
            let slot = merchant.last_buyback()?.slot;
            return Some(MerchantRequest::Buyback { slot });
        }
        _ => {
            let index: usize = action.strip_prefix(ACTION_ITEM_PREFIX)?.parse().ok()?;
            return item_request(merchant, index, button);
        }
    }
    None
}

/// Right-click buys (`BuyMerchantItem`); any click on a buyback item buys it back
/// (`BuybackItem`). A left click on a vendor item picks it up (the cursor item).
fn item_request(
    merchant: &MerchantState,
    index: usize,
    button: MouseButton,
) -> Option<MerchantRequest> {
    match merchant.tab {
        MerchantTab::Merchant if button != MouseButton::Right => None,
        MerchantTab::Merchant => {
            let item = merchant.page_items().get(index)?;
            Some(MerchantRequest::Buy {
                slot: item.slot,
                item_id: item.item_id,
                count: 1,
                destination: None,
            })
        }
        MerchantTab::Buyback => Some(MerchantRequest::Buyback {
            slot: merchant.buyback.get(index)?.slot,
        }),
    }
}

#[cfg(test)]
mod tests;
