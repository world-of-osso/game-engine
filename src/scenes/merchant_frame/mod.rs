use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::merchant_data::{MerchantIntentQueue, MerchantState, MerchantTabKind};
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::merchant_frame_component::{
    ACTION_BUY_PREFIX, ACTION_BUYBACK_PREFIX, ACTION_CLOSE, ACTION_GUILD_REPAIR, ACTION_PAGE_NEXT,
    ACTION_PAGE_PREV, ACTION_REPAIR_ALL, ACTION_TAB_PREFIX, MerchantFrameState, MerchantItem,
    MerchantTab, merchant_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;
use crate::ui_input::walk_up_for_onclick;

const SELL_TAB_EMPTY_TEXT: &str = "Sell items from your bags to populate buyback.";
const BUYBACK_TAB_EMPTY_TEXT: &str = "No items available for buyback.";
const BUY_TAB_EMPTY_TEXT: &str = "This vendor has no items for sale.";

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
        app.init_resource::<MerchantState>();
        app.init_resource::<MerchantIntentQueue>();
        app.add_systems(OnEnter(GameState::InWorld), build_merchant_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_merchant_frame_ui);
        app.add_systems(
            Update,
            (sync_merchant_frame_state, handle_merchant_frame_input)
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_merchant_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    merchant: Res<MerchantState>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(&merchant);
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
    mut wrap: Option<ResMut<MerchantFrameWrap>>,
    mut last_model: Option<ResMut<MerchantFrameModel>>,
    merchant: Res<MerchantState>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(&merchant);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn handle_merchant_frame_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    mut merchant: ResMut<MerchantState>,
    mut intents: ResMut<MerchantIntentQueue>,
) {
    if !merchant.is_open()
        || !crate::networking::gameplay_input_allowed(reconnect)
        || modal_open.is_some()
    {
        return;
    }
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    dispatch_action(&action, &mut merchant, &mut intents);
}

fn build_state(merchant: &MerchantState) -> MerchantFrameState {
    MerchantFrameState {
        visible: merchant.is_open(),
        tabs: build_tabs(merchant.current_tab()),
        items: build_items(merchant),
        page: merchant.page + 1,
        total_pages: merchant.page_count(),
        player_money: merchant.player_money.display(),
        empty_text: build_empty_text(merchant),
    }
}

fn build_tabs(active_tab: MerchantTabKind) -> Vec<MerchantTab> {
    vec![
        build_tab("Buy", MerchantTabKind::Buy, active_tab),
        build_tab("Sell", MerchantTabKind::Sell, active_tab),
        build_tab("Buyback", MerchantTabKind::Buyback, active_tab),
    ]
}

fn build_tab(name: &str, tab: MerchantTabKind, active_tab: MerchantTabKind) -> MerchantTab {
    MerchantTab {
        name: name.into(),
        active: tab == active_tab,
        action: format!("{ACTION_TAB_PREFIX}{}", tab_token(tab)),
    }
}

fn build_items(merchant: &MerchantState) -> Vec<MerchantItem> {
    match merchant.current_tab() {
        MerchantTabKind::Buy => merchant
            .current_page_items()
            .iter()
            .map(|item| MerchantItem {
                name: item.name.clone(),
                price: item.buy_price.display_short(),
                icon_fdid: item.icon_fdid,
                action: format!("{ACTION_BUY_PREFIX}{}", item.item_id),
            })
            .collect(),
        MerchantTabKind::Sell => Vec::new(),
        MerchantTabKind::Buyback => merchant
            .current_page_buyback_items()
            .iter()
            .map(|item| MerchantItem {
                name: item.name.clone(),
                price: item.buyback_price.display_short(),
                icon_fdid: item.icon_fdid,
                action: format!("{ACTION_BUYBACK_PREFIX}{}", item.slot),
            })
            .collect(),
    }
}

fn build_empty_text(merchant: &MerchantState) -> Option<String> {
    if !merchant.is_open() {
        return None;
    }
    let has_items = match merchant.current_tab() {
        MerchantTabKind::Buy => !merchant.inventory.is_empty(),
        MerchantTabKind::Sell => false,
        MerchantTabKind::Buyback => !merchant.buyback_inventory.is_empty(),
    };
    if has_items {
        return None;
    }
    Some(
        match merchant.current_tab() {
            MerchantTabKind::Buy => BUY_TAB_EMPTY_TEXT,
            MerchantTabKind::Sell => SELL_TAB_EMPTY_TEXT,
            MerchantTabKind::Buyback => BUYBACK_TAB_EMPTY_TEXT,
        }
        .into(),
    )
}

fn dispatch_action(action: &str, merchant: &mut MerchantState, intents: &mut MerchantIntentQueue) {
    if action == ACTION_CLOSE {
        merchant.close();
        return;
    }
    if action == ACTION_PAGE_PREV {
        merchant.prev_page();
        return;
    }
    if action == ACTION_PAGE_NEXT {
        merchant.next_page();
        return;
    }
    if action == ACTION_REPAIR_ALL {
        intents.repair_all();
        return;
    }
    if action == ACTION_GUILD_REPAIR {
        return;
    }
    if let Some(tab) = parse_tab_action(action) {
        merchant.set_tab(tab);
        return;
    }
    if let Some(item_id) = parse_u32_action(action, ACTION_BUY_PREFIX) {
        intents.buy(item_id, 1);
        return;
    }
    if let Some(slot) = parse_u8_action(action, ACTION_BUYBACK_PREFIX) {
        intents.buyback(slot);
    }
}

fn parse_tab_action(action: &str) -> Option<MerchantTabKind> {
    let token = action.strip_prefix(ACTION_TAB_PREFIX)?;
    match token {
        "buy" => Some(MerchantTabKind::Buy),
        "sell" => Some(MerchantTabKind::Sell),
        "buyback" => Some(MerchantTabKind::Buyback),
        _ => None,
    }
}

fn parse_u32_action(action: &str, prefix: &str) -> Option<u32> {
    action.strip_prefix(prefix)?.parse().ok()
}

fn parse_u8_action(action: &str, prefix: &str) -> Option<u8> {
    action.strip_prefix(prefix)?.parse().ok()
}

fn tab_token(tab: MerchantTabKind) -> &'static str {
    match tab {
        MerchantTabKind::Buy => "buy",
        MerchantTabKind::Sell => "sell",
        MerchantTabKind::Buyback => "buyback",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::auction_house_data::Money;
    use game_engine::merchant_data::{MerchantBuybackItemDef, MerchantItemDef};

    fn merchant_state() -> MerchantState {
        MerchantState {
            npc_entity_id: Some(77),
            inventory: vec![MerchantItemDef {
                item_id: 100,
                name: "Arrow".into(),
                icon_fdid: 0,
                buy_price: Money(10),
                sell_price: Money(2),
                max_stack: 200,
            }],
            buyback_inventory: vec![MerchantBuybackItemDef {
                slot: 3,
                item_id: 700,
                name: "Bent Sword".into(),
                icon_fdid: 0,
                buyback_price: Money(2500),
            }],
            player_money: Money(5000),
            repair_cost: Money(300),
            items_per_page: 10,
            active_tab: MerchantTabKind::Buy,
            ..Default::default()
        }
    }

    #[test]
    fn build_state_uses_buyback_items_for_buyback_tab() {
        let mut merchant = merchant_state();
        merchant.set_tab(MerchantTabKind::Buyback);

        let state = build_state(&merchant);

        assert!(state.visible);
        assert!(state.tabs[2].active);
        assert_eq!(state.items.len(), 1);
        assert_eq!(state.items[0].name, "Bent Sword");
        assert_eq!(state.items[0].price, "25s");
        assert_eq!(state.items[0].action, "merchant_buyback:3");
    }

    #[test]
    fn build_state_shows_buyback_empty_text() {
        let mut merchant = merchant_state();
        merchant.buyback_inventory.clear();
        merchant.set_tab(MerchantTabKind::Buyback);

        let state = build_state(&merchant);

        assert_eq!(state.empty_text.as_deref(), Some(BUYBACK_TAB_EMPTY_TEXT));
    }

    #[test]
    fn dispatch_action_switches_to_buyback_tab() {
        let mut merchant = merchant_state();
        let mut intents = MerchantIntentQueue::default();

        dispatch_action("merchant_tab:buyback", &mut merchant, &mut intents);

        assert_eq!(merchant.current_tab(), MerchantTabKind::Buyback);
        assert_eq!(merchant.page, 0);
    }

    #[test]
    fn dispatch_action_queues_buyback_purchase() {
        let mut merchant = merchant_state();
        let mut intents = MerchantIntentQueue::default();

        dispatch_action("merchant_buyback:3", &mut merchant, &mut intents);

        assert_eq!(intents.pending.len(), 1);
        assert!(matches!(
            intents.pending[0],
            game_engine::merchant_data::MerchantIntent::Buyback { slot: 3 }
        ));
    }
}
