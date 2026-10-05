//! Portable vendor session for the native host, ported from the Bevy client's
//! `scenes/merchant_frame`, `scenes/bag_frame`, `scenes/cursor_item/stack_split_frame` and
//! `networking/merchant`: the server's vendor list, buyback list, bags and money drive the
//! shared MerchantFrame, backpack and StackSplitFrame screens, and frame clicks become the
//! requests the server expects (docs/specs/merchant-frame.md).

use shared::protocol::{
    BuybackItem, BuybackList, InventoryDelta, InventorySnapshot, ItemLocation, VendorInventory,
};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::bag_data::{InventoryState, ItemQuality};
use crate::bag_frame_component::{
    BagContainerState, BagFrameState, BagSlotState, bag_frame_screen, parse_bag_slot_action,
};
use crate::item_icons::item_icon_fdid;
use crate::merchant_data::{
    BUYBACK_ITEMS_PER_PAGE, MerchantRequest, MerchantState, MerchantTab, quality_color,
};
use crate::merchant_frame_component::{
    ACTION_BUYBACK_LAST, ACTION_CLOSE, ACTION_GUILD_REPAIR, ACTION_ITEM_PREFIX, ACTION_PAGE_NEXT,
    ACTION_PAGE_PREV, ACTION_REPAIR_ALL, ACTION_REPAIR_ITEM, ACTION_SELL_ALL_JUNK,
    ACTION_TAB_BUYBACK, ACTION_TAB_MERCHANT, CellTint, MerchantCell, MerchantFrameState,
    merchant_frame_screen,
};
use crate::stack_split::{StackSplitOwner, StackSplitState};
use crate::stack_split_frame_component::{
    ACTION_CANCEL, ACTION_LEFT, ACTION_OKAY, ACTION_RIGHT, FRAME_W as SPLIT_W,
    StackSplitFrameState, stack_split_frame_screen,
};

/// `INV_Misc_QuestionMark`, Retail's icon for an item without one.
const UNKNOWN_ICON_FDID: u32 = 134_400;
/// `MERCHANT_BUYBACK`.
const MERCHANT_BUYBACK: &str = "Merchant Buyback";
/// Bevy window manager container slot: bags stack up from 16 px right and 96 px
/// above the bottom (clearing the micro menu and bags bar).
const CONTAINER_RIGHT: f32 = 16.0;
const CONTAINER_BOTTOM: f32 = 96.0;

/// A mouse click on a frame action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Click {
    pub right: bool,
    pub shift: bool,
}

impl Click {
    pub const LEFT: Self = Self {
        right: false,
        shift: false,
    };
    pub const RIGHT: Self = Self {
        right: true,
        shift: false,
    };
    pub const SHIFT_LEFT: Self = Self {
        right: false,
        shift: true,
    };
}

/// What the host sends for a frame input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MerchantEffect {
    /// A vendor request to the open vendor `npc`.
    Request { npc: u64, request: MerchantRequest },
    /// `CloseInteraction { npc }`: the player closed the frame.
    CloseInteraction { npc: u64 },
}

/// `StackSplitFrame` keys (`OnKeyDown` / `OnChar`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitKey {
    Digit(u32),
    Backspace,
    Decrement,
    Increment,
    Enter,
    Escape,
}

/// The vendor frame, the backpack it sells from, the player's money and repair cost.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MerchantSession {
    pub merchant: MerchantState,
    pub inventory: InventoryState,
    /// Copper, from the local player's replicated `Gold`.
    pub money: u64,
    /// `GetRepairAllCost`, from `DurabilityStateUpdate`.
    pub repair_cost: u32,
    pub split: Option<StackSplitState>,
    /// `InRepairMode()`: `MerchantRepairItemButton` shows the repair cursor (MF.xml:305-313).
    pub repair_mode: bool,
}

impl MerchantSession {
    pub fn is_open(&self) -> bool {
        self.merchant.is_open()
    }

    /// `MerchantFrame:SetPortraitToUnit("npc")` (MerchantFrame.lua:269): the open vendor,
    /// whose portrait fills `MerchantFramePortrait`.
    pub fn portrait_unit(&self) -> Option<u64> {
        self.merchant.npc.filter(|_| self.is_open())
    }

    /// `VendorInventory`: opens the frame (and the backpack) or refreshes its stock.
    pub fn receive_inventory(&mut self, inventory: VendorInventory, vendor_name: String) {
        self.merchant.apply_inventory(inventory, vendor_name);
    }

    pub fn receive_buyback(&mut self, list: BuybackList) {
        self.merchant.buyback = list.items;
    }

    pub fn receive_inventory_snapshot(&mut self, snapshot: &InventorySnapshot) {
        self.inventory.apply_snapshot(snapshot);
        self.drop_stale_split();
    }

    pub fn receive_inventory_delta(&mut self, delta: &InventoryDelta) {
        self.inventory.apply_delta(delta);
        self.drop_stale_split();
    }

    /// `InteractionClosed` for the open vendor closes it without a request.
    pub fn receive_interaction_closed(&mut self, npc: u64) {
        if self.merchant.npc == Some(npc) {
            self.close_frame();
        }
    }

    /// The close button or Escape: close the frame and end the interaction.
    pub fn close(&mut self) -> Option<MerchantEffect> {
        let npc = self.merchant.npc?;
        self.close_frame();
        Some(MerchantEffect::CloseInteraction { npc })
    }

    /// The frame closing resets the cursor (`ResetCursor`, MF.lua:167).
    fn close_frame(&mut self) {
        self.merchant.close();
        self.split = None;
        self.repair_mode = false;
    }

    /// A click on a MerchantFrame or StackSplitFrame action (MerchantFrame.lua:632-693).
    pub fn click_frame(&mut self, action: &str, click: Click) -> Option<MerchantEffect> {
        let npc = self.merchant.npc?;
        if self.split.is_some() {
            return self.click_with_split(action, click);
        }
        if action == ACTION_CLOSE {
            return self.close();
        }
        let request = match action.strip_prefix(ACTION_ITEM_PREFIX) {
            Some(index) => self.item_click(index.parse().ok()?, click),
            None => self.frame_button(action),
        };
        request.map(|request| MerchantEffect::Request { npc, request })
    }

    /// With the split frame open its buttons act; any other click closes it
    /// (`StackSplitFrame:Hide`) and then acts.
    fn click_with_split(&mut self, action: &str, click: Click) -> Option<MerchantEffect> {
        let split = self.split.as_mut()?;
        match split_click(split, action) {
            Some(outcome) => self.finish_split(outcome),
            None if !action.starts_with("stack_split:") => {
                self.split = None;
                self.click_frame(action, click)
            }
            None => None,
        }
    }

    /// Paging, tabs, direct merchant services and the last-sale buyback slot.
    fn frame_button(&mut self, action: &str) -> Option<MerchantRequest> {
        match action {
            ACTION_PAGE_PREV => self.merchant.prev_page(),
            ACTION_PAGE_NEXT => self.merchant.next_page(),
            ACTION_TAB_MERCHANT => self.merchant.set_tab(MerchantTab::Merchant),
            ACTION_TAB_BUYBACK => self.merchant.set_tab(MerchantTab::Buyback),
            ACTION_REPAIR_ALL => return Some(MerchantRequest::Repair { item_guid: None }),
            // `if CanGuildBankRepair() then RepairAllItems(true)` (MF.xml:368-375).
            ACTION_GUILD_REPAIR if self.merchant.guild_repair_money.is_some() => {
                return Some(MerchantRequest::GuildRepairAll);
            }
            // `ShowRepairCursor` / `HideRepairCursor`.
            ACTION_REPAIR_ITEM if self.merchant.can_repair => self.repair_mode = !self.repair_mode,
            ACTION_SELL_ALL_JUNK
                if self.merchant.tab == MerchantTab::Merchant && has_junk(&self.inventory) =>
            {
                return Some(MerchantRequest::SellAllJunk);
            }
            ACTION_BUYBACK_LAST => {
                let slot = self.merchant.last_buyback()?.slot;
                return Some(MerchantRequest::Buyback { slot });
            }
            _ => {}
        }
        None
    }

    /// Right-click buys one purchase (`BuyMerchantItem`); Shift-click opens the split
    /// frame for a stacking item; any click on a buyback cell buys it back.
    fn item_click(&mut self, index: usize, click: Click) -> Option<MerchantRequest> {
        match self.merchant.tab {
            MerchantTab::Merchant if click.shift => {
                self.split = vendor_split(&self.merchant, index, self.money);
                None
            }
            MerchantTab::Merchant if !click.right => None,
            MerchantTab::Merchant => {
                let item = self.merchant.page_items().get(index)?;
                Some(MerchantRequest::Buy {
                    slot: item.slot,
                    item_id: item.item_id,
                    count: 1,
                    destination: None,
                })
            }
            MerchantTab::Buyback => Some(MerchantRequest::Buyback {
                slot: self.merchant.buyback.get(index)?.slot,
            }),
        }
    }

    /// A left click on an item while the repair cursor is shown repairs it
    /// (`PickupContainerItem` / `PickupInventoryItem` in repair mode → `RepairItem`);
    /// `None` outside repair mode, where the click belongs to the cursor item.
    pub fn repair_click(&mut self, location: ItemLocation) -> Option<Option<MerchantEffect>> {
        if !self.repair_mode {
            return None;
        }
        let npc = self.merchant.npc?;
        Some(
            self.inventory
                .item_at(location)
                .map(|item| MerchantEffect::Request {
                    npc,
                    request: MerchantRequest::Repair {
                        item_guid: Some(item.item_guid),
                    },
                }),
        )
    }

    /// Right-clicking a bag item on the merchant tab sells the stack
    /// (`ContainerFrameItemButton_OnClick` → `UseContainerItem`).
    pub fn click_bag(&mut self, action: &str, click: Click) -> Option<MerchantEffect> {
        let npc = self.merchant.npc?;
        self.split = None;
        if !click.right || self.merchant.tab != MerchantTab::Merchant {
            return None;
        }
        let (bag, slot) = parse_bag_slot_action(action)?;
        let item = self
            .inventory
            .slot(bag, slot)
            .filter(|item| !item.is_empty())?;
        Some(MerchantEffect::Request {
            npc,
            request: MerchantRequest::Sell {
                item_guid: item.item_guid,
                count: 0,
            },
        })
    }

    /// A key while the split frame is open; `None` when it is closed (the key is not its).
    pub fn split_key(&mut self, key: SplitKey) -> Option<Option<MerchantEffect>> {
        let split = self.split.as_mut()?;
        let outcome = match key {
            SplitKey::Enter => Some(SplitOutcome::Okay),
            SplitKey::Escape => Some(SplitOutcome::Cancel),
            SplitKey::Backspace => {
                split.backspace();
                None
            }
            SplitKey::Decrement => {
                split.decrement();
                None
            }
            SplitKey::Increment => {
                split.increment();
                None
            }
            SplitKey::Digit(digit) => {
                split.type_digit(digit);
                None
            }
        };
        Some(outcome.and_then(|outcome| self.finish_split(outcome)))
    }

    /// `StackSplitOkayButton_OnClick` → `BuyMerchantItem(index, split)`.
    fn finish_split(&mut self, outcome: SplitOutcome) -> Option<MerchantEffect> {
        let split = self.split.take()?;
        let npc = self.merchant.npc?;
        if outcome == SplitOutcome::Cancel {
            return None;
        }
        let StackSplitOwner::Merchant(index) = split.owner else {
            return None;
        };
        let item = self.merchant.page_items().get(index)?;
        Some(MerchantEffect::Request {
            npc,
            request: MerchantRequest::Buy {
                slot: item.slot,
                item_id: item.item_id,
                count: split.split / split.min_split,
                destination: None,
            },
        })
    }

    fn drop_stale_split(&mut self) {
        if let Some(StackSplitOwner::Bag(location)) = self.split.as_ref().map(|split| split.owner)
            && self
                .inventory
                .item_at(location)
                .is_none_or(|item| item.count < 2)
        {
            self.split = None;
        }
    }

    /// Everything the MerchantFrame shows.
    pub fn frame_state(&self) -> MerchantFrameState {
        MerchantFrameState {
            has_junk: has_junk(&self.inventory),
            repair_mode: self.repair_mode,
            ..build_frame_state(&self.merchant, self.money, u64::from(self.repair_cost))
        }
    }

    /// The backpack opens with the vendor (`OpenAllBags`) and closes with it.
    pub fn bag_state(&self) -> BagFrameState {
        let open = self.is_open();
        BagFrameState {
            bags: self
                .inventory
                .bags
                .iter()
                .map(|bag| BagContainerState {
                    bag_index: bag.index,
                    title: bag.name.clone(),
                    portrait_fdid: self.bag_portrait(bag.index),
                    slots: self
                        .inventory
                        .slots
                        .get(bag.index)
                        .into_iter()
                        .flatten()
                        .map(|slot| BagSlotState {
                            icon_fdid: slot.icon_fdid,
                            count: slot.count,
                            quality_border: slot.quality.border_color().into(),
                            locked: false,
                        })
                        .collect(),
                    visible: open && bag.index == 0,
                })
                .collect(),
        }
    }

    /// `UpdateMiscellaneousFrames` (ContainerFrame.lua:823-829): the backpack's own
    /// icon, an equipped bag's item icon.
    fn bag_portrait(&self, bag: usize) -> u32 {
        if bag == 0 {
            return crate::bag_frame_component::BACKPACK_PORTRAIT;
        }
        u8::try_from(bag)
            .ok()
            .and_then(shared::protocol::EquipmentSlot::from_bag_index)
            .and_then(|slot| self.inventory.equipped(slot))
            .map_or(0, |item| item.icon_fdid)
    }

    /// The split frame, BOTTOMLEFT on its vendor cell's TOPLEFT.
    pub fn split_state(&self, registry: &FrameRegistry) -> StackSplitFrameState {
        let Some(split) = &self.split else {
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
        let owner = match split.owner {
            StackSplitOwner::Merchant(index) => format!("MerchantItem{}", index + 1),
            StackSplitOwner::Bag(_) => return state,
        };
        if let Some(rect) = registry
            .get_by_name(&owner)
            .and_then(|id| registry.get(id)?.layout_rect.clone())
        {
            state.x = rect.x;
            state.y = rect.y - state.height();
        }
        state
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SplitOutcome {
    Okay,
    Cancel,
}

fn split_click(split: &mut StackSplitState, action: &str) -> Option<SplitOutcome> {
    match action {
        ACTION_LEFT => split.decrement(),
        ACTION_RIGHT => split.increment(),
        ACTION_OKAY => return Some(SplitOutcome::Okay),
        ACTION_CANCEL => return Some(SplitOutcome::Cancel),
        _ => {}
    }
    None
}

/// `MerchantItemButton_OnModifiedClick` (MF.lua:660-693): a stacking vendor item opens
/// the split frame capped at `min(maxStack, floor(money / (price / stackCount)))`,
/// stepping by the purchase size.
fn vendor_split(merchant: &MerchantState, index: usize, money: u64) -> Option<StackSplitState> {
    let item = merchant.page_items().get(index)?;
    let stack = item.stack_count.max(1);
    if item.max_stack <= 1 {
        return None;
    }
    let affordable = match item.price {
        0 => item.max_stack,
        price => u32::try_from(money * u64::from(stack) / u64::from(price)).unwrap_or(u32::MAX),
    };
    let max = item.max_stack.min(affordable);
    (max >= stack)
        .then(|| StackSplitState::open(StackSplitOwner::Merchant(index), max, stack))
        .flatten()
}

/// The `PlaySound` of a MerchantFrame button that fired: the page buttons
/// (MF.lua:574, 581), Repair All (MF.xml:256) and the guild bank repair (MF.xml:372).
/// Disabled buttons have no action.
pub fn click_sound(action: &str) -> Option<u32> {
    use game_engine_core::ui_sound_kits::{IG_MAINMENU_OPTION_CHECKBOX_ON, ITEM_REPAIR};
    match action {
        ACTION_PAGE_PREV | ACTION_PAGE_NEXT => Some(IG_MAINMENU_OPTION_CHECKBOX_ON),
        ACTION_REPAIR_ALL | ACTION_GUILD_REPAIR => Some(ITEM_REPAIR),
        _ => None,
    }
}

/// `C_MerchantFrame.GetNumJunkItems() > 0`: a poor bag item a vendor buys.
fn has_junk(inventory: &InventoryState) -> bool {
    inventory.slots.iter().flatten().any(|item| {
        item.quality == ItemQuality::Poor
            && crate::item_catalog::item_catalog_entry(item.item_id)
                .is_some_and(|entry| entry.sell_price > 0)
    })
}

fn build_frame_state(merchant: &MerchantState, money: u64, repair_cost: u64) -> MerchantFrameState {
    let buyback_tab = merchant.tab == MerchantTab::Buyback;
    let paged = !buyback_tab && merchant.page_count() > 1;
    MerchantFrameState {
        visible: merchant.is_open(),
        title: if buyback_tab {
            MERCHANT_BUYBACK.into()
        } else {
            merchant.vendor_name.clone()
        },
        buyback_tab,
        cells: tab_cells(merchant, money),
        // MERCHANT_PAGE_NUMBER "Page %s of %s".
        page_text: paged
            .then(|| format!("Page {} of {}", merchant.page + 1, merchant.page_count())),
        prev_enabled: merchant.page > 0,
        next_enabled: merchant.page + 1 < merchant.page_count(),
        // `GetRepairAllCost()` enables Repair All while anything is damaged.
        repair: merchant.can_repair.then_some(repair_cost > 0),
        repair_mode: false,
        // `CanMerchantRepair() and CanGuildBankRepair()` (MF.lua:935-946).
        guild_repair: merchant.can_repair && merchant.guild_repair_money.is_some(),
        last_buyback: merchant.last_buyback().map(|item| MerchantCell {
            action: ACTION_BUYBACK_LAST.into(),
            ..buyback_cell(item, money, 0)
        }),
        money,
        has_junk: false,
    }
}

fn tab_cells(merchant: &MerchantState, money: u64) -> Vec<MerchantCell> {
    match merchant.tab {
        MerchantTab::Merchant => merchant_cells(merchant, money),
        MerchantTab::Buyback => merchant
            .buyback
            .iter()
            .take(BUYBACK_ITEMS_PER_PAGE)
            .enumerate()
            .map(|(index, item)| buyback_cell(item, money, index))
            .collect(),
    }
}

fn icon(item_id: u32) -> u32 {
    item_icon_fdid(item_id).unwrap_or(UNKNOWN_ICON_FDID)
}

fn merchant_cells(merchant: &MerchantState, money: u64) -> Vec<MerchantCell> {
    merchant
        .page_items()
        .iter()
        .enumerate()
        .map(|(index, item)| MerchantCell {
            name: item.name.clone(),
            name_color: quality_color(item.quality),
            icon_fdid: icon(item.item_id),
            count: item.stack_count,
            stock: item.num_available,
            price: u64::from(item.price),
            // `canAfford == false` greys the price (MF.lua:316).
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

fn buyback_cell(item: &BuybackItem, money: u64, index: usize) -> MerchantCell {
    MerchantCell {
        name: item.name.clone(),
        name_color: quality_color(item.quality),
        icon_fdid: icon(item.item_id),
        count: item.count,
        stock: None,
        price: u64::from(item.price),
        price_gray: money < u64::from(item.price),
        tint: CellTint::Normal,
        action: format!("{ACTION_ITEM_PREFIX}{index}"),
    }
}

/// The MerchantFrame, backpack and StackSplitFrame in one screen.
pub fn merchant_screen(ctx: &SharedContext) -> Element {
    let mut elements = merchant_frame_screen(ctx);
    elements.extend(bag_frame_screen(ctx));
    elements.extend(stack_split_frame_screen(ctx));
    elements
}

/// Window placement after each rebuild: the backpack in the container slot at the
/// bottom right (the Bevy window manager's `container_positions`). Also blends the
/// repair-item `HighlightTexture` additively (`alphaMode="ADD"`, MF.xml:316).
pub fn place_merchant_windows(registry: &mut FrameRegistry) {
    if let Some(id) = registry.get_by_name("MerchantRepairItemButtonHighlight")
        && let Some(frame) = registry.get_mut(id)
        && let Some(ui_toolkit::frame::WidgetData::Texture(texture)) = &mut frame.widget_data
    {
        texture.blend_mode = ui_toolkit::widgets::texture::BlendMode::Additive;
    }
    let (width, height) = (registry.screen_width, registry.screen_height);
    let Some(id) = registry.get_by_name("ContainerFrame0") else {
        return;
    };
    let Some((bag_w, bag_h)) = registry.get(id).map(|frame| {
        let fixed = |dimension| match dimension {
            ui_toolkit::frame::Dimension::Fixed(value) => value,
            _ => 0.0,
        };
        (fixed(frame.width), fixed(frame.height))
    }) else {
        return;
    };
    let x = width - CONTAINER_RIGHT - bag_w;
    let y = height - CONTAINER_BOTTOM - bag_h;
    let _ = registry.set_pos(id, x, y);
}

/// Width of the split frame, for hosts clamping it on screen.
pub const STACK_SPLIT_WIDTH: f32 = SPLIT_W;
