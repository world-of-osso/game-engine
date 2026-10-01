//! The cursor item: left clicks on bag slots, paperdoll slots, the merchant frame
//! and the world drive [`CursorItem`] (Retail `ContainerFrameItemButton_OnClick`,
//! `PaperDollItemSlotButton_OnClick`, `MerchantItemButton_OnClick`, WorldFrame
//! drops raising `DELETE_ITEM_CONFIRM`); a drag released over another target drops
//! there too (`OnDragStart` / `OnReceiveDrag`). Shift-clicking a stack or a vendor
//! item opens the [`StackSplit`] frame. The picked-up item's icon follows the
//! pointer.

mod stack_split_frame;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::bag_data::{InventoryRequest, InventoryState};
use game_engine::cursor_item::{CursorEffect, CursorItem, CursorTarget, DestroyConfirm};
use game_engine::merchant_data::{MerchantRequest, MerchantState};
use game_engine::stack_split::StackSplit;
use game_engine::status::CharacterStatsSnapshot;
use game_engine::ui::frame::{Dimension, WidgetData, WidgetType};
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::UiState;
use game_engine::ui::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::bag_frame_component::parse_bag_slot_action;
use game_engine::ui::screens::character_frame_component::parse_equipment_slot_action;
use game_engine::ui::screens::merchant_frame_component::{ACTION_FRAME, ACTION_ITEM_PREFIX};
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::texture::{TextureData, TextureSource};
use shared::protocol::ItemLocation;

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::scenes::static_popup::StaticPopupSystems;
use crate::ui_input::walk_up_for_onclick;

pub use stack_split_frame::split_request;

use game_engine::ui::screens::cursor_item_component::{CURSOR_ICON_NAME, CURSOR_ICON_SIZE};
/// Pointer travel (UI px) that turns a press into a drag.
const DRAG_THRESHOLD: f32 = 4.0;
/// `StaticPopupDialogs["DELETE_ITEM"]` / `["DELETE_GOOD_ITEM"]`.
const DELETE_ITEM: &str = "DELETE_ITEM";
const DELETE_GOOD_ITEM: &str = "DELETE_GOOD_ITEM";

pub struct CursorItemPlugin;

impl Plugin for CursorItemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorItem>()
            .init_resource::<StackSplit>()
            .init_resource::<InventoryState>()
            .init_resource::<MerchantState>()
            .init_resource::<PopupStack>()
            .add_message::<InventoryRequest>()
            .add_message::<MerchantRequest>()
            .add_message::<PopupResult>();
        stack_split_frame::register(app);
        app.add_systems(OnExit(GameState::InWorld), reset_cursor);
        app.add_systems(
            Update,
            (
                clear_stale_cursor,
                stack_split_frame::handle_stack_split_input,
                handle_cursor_clicks,
                resolve_destroy_popup.after(StaticPopupSystems),
                stack_split_frame::sync_stack_split_frame,
                sync_cursor_icon,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn reset_cursor(mut cursor: ResMut<CursorItem>, mut split: ResMut<StackSplit>) {
    *cursor = CursorItem::Empty;
    split.0 = None;
}

fn clear_stale_cursor(
    mut cursor: ResMut<CursorItem>,
    inventory: Res<InventoryState>,
    merchant: Res<MerchantState>,
) {
    if !cursor.is_empty() {
        cursor.clear_if_stale(&inventory, &merchant);
    }
}

/// The frame click the cursor item reacts to: what was under the pointer.
#[derive(Clone, Debug, PartialEq)]
struct Hit {
    at: Vec2,
    /// `None` when no frame is under the pointer (the world).
    action: Option<Option<String>>,
}

impl Hit {
    fn target(&self) -> Option<CursorTarget> {
        match &self.action {
            None => Some(CursorTarget::World),
            Some(action) => cursor_target(action.as_deref()?),
        }
    }

    fn is_stack_split(&self) -> bool {
        self.action
            .as_ref()
            .and_then(|action| action.as_deref())
            .is_some_and(|action| action.starts_with("stack_split:"))
    }
}

/// The cursor target a frame's click action names.
fn cursor_target(action: &str) -> Option<CursorTarget> {
    if let Some((bag, slot)) = parse_bag_slot_action(action) {
        return Some(CursorTarget::Location(ItemLocation::Bag {
            bag: u8::try_from(bag).ok()?,
            slot: u8::try_from(slot).ok()?,
        }));
    }
    if let Some(slot) = parse_equipment_slot_action(action) {
        return Some(CursorTarget::Location(ItemLocation::Equipment(slot)));
    }
    if let Some(index) = action.strip_prefix(ACTION_ITEM_PREFIX) {
        return index.parse().ok().map(CursorTarget::MerchantItem);
    }
    (action == ACTION_FRAME).then_some(CursorTarget::MerchantFrame)
}

#[derive(SystemParam)]
struct Pointer<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    keys: Option<Res<'w, ButtonInput<KeyCode>>>,
    reconnect: Option<Res<'w, crate::networking::ReconnectState>>,
    modal_open: Option<Res<'w, crate::scenes::game_menu::UiModalOpen>>,
}

impl Pointer<'_, '_> {
    fn allowed(&self) -> bool {
        self.modal_open.is_none()
            && self
                .reconnect
                .as_ref()
                .is_none_or(|reconnect| !reconnect.is_active())
    }

    fn shift(&self) -> bool {
        self.keys
            .as_ref()
            .is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]))
    }

    fn hit(&self, ui: &UiState) -> Option<Hit> {
        let window = self.windows.single().ok()?;
        let at = ui_cursor_position(&ui.registry, window)?;
        let action = find_frame_at(&ui.registry, at.x, at.y)
            .map(|frame| walk_up_for_onclick(&ui.registry, frame).filter(|a| !a.is_empty()));
        Some(Hit { at, action })
    }
}

/// A left press that picked the item up, remembered until its release.
#[derive(Default)]
struct Press {
    picked_at: Option<(Vec2, Option<CursorTarget>)>,
}

#[derive(SystemParam)]
struct CursorWriters<'w> {
    inventory: MessageWriter<'w, InventoryRequest>,
    merchant: MessageWriter<'w, MerchantRequest>,
    popups: ResMut<'w, PopupStack>,
}

impl CursorWriters<'_> {
    fn apply(&mut self, effect: CursorEffect) {
        match effect {
            CursorEffect::Inventory(request) => {
                self.inventory.write(request);
            }
            CursorEffect::Merchant(request) => {
                self.merchant.write(request);
            }
            CursorEffect::ConfirmDestroy(confirm) => {
                self.popups.push(destroy_popup(&confirm));
            }
        }
    }
}

#[derive(SystemParam)]
struct ItemSources<'w> {
    inventory: Res<'w, InventoryState>,
    merchant: Res<'w, MerchantState>,
    stats: Option<Res<'w, CharacterStatsSnapshot>>,
}

fn handle_cursor_clicks(
    pointer: Pointer,
    ui: Res<UiState>,
    sources: ItemSources,
    mut cursor: ResMut<CursorItem>,
    mut split: ResMut<StackSplit>,
    mut writers: CursorWriters,
    mut press: Local<Press>,
) {
    let Some(mouse) = pointer.mouse.as_deref() else {
        return;
    };
    let (pressed, released) = (
        mouse.just_pressed(MouseButton::Left),
        mouse.just_released(MouseButton::Left),
    );
    if !(pressed || released) || !pointer.allowed() {
        return;
    }
    let Some(hit) = pointer.hit(&ui) else { return };
    if hit.is_stack_split() {
        return;
    }
    if pressed {
        // Retail hides the split frame on any other item click.
        split.0 = None;
        let money = sources.stats.as_ref().map_or(0, |stats| stats.gold);
        if pointer.shift() && cursor.is_empty() {
            split.0 = hit.target().and_then(|target| {
                split_request(target, &sources.inventory, &sources.merchant, money)
            });
            return;
        }
        let was_empty = cursor.is_empty();
        let Some(target) = hit.target() else { return };
        if let Some(effect) = cursor.click(target, &sources.inventory, &sources.merchant) {
            writers.apply(effect);
        }
        press.picked_at = (was_empty && !cursor.is_empty()).then_some((hit.at, Some(target)));
    } else if let Some((at, picked_target)) = press.picked_at.take()
        && at.distance(hit.at) >= DRAG_THRESHOLD
        && let Some(target) = hit.target().filter(|target| Some(*target) != picked_target)
        && let Some(effect) = cursor.click(target, &sources.inventory, &sources.merchant)
    {
        // A drag released over another target (`OnReceiveDrag`).
        writers.apply(effect);
    }
}

/// `DELETE_ITEM` ("Do you want to destroy %s?"); Rare and better get
/// `DELETE_GOOD_ITEM`, where Yes waits for `DELETE` typed into its edit box.
fn destroy_popup(confirm: &DestroyConfirm) -> PopupSpec {
    let (key, text) = if confirm.good {
        (
            DELETE_GOOD_ITEM,
            format!(
                "Do you want to destroy {}?\n\nType \"DELETE\" into the field to confirm.",
                confirm.name
            ),
        )
    } else {
        (
            DELETE_ITEM,
            format!("Do you want to destroy {}?", confirm.name),
        )
    };
    PopupSpec {
        key: key.into(),
        text,
        accept_label: "Yes".into(),
        cancel_label: Some("No".into()),
        timeout: None,
        // DELETE_ITEM_CONFIRM_STRING.
        confirm_text: confirm.good.then(|| "DELETE".into()),
    }
}

/// Yes destroys the cursor item (`DeleteCursorItem`), No clears the cursor
/// (`ClearCursor`); the popup hides once the cursor is empty (`OnUpdate`).
fn resolve_destroy_popup(
    mut results: MessageReader<PopupResult>,
    mut cursor: ResMut<CursorItem>,
    mut popups: ResMut<PopupStack>,
    mut requests: MessageWriter<InventoryRequest>,
) {
    for result in results.read() {
        if result.key != DELETE_ITEM && result.key != DELETE_GOOD_ITEM {
            continue;
        }
        match result.outcome {
            PopupOutcome::Accepted => {
                if let Some(request) = cursor.destroy() {
                    requests.write(request);
                }
            }
            PopupOutcome::Cancelled | PopupOutcome::TimedOut => *cursor = CursorItem::Empty,
        }
    }
    if cursor.is_empty() {
        for key in [DELETE_ITEM, DELETE_GOOD_ITEM] {
            if popups.contains(key) {
                popups.hide(key);
            }
        }
    }
}

/// The cursor item's icon under the pointer, above every frame.
fn sync_cursor_icon(
    mut ui: ResMut<UiState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cursor: Res<CursorItem>,
) {
    let existing = ui.registry.get_by_name(CURSOR_ICON_NAME);
    let pointer = windows
        .single()
        .ok()
        .and_then(|window| ui_cursor_position(&ui.registry, window));
    let (Some(fdid), Some(pointer)) = (cursor.icon_fdid(), pointer) else {
        if let Some(id) = existing.filter(|&id| ui.registry.get(id).is_some_and(|f| !f.hidden)) {
            ui.registry.set_hidden(id, true);
        }
        return;
    };
    let id = existing.unwrap_or_else(|| create_cursor_icon(&mut ui.registry));
    if let Some(frame) = ui.registry.get_mut(id) {
        let shown = matches!(
            &frame.widget_data,
            Some(WidgetData::Texture(TextureData { source: TextureSource::FileDataId(shown), .. }))
                if *shown == fdid
        );
        if !shown {
            frame.widget_data = Some(WidgetData::Texture(TextureData {
                source: TextureSource::FileDataId(fdid),
                ..Default::default()
            }));
        }
    }
    let half = CURSOR_ICON_SIZE * 0.5;
    let _ = ui.registry.set_pos(id, pointer.x - half, pointer.y - half);
    ui.registry.set_hidden(id, false);
}

fn create_cursor_icon(registry: &mut FrameRegistry) -> u64 {
    let id = registry.create_frame(CURSOR_ICON_NAME, None);
    if let Some(frame) = registry.get_mut(id) {
        frame.widget_type = WidgetType::Texture;
        frame.width = Dimension::Fixed(CURSOR_ICON_SIZE);
        frame.height = Dimension::Fixed(CURSOR_ICON_SIZE);
        frame.strata = FrameStrata::Tooltip;
        frame.mouse_enabled = false;
    }
    let _ = registry.set_pos_type(id, PositionType::Absolute);
    id
}

/// Escape clears the cursor item before anything else (`ClearCursor`).
pub fn clear_cursor_item(cursor: Option<&mut CursorItem>) -> bool {
    match cursor {
        Some(cursor) if !cursor.is_empty() => {
            *cursor = CursorItem::Empty;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
