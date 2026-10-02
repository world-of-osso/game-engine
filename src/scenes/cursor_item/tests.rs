use bevy::ecs::system::RunSystemOnce;
use bevy::input::keyboard::{Key, KeyboardInput};
use game_engine::bag_data::{InventorySlot, ItemQuality};
use game_engine::stack_split::{StackSplitOwner, StackSplitState};
use game_engine::ui::popup::PopupId;
use shared::protocol::{DestroyItem, EquipmentSlot, VendorInventory, VendorItem};

use super::*;

fn bag(slot: u8) -> ItemLocation {
    ItemLocation::Bag { bag: 0, slot }
}

/// 20 Linen Cloth in backpack slot 0 and a Ruined Pelt in slot 2.
fn inventory() -> InventoryState {
    let mut inventory = InventoryState::default();
    inventory.set_item(
        0,
        0,
        InventorySlot {
            icon_fdid: 132_889,
            count: 20,
            name: "Linen Cloth".into(),
            item_guid: 41,
            item_id: 2589,
            ..Default::default()
        },
    );
    inventory.set_item(
        0,
        2,
        InventorySlot {
            icon_fdid: 134_366,
            count: 1,
            quality: ItemQuality::Poor,
            name: "Ruined Pelt".into(),
            item_guid: 43,
            item_id: 4865,
            ..Default::default()
        },
    );
    inventory
}

/// Innkeeper Farley's water: 5 per purchase at 25 copper, stacks to 20.
fn farley() -> MerchantState {
    let mut merchant = MerchantState::default();
    merchant.apply_inventory(
        VendorInventory {
            npc: 0x0000_0001_0000_04BD,
            can_repair: false,
            guild_repair_money: None,
            items: vec![VendorItem {
                slot: 3,
                item_id: 159,
                name: "Refreshing Spring Water".into(),
                quality: 1,
                price: 25,
                stack_count: 5,
                max_stack: 20,
                num_available: None,
                usable: true,
                max_durability: None,
            }],
        },
        "Innkeeper Farley".into(),
    );
    merchant
}

#[test]
fn click_actions_name_their_cursor_targets() {
    assert_eq!(
        cursor_target("bag_slot:1:7"),
        Some(CursorTarget::Location(ItemLocation::Bag {
            bag: 1,
            slot: 7
        }))
    );
    assert_eq!(
        cursor_target("equipment_slot:15"),
        Some(CursorTarget::Location(ItemLocation::Equipment(
            EquipmentSlot::MainHand
        )))
    );
    assert_eq!(
        cursor_target("merchant_item:3"),
        Some(CursorTarget::MerchantItem(3))
    );
    assert_eq!(
        cursor_target("merchant_frame"),
        Some(CursorTarget::MerchantFrame)
    );
    assert_eq!(cursor_target("merchant_tab:buyback"), None);
    let world = Hit {
        at: Vec2::ZERO,
        action: None,
    };
    assert_eq!(world.target(), Some(CursorTarget::World));
    let plain_frame = Hit {
        at: Vec2::ZERO,
        action: Some(None),
    };
    assert_eq!(plain_frame.target(), None);
}

#[test]
fn shift_clicks_open_the_split_for_stacks_and_affordable_vendor_bundles() {
    let (inventory, merchant) = (inventory(), farley());
    let split = split_request(CursorTarget::Location(bag(0)), &inventory, &merchant, 0).unwrap();
    assert_eq!(
        (split.owner, split.max, split.min_split),
        (StackSplitOwner::Bag(bag(0)), 20, 1)
    );
    assert_eq!(
        split_request(CursorTarget::Location(bag(2)), &inventory, &merchant, 0),
        None
    );

    // 60 copper buys 12 water (25 per 5), capped by the 20-stack at 1 gold.
    let water = split_request(CursorTarget::MerchantItem(0), &inventory, &merchant, 60).unwrap();
    assert_eq!((water.max, water.min_split), (12, 5));
    let rich = split_request(CursorTarget::MerchantItem(0), &inventory, &merchant, 10_000).unwrap();
    assert_eq!(rich.max, 20);
    // 20 copper does not buy one bundle.
    assert_eq!(
        split_request(CursorTarget::MerchantItem(0), &inventory, &merchant, 20),
        None
    );
}

fn stack_split_app(split: StackSplitState) -> App {
    let mut app = App::new();
    app.add_message::<KeyboardInput>()
        .add_message::<MerchantRequest>()
        .insert_resource(UiState {
            registry: FrameRegistry::new(1920.0, 1080.0),
            event_bus: ui_toolkit::event::EventBus::new(),
            focused_frame: None,
        })
        .insert_resource(StackSplit(Some(split)))
        .insert_resource(CursorItem::Empty)
        .insert_resource(inventory())
        .insert_resource(farley());
    app
}

fn press(app: &mut App, key_code: KeyCode) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.world_mut()
        .run_system_once(stack_split_frame::handle_stack_split_input)
        .unwrap();
    // A fresh reader per run would see this key again.
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .clear();
}

#[test]
fn typing_seven_and_enter_puts_seven_linen_on_the_cursor() {
    let split = StackSplitState::open(StackSplitOwner::Bag(bag(0)), 20, 1).unwrap();
    let mut app = stack_split_app(split);
    press(&mut app, KeyCode::Digit7);
    assert_eq!(
        app.world()
            .resource::<StackSplit>()
            .0
            .as_ref()
            .unwrap()
            .split,
        7
    );
    press(&mut app, KeyCode::Enter);

    assert_eq!(app.world().resource::<StackSplit>().0, None);
    assert!(matches!(
        app.world().resource::<CursorItem>(),
        CursorItem::Inventory { from, count: 7, split: true, .. } if *from == bag(0)
    ));
}

#[test]
fn okay_on_a_vendor_bundle_buys_the_chosen_number_of_purchases() {
    let split = StackSplitState::open(StackSplitOwner::Merchant(0), 20, 5).unwrap();
    let mut app = stack_split_app(split);
    press(&mut app, KeyCode::ArrowRight);
    press(&mut app, KeyCode::ArrowRight);
    press(&mut app, KeyCode::NumpadEnter);
    let sent: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<MerchantRequest>>()
        .drain()
        .collect();
    assert_eq!(
        sent,
        vec![MerchantRequest::Buy {
            slot: 3,
            item_id: 159,
            count: 3,
            destination: None
        }]
    );

    let split = StackSplitState::open(StackSplitOwner::Bag(bag(0)), 20, 1).unwrap();
    let mut app = stack_split_app(split);
    press(&mut app, KeyCode::Escape);
    assert_eq!(app.world().resource::<StackSplit>().0, None);
    assert!(app.world().resource::<CursorItem>().is_empty());
}

fn destroy_app(cursor: CursorItem) -> App {
    let mut app = App::new();
    app.add_message::<PopupResult>()
        .add_message::<InventoryRequest>()
        .insert_resource(cursor)
        .insert_resource(PopupStack::default());
    app
}

fn pelt_on_cursor() -> CursorItem {
    let mut cursor = CursorItem::Empty;
    cursor.click(
        CursorTarget::Location(bag(2)),
        &inventory(),
        &MerchantState::default(),
    );
    cursor
}

#[test]
fn yes_on_delete_item_destroys_the_cursor_stack_and_no_clears_the_cursor() {
    let mut app = destroy_app(pelt_on_cursor());
    let id = app
        .world_mut()
        .resource_mut::<PopupStack>()
        .push(destroy_popup(&DestroyConfirm {
            name: "Ruined Pelt".into(),
            good: false,
        }));
    assert_eq!(
        app.world().resource::<PopupStack>().visible()[0].spec.text,
        "Do you want to destroy Ruined Pelt?"
    );
    app.world_mut().write_message(PopupResult {
        id,
        key: DELETE_ITEM.into(),
        outcome: PopupOutcome::Accepted,
    });
    app.world_mut()
        .run_system_once(resolve_destroy_popup)
        .unwrap();
    let sent: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<InventoryRequest>>()
        .drain()
        .collect();
    assert_eq!(
        sent,
        vec![InventoryRequest::Destroy(DestroyItem {
            location: bag(2),
            count: 0
        })]
    );
    assert!(app.world().resource::<CursorItem>().is_empty());

    let mut app = destroy_app(pelt_on_cursor());
    app.world_mut().write_message(PopupResult {
        id: PopupId(1),
        key: DELETE_ITEM.into(),
        outcome: PopupOutcome::Cancelled,
    });
    app.world_mut()
        .run_system_once(resolve_destroy_popup)
        .unwrap();
    assert!(app.world().resource::<CursorItem>().is_empty());
    assert!(
        app.world()
            .resource::<Messages<InventoryRequest>>()
            .is_empty()
    );
}

#[test]
fn the_delete_popup_hides_once_the_cursor_is_cleared() {
    let mut app = destroy_app(CursorItem::Empty);
    app.world_mut()
        .resource_mut::<PopupStack>()
        .push(destroy_popup(&DestroyConfirm {
            name: "Ruined Pelt".into(),
            good: false,
        }));
    app.world_mut()
        .run_system_once(resolve_destroy_popup)
        .unwrap();
    assert!(!app.world().resource::<PopupStack>().contains(DELETE_ITEM));
}

#[test]
fn escape_clears_a_held_item_before_anything_else() {
    let mut cursor = pelt_on_cursor();
    assert!(clear_cursor_item(Some(&mut cursor)));
    assert!(cursor.is_empty());
    assert!(!clear_cursor_item(Some(&mut cursor)));
    assert!(!clear_cursor_item(None));
}
