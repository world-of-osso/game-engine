//! `StackSplitFrame`: opened by a Shift-click on a bag stack
//! (`ContainerFrameItemButtonMixin:OnModifiedClick`, ContainerFrame.lua:1608-1616)
//! or a vendor item (`MerchantItemButton_OnModifiedClick`, MerchantFrame.lua:660-693);
//! its arrows, digits and Okay pick the amount. Okay puts the split on the cursor
//! (`SplitContainerItem`) or buys it (`BuyMerchantItem(index, split)`).

use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use game_engine::bag_data::InventoryState;
use game_engine::cursor_item::{CursorItem, CursorTarget};
use game_engine::merchant_data::{MerchantRequest, MerchantState, MerchantTab};
use game_engine::stack_split::{StackSplit, StackSplitOwner, StackSplitState};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::stack_split_frame_component::{
    ACTION_CANCEL, ACTION_LEFT, ACTION_OKAY, ACTION_RIGHT, FRAME_W, StackSplitFrameState,
    stack_split_frame_screen,
};
use shared::protocol::ItemLocation;
use ui_toolkit::screen::{Screen, SharedContext};

use super::Pointer;
use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;

pub(super) struct StackSplitFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for StackSplitFrameRes {}
unsafe impl Sync for StackSplitFrameRes {}

#[derive(Resource)]
pub(super) struct StackSplitFrameWrap(StackSplitFrameRes);

#[derive(Resource, Clone, PartialEq)]
pub(super) struct StackSplitFrameModel(StackSplitFrameState);

pub(super) fn register(app: &mut App) {
    app.add_systems(
        OnEnter(GameState::InWorld),
        build_stack_split_frame_ui.run_if(inworld_scene_stage_allows_ui),
    );
    app.add_systems(OnExit(GameState::InWorld), teardown_stack_split_frame_ui);
}

/// What a Shift-click on `target` opens, if anything: a bag stack of more than one
/// item, or a vendor item that stacks, capped by what `money` affords.
pub fn split_request(
    target: CursorTarget,
    inventory: &InventoryState,
    merchant: &MerchantState,
    money: u64,
) -> Option<StackSplitState> {
    match target {
        CursorTarget::Location(location @ ItemLocation::Bag { .. }) => {
            let count = inventory.item_at(location)?.count;
            (count > 1)
                .then(|| StackSplitState::open(StackSplitOwner::Bag(location), count, 1))
                .flatten()
        }
        CursorTarget::MerchantItem(index) if merchant.tab == MerchantTab::Merchant => {
            let item = merchant.page_items().get(index)?;
            let stack = item.stack_count.max(1);
            if item.max_stack <= 1 {
                return None;
            }
            // canAfford = floor(GetMoney() / (price / stackCount)).
            let affordable = match item.price {
                0 => item.max_stack,
                price => {
                    u32::try_from(money * u64::from(stack) / u64::from(price)).unwrap_or(u32::MAX)
                }
            };
            let max = item.max_stack.min(affordable);
            (max >= stack)
                .then(|| StackSplitState::open(StackSplitOwner::Merchant(index), max, stack))
                .flatten()
        }
        _ => None,
    }
}

/// Clicks on the frame's buttons and its keys (`OnKeyDown`, `OnChar`).
pub(super) fn handle_stack_split_input(
    pointer: Pointer,
    ui: Res<UiState>,
    mut keys: MessageReader<KeyboardInput>,
    mut split: ResMut<StackSplit>,
    mut cursor: ResMut<CursorItem>,
    inventory: Res<InventoryState>,
    merchant: Res<MerchantState>,
    mut buys: MessageWriter<MerchantRequest>,
) {
    let Some(state) = split.0.as_mut() else {
        keys.clear();
        return;
    };
    let mut action = None;
    if pointer
        .mouse
        .as_ref()
        .is_some_and(|mouse| mouse.just_pressed(MouseButton::Left))
        && let Some(hit) = pointer.hit(&ui)
    {
        action = hit.action.flatten();
    }
    let mut outcome = action.as_deref().and_then(|action| click(state, action));
    for event in keys.read() {
        if event.state == ButtonState::Pressed && outcome.is_none() {
            outcome = key(state, event.key_code);
        }
    }
    let Some(outcome) = outcome else { return };
    let state = split.0.take().expect("split frame open");
    if outcome == Outcome::Okay {
        okay(&state, &inventory, &merchant, &mut cursor, &mut buys);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Okay,
    Cancel,
}

fn click(state: &mut StackSplitState, action: &str) -> Option<Outcome> {
    match action {
        ACTION_LEFT => state.decrement(),
        ACTION_RIGHT => state.increment(),
        ACTION_OKAY => return Some(Outcome::Okay),
        ACTION_CANCEL => return Some(Outcome::Cancel),
        _ => {}
    }
    None
}

fn key(state: &mut StackSplitState, key: KeyCode) -> Option<Outcome> {
    match key {
        KeyCode::Enter | KeyCode::NumpadEnter => return Some(Outcome::Okay),
        KeyCode::Escape => return Some(Outcome::Cancel),
        KeyCode::Backspace | KeyCode::Delete => state.backspace(),
        KeyCode::ArrowLeft | KeyCode::ArrowDown => state.decrement(),
        KeyCode::ArrowRight | KeyCode::ArrowUp => state.increment(),
        other => {
            if let Some(digit) = digit(other) {
                state.type_digit(digit);
            }
        }
    }
    None
}

fn digit(key: KeyCode) -> Option<u32> {
    use KeyCode::*;
    [
        (Digit0, Numpad0),
        (Digit1, Numpad1),
        (Digit2, Numpad2),
        (Digit3, Numpad3),
        (Digit4, Numpad4),
        (Digit5, Numpad5),
        (Digit6, Numpad6),
        (Digit7, Numpad7),
        (Digit8, Numpad8),
        (Digit9, Numpad9),
    ]
    .iter()
    .position(|(digit, numpad)| key == *digit || key == *numpad)
    .map(|digit| digit as u32)
}

/// `StackSplitOkayButton_OnClick` → the owner's `SplitStack(split)`.
fn okay(
    state: &StackSplitState,
    inventory: &InventoryState,
    merchant: &MerchantState,
    cursor: &mut CursorItem,
    buys: &mut MessageWriter<MerchantRequest>,
) {
    match state.owner {
        StackSplitOwner::Bag(location) => {
            if cursor.is_empty() {
                *cursor = CursorItem::split_from(inventory, location, state.split);
            }
        }
        StackSplitOwner::Merchant(index) => {
            if let Some(item) = merchant.page_items().get(index) {
                buys.write(MerchantRequest::Buy {
                    slot: item.slot,
                    item_id: item.item_id,
                    count: state.split / state.min_split,
                    destination: None,
                });
            }
        }
    }
}

fn build_stack_split_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = StackSplitFrameState::default();
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(stack_split_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(StackSplitFrameWrap(StackSplitFrameRes { screen, shared }));
    commands.insert_resource(StackSplitFrameModel(state));
}

fn teardown_stack_split_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<StackSplitFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<StackSplitFrameWrap>();
    commands.remove_resource::<StackSplitFrameModel>();
}

/// The frame follows [`StackSplit`]; it closes when its owner stack is gone.
pub(super) fn sync_stack_split_frame(
    mut ui: ResMut<UiState>,
    wrap: Option<ResMut<StackSplitFrameWrap>>,
    last_model: Option<ResMut<StackSplitFrameModel>>,
    mut split: ResMut<StackSplit>,
    inventory: Res<InventoryState>,
) {
    if let Some(StackSplitOwner::Bag(location)) = split.0.as_ref().map(|state| state.owner)
        && inventory
            .item_at(location)
            .is_none_or(|item| item.count < 2)
    {
        split.0 = None;
    }
    let (Some(mut wrap), Some(mut last_model)) = (wrap, last_model) else {
        return;
    };
    let state = frame_state(split.0.as_ref(), &ui.registry);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

/// Anchored to its owner: BOTTOMRIGHT on a bag slot's TOPRIGHT, BOTTOMLEFT on a
/// vendor item's TOPLEFT.
fn frame_state(split: Option<&StackSplitState>, registry: &FrameRegistry) -> StackSplitFrameState {
    let Some(split) = split else {
        return StackSplitFrameState::default();
    };
    let mut state = StackSplitFrameState {
        visible: true,
        x: 0.0,
        y: 0.0,
        text: split.text(),
        total_text: split.total_text(),
        left_enabled: split.left_enabled(),
        right_enabled: split.right_enabled(),
    };
    let (owner, right_aligned) = match split.owner {
        StackSplitOwner::Bag(ItemLocation::Bag { bag, slot }) => {
            (format!("ContainerFrame{bag}Slot{slot}"), true)
        }
        StackSplitOwner::Bag(ItemLocation::Equipment(_)) => return state,
        StackSplitOwner::Merchant(index) => (format!("MerchantItem{}", index + 1), false),
    };
    if let Some(rect) = registry
        .get_by_name(&owner)
        .and_then(|id| registry.get(id)?.layout_rect.clone())
    {
        state.x = if right_aligned {
            rect.x + rect.width - FRAME_W
        } else {
            rect.x
        };
        state.y = rect.y - state.height();
    }
    state
}
