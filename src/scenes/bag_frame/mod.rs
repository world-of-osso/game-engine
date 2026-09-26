use bevy::prelude::*;
use game_engine::bag_data::InventoryState;
use game_engine::bank_data::{BankRequest, BankState, GuildBankRequest, GuildBankState};
use game_engine::mail_data::{MailState, MailTab};
use game_engine::merchant_data::{MerchantRequest, MerchantState, MerchantTab};
use game_engine::trade::{TradeAction, TradeClientState};
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::bag_frame_component::{
    ACTION_BAG_TOGGLE_PREFIX, BagContainerState, BagFrameState, BagSlotState, bag_frame_screen,
    parse_bag_slot_action,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::sound::{UiSoundKind, UiSoundQueue, queue_ui_sound};
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};

struct BagFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for BagFrameRes {}
unsafe impl Sync for BagFrameRes {}

#[derive(Resource)]
struct BagFrameWrap(BagFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct BagFrameModel(BagFrameState);

pub struct BagFramePlugin;

impl Plugin for BagFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryState>()
            .init_resource::<MerchantState>()
            .init_resource::<BankState>()
            .init_resource::<GuildBankState>()
            .init_resource::<TradeClientState>()
            .init_resource::<MailState>()
            .add_message::<MerchantRequest>()
            .add_message::<BankRequest>()
            .add_message::<GuildBankRequest>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_bag_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_bag_frame_ui);
        app.add_systems(
            Update,
            (toggle_bag_frame, use_bag_item, sync_bag_frame_state)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn build_bag_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    inventory: Res<InventoryState>,
    window_manager: Res<WindowManager>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(&inventory, &window_manager);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(bag_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(BagFrameWrap(BagFrameRes { screen, shared }));
    commands.insert_resource(BagFrameModel(state));
}

fn teardown_bag_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<BagFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<BagFrameWrap>();
    commands.remove_resource::<BagFrameModel>();
}

fn sync_bag_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<BagFrameWrap>>,
    mut last_model: Option<ResMut<BagFrameModel>>,
    inventory: Res<InventoryState>,
    window_manager: Res<WindowManager>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(&inventory, &window_manager);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn toggle_bag_frame(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    inventory: Res<InventoryState>,
    mut window_manager: ResMut<WindowManager>,
    mut sounds: Option<ResMut<UiSoundQueue>>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    let _ = apply_bag_toggle_action(
        &action,
        &inventory,
        &mut window_manager,
        sounds.as_deref_mut(),
    );
}

/// NPC frames a right-clicked bag item goes to.
#[derive(bevy::ecs::system::SystemParam)]
struct BagItemTargets<'w> {
    merchant: Res<'w, MerchantState>,
    bank: Res<'w, BankState>,
    guild: Res<'w, GuildBankState>,
}

#[derive(bevy::ecs::system::SystemParam)]
struct BagItemRequests<'w> {
    merchant: MessageWriter<'w, MerchantRequest>,
    bank: MessageWriter<'w, BankRequest>,
    guild: MessageWriter<'w, GuildBankRequest>,
    trade: ResMut<'w, TradeClientState>,
    mail: ResMut<'w, MailState>,
}

impl BagItemRequests<'_> {
    fn any_open(&self, targets: &BagItemTargets) -> bool {
        targets.merchant.is_open()
            || targets.bank.is_open()
            || targets.guild.is_open()
            || self.trade.is_open()
            || self.mail.is_open()
    }
}

/// Retail `ContainerFrameItemButton_OnClick` (`C_Container.UseContainerItem`):
/// right-clicking a bag item while the merchant tab is shown sells it; while a bank
/// frame is open it deposits it into the shown bank tab; with a trade open it is
/// offered; with Send Mail shown it is attached. Other right-click uses are not built.
fn use_bag_item(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    inventory: Res<InventoryState>,
    targets: BagItemTargets,
    mut requests: BagItemRequests,
) {
    if !requests.any_open(&targets)
        || !crate::networking::gameplay_input_allowed(reconnect)
        || modal_open.is_some()
    {
        return;
    }
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    if let Some(request) = sell_request(&action, &inventory, &targets.merchant) {
        requests.merchant.write(request);
    } else if let Some(request) = bank_deposit_request(&action, &inventory, &targets.bank) {
        requests.bank.write(request);
    } else if let Some(request) = guild_deposit_request(&action, &inventory, &targets.guild) {
        requests.guild.write(request);
    } else if let Some(offer) = trade_offer(&action, &inventory, &requests.trade) {
        requests.trade.queue(offer);
    } else {
        attach_to_mail(&action, &inventory, &mut requests.mail);
    }
}

/// With the trade window open the item goes to the first free traded slot (Retail
/// `UseContainerItem` → `ClickTradeButton`), the whole stack.
fn trade_offer(
    action: &str,
    inventory: &InventoryState,
    trade: &TradeClientState,
) -> Option<TradeAction> {
    if !trade.is_open() {
        return None;
    }
    let (bag, slot) = parse_bag_slot_action(action)?;
    let item = inventory
        .slot(bag, slot)
        .filter(|item| item.item_guid != 0)?;
    if trade.offers(item.item_guid) {
        return None;
    }
    Some(TradeAction::SetItem(shared::protocol::SetTradeItem {
        slot: trade.first_free_slot()?,
        item_guid: item.item_guid,
        stack_count: u16::try_from(item.count.max(1)).unwrap_or(u16::MAX),
    }))
}

/// With Send Mail shown the item goes on the next attachment button (Retail
/// `UseContainerItem` → `ClickSendMailItemButton`).
fn attach_to_mail(action: &str, inventory: &InventoryState, mail: &mut MailState) -> bool {
    if !mail.is_open() || mail.tab != MailTab::Send {
        return false;
    }
    clicked_item_guid(action, inventory).is_some_and(|guid| mail.attach(guid))
}

fn clicked_item_guid(action: &str, inventory: &InventoryState) -> Option<u64> {
    let (bag, slot) = parse_bag_slot_action(action)?;
    Some(inventory.slot(bag, slot)?.item_guid).filter(|guid| *guid != 0)
}

/// Deposit into the shown bank's selected tab (Retail `UseContainerItem` with the
/// bank open); nothing while its purchase prompt shows.
fn bank_deposit_request(
    action: &str,
    inventory: &InventoryState,
    bank: &BankState,
) -> Option<BankRequest> {
    if !bank.is_open() || bank.shows_purchase_prompt() {
        return None;
    }
    Some(BankRequest::Deposit {
        bank: bank.shown,
        tab: bank.selected_tab(bank.shown) as u8,
        item_guid: clicked_item_guid(action, inventory)?,
    })
}

fn guild_deposit_request(
    action: &str,
    inventory: &InventoryState,
    guild: &GuildBankState,
) -> Option<GuildBankRequest> {
    let purchased = guild.contents.as_ref().map_or(0, |c| c.tabs.len());
    if !guild.is_open() || guild.tab >= purchased {
        return None;
    }
    Some(GuildBankRequest::Deposit {
        tab: guild.tab as u8,
        item_guid: clicked_item_guid(action, inventory)?,
    })
}

fn sell_request(
    action: &str,
    inventory: &InventoryState,
    merchant: &MerchantState,
) -> Option<MerchantRequest> {
    if !merchant.is_open() || merchant.tab != MerchantTab::Merchant {
        return None;
    }
    Some(MerchantRequest::Sell {
        item_guid: clicked_item_guid(action, inventory)?,
        count: 0,
    })
}

fn build_state(inventory: &InventoryState, window_manager: &WindowManager) -> BagFrameState {
    BagFrameState {
        bags: inventory
            .bags
            .iter()
            .map(|bag| BagContainerState {
                bag_index: bag.index,
                title: bag.name.clone(),
                slots: inventory
                    .slots
                    .get(bag.index)
                    .into_iter()
                    .flatten()
                    .map(|slot| BagSlotState {
                        icon_fdid: slot.icon_fdid,
                        count: slot.count,
                        quality_border: slot.quality.border_color().into(),
                    })
                    .collect(),
                visible: window_manager.is_open(WindowId::Bag(bag.index)),
            })
            .collect(),
    }
}

fn apply_bag_toggle_action(
    action: &str,
    inventory: &InventoryState,
    window_manager: &mut WindowManager,
    sounds: Option<&mut UiSoundQueue>,
) -> bool {
    let Some(bag_index) = parse_bag_toggle_action(action) else {
        return false;
    };
    if !inventory.bags.iter().any(|bag| bag.index == bag_index) {
        return false;
    }

    let opened = window_manager.toggle(WindowId::Bag(bag_index));
    if let Some(sounds) = sounds {
        let sound = if opened {
            UiSoundKind::BagOpen
        } else {
            UiSoundKind::BagClose
        };
        queue_ui_sound(sounds, sound);
    }
    true
}

fn parse_bag_toggle_action(action: &str) -> Option<usize> {
    action.strip_prefix(ACTION_BAG_TOGGLE_PREFIX)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linen_bags() -> InventoryState {
        let mut inventory = InventoryState::default();
        inventory.set_item(
            0,
            3,
            InventorySlot {
                icon_fdid: 132_889,
                count: 20,
                item_guid: 41,
                item_id: 2589,
                ..Default::default()
            },
        );
        inventory
    }

    #[test]
    fn right_clicking_a_bag_item_offers_the_stack_in_the_first_free_trade_slot() {
        use shared::protocol::{
            SetTradeItem, TradeItemSnapshot, TradePartySnapshot, TradePhase, TradeSnapshot,
        };
        let inventory = linen_bags();
        let party = |slots| TradePartySnapshot {
            name: "Tradea".into(),
            accepted: false,
            gold: 0,
            slots,
        };
        let mut slots = vec![None; 7];
        slots[0] = Some(TradeItemSnapshot {
            item_guid: 7,
            item_id: 2770,
            name: "Copper Ore".into(),
            quality: 1,
            stack_count: 10,
        });
        let mut trade = TradeClientState::default();
        assert_eq!(trade_offer("bag_slot:0:3", &inventory, &trade), None);
        trade.snapshot = Some(TradeSnapshot {
            phase: TradePhase::Open,
            player: party(slots),
            other: party(vec![None; 7]),
        });
        assert_eq!(
            trade_offer("bag_slot:0:3", &inventory, &trade),
            Some(TradeAction::SetItem(SetTradeItem {
                slot: 1,
                item_guid: 41,
                stack_count: 20
            }))
        );
        assert_eq!(trade_offer("bag_slot:0:4", &inventory, &trade), None);
    }

    #[test]
    fn right_clicking_a_bag_item_attaches_it_only_on_send_mail() {
        let inventory = linen_bags();
        let mut mail = MailState::default();
        mail.open(9);
        assert!(!attach_to_mail("bag_slot:0:3", &inventory, &mut mail));
        mail.tab = MailTab::Send;
        assert!(attach_to_mail("bag_slot:0:3", &inventory, &mut mail));
        assert!(!attach_to_mail("bag_slot:0:3", &inventory, &mut mail));
        assert_eq!(mail.attachments, vec![41]);
    }

    #[test]
    fn right_clicking_a_bag_item_sells_the_stack_only_on_the_merchant_tab() {
        let mut inventory = InventoryState::default();
        inventory.set_item(
            0,
            3,
            InventorySlot {
                icon_fdid: 132_889,
                count: 20,
                item_guid: 41,
                item_id: 2589,
                ..Default::default()
            },
        );
        let mut merchant = MerchantState::default();
        assert_eq!(sell_request("bag_slot:0:3", &inventory, &merchant), None);

        merchant.npc = Some(0x0000_0001_0000_04BD);
        assert_eq!(
            sell_request("bag_slot:0:3", &inventory, &merchant),
            Some(MerchantRequest::Sell {
                item_guid: 41,
                count: 0
            })
        );
        assert_eq!(sell_request("bag_slot:0:4", &inventory, &merchant), None);
        assert_eq!(sell_request("bag_toggle:0", &inventory, &merchant), None);

        merchant.set_tab(MerchantTab::Buyback);
        assert_eq!(sell_request("bag_slot:0:3", &inventory, &merchant), None);
    }
    #[test]
    fn right_clicking_a_bag_item_deposits_into_the_open_bank_tab() {
        use shared::protocol::{BankContents, BankTabView, BankType};
        let mut inventory = InventoryState::default();
        inventory.set_item(
            0,
            3,
            InventorySlot {
                icon_fdid: 132_889,
                count: 20,
                item_guid: 41,
                item_id: 2589,
                ..Default::default()
            },
        );
        let mut bank = BankState::default();
        assert_eq!(
            bank_deposit_request("bag_slot:0:3", &inventory, &bank),
            None
        );
        bank.open(0x0000_0001_0000_0990);
        let tab = BankTabView {
            name: "Tab 1".into(),
            icon: 134_400,
            deposit_flags: 0,
            slots: vec![None; 98],
        };
        bank.apply(BankContents {
            bank: BankType::Account,
            tabs: vec![tab.clone(), tab],
            next_tab_cost: None,
            money: Some(0),
        });
        bank.show(BankType::Account);
        bank.select_tab(1);
        assert_eq!(
            bank_deposit_request("bag_slot:0:3", &inventory, &bank),
            Some(BankRequest::Deposit {
                bank: BankType::Account,
                tab: 1,
                item_guid: 41
            })
        );
        assert_eq!(
            bank_deposit_request("bag_slot:0:4", &inventory, &bank),
            None
        );
        // The character bank has no tab yet: its purchase prompt shows.
        bank.apply(BankContents {
            bank: BankType::Character,
            tabs: Vec::new(),
            next_tab_cost: Some(10_000),
            money: None,
        });
        bank.show(BankType::Character);
        assert_eq!(
            bank_deposit_request("bag_slot:0:3", &inventory, &bank),
            None
        );

        let mut guild = GuildBankState::default();
        assert_eq!(
            guild_deposit_request("bag_slot:0:3", &inventory, &guild),
            None
        );
        guild.open(9);
        guild.apply(shared::protocol::GuildBankContents {
            object: 9,
            guild_name: "Bank Testers".into(),
            tabs: Vec::new(),
            money: 0,
            withdraw_money_remaining: None,
            next_tab_cost: Some(1_000_000),
            is_leader: true,
        });
        assert_eq!(
            guild_deposit_request("bag_slot:0:3", &inventory, &guild),
            None
        );
    }

    use game_engine::bag_data::{BagInfo, InventorySlot, ItemQuality};

    #[test]
    fn parse_bag_toggle_action_extracts_index() {
        assert_eq!(parse_bag_toggle_action("bag_toggle:0"), Some(0));
        assert_eq!(parse_bag_toggle_action("bag_toggle:4"), Some(4));
        assert_eq!(parse_bag_toggle_action("bag_toggle:nope"), None);
        assert_eq!(parse_bag_toggle_action("guild_toggle"), None);
    }

    #[test]
    fn apply_bag_toggle_action_toggles_open_state_and_queues_sounds() {
        let inventory = InventoryState::default();
        let mut window_manager = WindowManager::default();
        let mut sounds = UiSoundQueue::default();

        assert!(apply_bag_toggle_action(
            "bag_toggle:0",
            &inventory,
            &mut window_manager,
            Some(&mut sounds)
        ));
        assert!(window_manager.is_open(WindowId::Bag(0)));
        assert!(apply_bag_toggle_action(
            "bag_toggle:0",
            &inventory,
            &mut window_manager,
            Some(&mut sounds)
        ));
        assert!(!window_manager.is_open(WindowId::Bag(0)));
        assert_eq!(
            sounds.queued_kinds(),
            vec![UiSoundKind::BagOpen, UiSoundKind::BagClose]
        );
    }

    #[test]
    fn build_state_maps_inventory_slots_and_visibility() {
        let inventory = InventoryState {
            bags: vec![BagInfo {
                index: 0,
                name: "Backpack".into(),
                size: 2,
                icon_fdid: 123,
            }],
            slots: vec![vec![
                InventorySlot {
                    icon_fdid: 11,
                    count: 3,
                    quality: ItemQuality::Rare,
                    name: "Potion".into(),
                    ..Default::default()
                },
                InventorySlot::default(),
            ]],
            ..Default::default()
        };
        let mut window_manager = WindowManager::default();
        window_manager.open(WindowId::Bag(0));

        let state = build_state(&inventory, &window_manager);

        assert_eq!(state.bags.len(), 1);
        assert_eq!(state.bags[0].title, "Backpack");
        assert!(state.bags[0].visible);
        assert_eq!(state.bags[0].slots[0].icon_fdid, 11);
        assert_eq!(state.bags[0].slots[0].count, 3);
        assert_eq!(state.bags[0].slots[0].quality_border, "0.0,0.57,0.95,1.0");
    }
}
