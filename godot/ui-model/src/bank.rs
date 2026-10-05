//! Portable BankFrame session for the native host, ported from the Bevy client's
//! `scenes/bank_frame` and `networking/bank`: the server's `BankContents` for the open
//! banker drive the shared BankFrame and backpack, and frame and bag clicks become the
//! `BankChannel` requests the server answers (docs/specs/bank-frame.md). Bags, money and
//! bank contents change only through the server's replies.

use std::collections::{BTreeMap, HashMap};

use shared::protocol::{
    BankContents, BankFailed, BankType, InventoryDelta, InventorySnapshot, ItemLocation, ItemStack,
    NpcRole,
};

use crate::bag_data::InventoryState;
use crate::bank_art::{MoneyBoxNames, SlotItem};
use crate::bank_data::{BankPrompt, BankRequest, BankState, money_text};
use crate::bank_frame_component::{
    self as frame, BankFrameState, BankPromptView, MoneyFrameView, PurchasePromptView, SideTab,
};
use crate::merchant::Click;
use crate::popup::PopupSpec;

/// Text of the frame edit boxes, by name.
pub type InputTexts = HashMap<String, String>;

pub const BUY_BANK_TAB_POPUP: &str = "CONFIRM_BUY_BANK_TAB";

/// What the host sends for a frame input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BankEffect {
    /// A bank request to the open banker `npc`.
    Request { npc: u64, request: BankRequest },
    /// `CloseInteraction { npc }`: the player closed the frame.
    CloseInteraction { npc: u64 },
}

fn text<'a>(texts: &'a InputTexts, name: &str) -> &'a str {
    texts.get(name).map_or("", String::as_str)
}

/// Copper typed into a gold / silver / copper entry; empty boxes count as zero.
pub fn money_input(texts: &InputTexts, boxes: MoneyBoxNames) -> u64 {
    let part = |name| text(texts, name).trim().parse::<u64>().unwrap_or(0);
    part(boxes.gold) * 10_000 + part(boxes.silver) * 100 + part(boxes.copper)
}

pub(crate) fn cleared(boxes: MoneyBoxNames) -> Vec<(&'static str, String)> {
    boxes
        .all()
        .into_iter()
        .map(|name| (name, String::new()))
        .collect()
}

pub(crate) fn index(action: &str, prefix: &str) -> Option<usize> {
    action.strip_prefix(prefix)?.parse().ok()
}

/// `CONFIRM_BUY_*_TAB`: Yes / No with the price on its own line.
pub(crate) fn confirm_purchase(key: &str, text: &str, cost: u64) -> PopupSpec {
    PopupSpec {
        key: key.into(),
        text: format!("{text}\n{}", money_text(cost)),
        accept_label: "Yes".into(),
        cancel_label: Some("No".into()),
        timeout: None,
        confirm_text: None,
    }
}

/// Icon FileDataID of an item. The host injects the item-table lookup
/// (`item_icons::item_icon_fdid_for`); the portable default reads no data and shows
/// Retail's `INV_Misc_QuestionMark`, as Retail does for an item it can't resolve.
#[derive(Clone, Copy, Debug)]
pub struct ItemIcons(pub fn(shared::item_data::ItemDefinitionSource, u32) -> Option<u32>);

impl Default for ItemIcons {
    fn default() -> Self {
        Self(|_, _| None)
    }
}

impl ItemIcons {
    pub fn slot_item(self, stack: &ItemStack) -> SlotItem {
        SlotItem {
            icon_fdid: (self.0)(stack.definition_source, stack.item_id)
                .unwrap_or(crate::bank_art::UNKNOWN_ICON),
            count: stack.count,
            quality_border: "1.0,1.0,1.0,1.0".into(),
        }
    }
}

/// The server's bag stacks by container and slot, as `InventorySnapshot` and
/// `InventoryDelta` last described them. What a deposit names by item GUID.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BagStacks(BTreeMap<(u8, u8), ItemStack>);

impl BagStacks {
    pub fn apply_snapshot(&mut self, snapshot: &InventorySnapshot) {
        self.0 = snapshot
            .bags
            .iter()
            .flat_map(|bag| {
                bag.items
                    .iter()
                    .map(|item| ((bag.bag, item.slot), item.item.clone()))
            })
            .collect();
    }

    pub fn apply_delta(&mut self, delta: &InventoryDelta) {
        for change in &delta.changes {
            let ItemLocation::Bag { bag, slot } = change.location else {
                continue;
            };
            match &change.item {
                Some(item) => self.0.insert((bag, slot), item.clone()),
                None => self.0.remove(&(bag, slot)),
            };
        }
    }

    /// The host's bag model, which already follows the same snapshots and deltas.
    pub fn from_inventory(inventory: &InventoryState) -> Self {
        let mut stacks = BTreeMap::new();
        for (bag, slots) in inventory.slots.iter().enumerate() {
            for (slot, item) in slots.iter().enumerate() {
                if item.is_empty() {
                    continue;
                }
                let (Ok(bag), Ok(slot)) = (u8::try_from(bag), u8::try_from(slot)) else {
                    continue;
                };
                stacks.insert(
                    (bag, slot),
                    ItemStack {
                        item_guid: item.item_guid,
                        item_id: item.item_id,
                        definition_source: item.definition_source,
                        count: item.count,
                        durability: item.durability,
                        soulbound: item.soulbound,
                    },
                );
            }
        }
        Self(stacks)
    }

    pub fn slot(&self, bag: u8, slot: u8) -> Option<&ItemStack> {
        self.0.get(&(bag, slot))
    }
}

/// `BANK` / `ACCOUNT_BANK_PANEL_TITLE`.
fn bank_title(bank: BankType) -> &'static str {
    match bank {
        BankType::Character => "Bank",
        BankType::Account => "Warband Bank",
    }
}

/// `CHARACTER_BANK_TAB_PURCHASE_PROMPT` / `ACCOUNT_BANK_TAB_PURCHASE_PROMPT`.
fn purchase_text(bank: BankType) -> &'static str {
    match bank {
        BankType::Character => {
            "The Bank offers additional storage for your items.\n\nDo you wish to purchase this tab?"
        }
        BankType::Account => {
            "The Warband Bank offers storage that is shared with all members of your Warband.\n\nDo you wish to purchase this tab?"
        }
    }
}

/// The open banker's frame, the bags it deposits from and the player's money.
#[derive(Clone, Debug, Default)]
pub struct BankSession {
    pub state: BankState,
    pub inventory: BagStacks,
    pub icons: ItemIcons,
    /// Copper, from the local player's replicated `Gold`.
    pub money: u64,
    /// `CONFIRM_BUY_*_BANK_TAB` waiting for Yes / No.
    pub purchase_confirmation: Option<PopupSpec>,
    /// Edit box texts the host must set (cleared money boxes, the tab name).
    pub text_edits: Vec<(&'static str, String)>,
}

impl BankSession {
    pub fn is_open(&self) -> bool {
        self.state.is_open()
    }

    /// `InteractionOpened`: only the banker role opens the frame.
    pub fn open_role(&mut self, npc: u64, role: NpcRole) -> bool {
        if role != NpcRole::Banker {
            return false;
        }
        self.state.open(npc);
        self.purchase_confirmation = None;
        true
    }

    pub fn apply_contents(&mut self, contents: BankContents) {
        self.state.apply(contents);
    }

    /// A refusal for the open banker: the Retail error text to show.
    pub fn apply_failed(&self, failed: BankFailed) -> Option<&'static str> {
        (self.state.npc == Some(failed.npc)).then(|| failed.error.message())
    }

    /// `InteractionClosed` from the server: close without a request.
    pub fn close_for(&mut self, npc: u64) -> bool {
        if self.state.npc != Some(npc) {
            return false;
        }
        self.close_frame();
        true
    }

    /// The close button or Escape: close the frame and end the interaction.
    pub fn close(&mut self) -> Vec<BankEffect> {
        let Some(npc) = self.state.npc else {
            return Vec::new();
        };
        self.close_frame();
        vec![BankEffect::CloseInteraction { npc }]
    }

    /// Another frame or a reset ends the interaction; the server already knows.
    pub fn close_frame(&mut self) {
        self.state.close();
        self.purchase_confirmation = None;
        self.text_edits.clear();
    }

    fn effect(&self, request: BankRequest) -> Vec<BankEffect> {
        self.state
            .npc
            .map(|npc| BankEffect::Request { npc, request })
            .into_iter()
            .collect()
    }

    /// Right-click on a bag slot (`UseContainerItem` with the bank open), or a
    /// cursor item dropped on the bank: deposit it into the shown tab.
    pub fn deposit_bag(&mut self, bag: u8, slot: u8) -> Vec<BankEffect> {
        if !self.is_open() || self.state.shows_purchase_prompt() || self.state.prompt.is_some() {
            return Vec::new();
        }
        let Some(item) = self.inventory.slot(bag, slot) else {
            return Vec::new();
        };
        let bank = self.state.shown;
        let request = BankRequest::Deposit {
            bank,
            tab: self.state.selected_tab(bank) as u8,
            item_guid: item.item_guid,
        };
        self.effect(request)
    }

    /// Yes / No on `CONFIRM_BUY_*_BANK_TAB`.
    pub fn confirm_purchase(&mut self, accepted: bool) -> Vec<BankEffect> {
        if self.purchase_confirmation.take().is_none() || !accepted {
            return Vec::new();
        }
        let bank = self.state.shown;
        self.effect(BankRequest::PurchaseTab { bank })
    }

    /// A click on a BankFrame action (BF.lua): right-click a slot moves its stack to the
    /// bags, right-click a bank tab opens its settings, Purchase asks first.
    pub fn click(&mut self, action: &str, click: Click, texts: &InputTexts) -> Vec<BankEffect> {
        if !self.is_open() {
            return Vec::new();
        }
        if action == frame::ACTION_CLOSE {
            return self.close();
        }
        if self.frame_button(action) {
            return Vec::new();
        }
        let request = match action {
            frame::ACTION_MONEY_ACCEPT => self.money_request(texts),
            frame::ACTION_AUTO_DEPOSIT => self.auto_deposit_request(),
            frame::ACTION_SETTINGS_ACCEPT => self.settings_request(texts),
            _ => self.indexed_click(action, click),
        };
        request.map_or_else(Vec::new, |request| self.effect(request))
    }

    /// Buttons that only change the frame; `false` for any other action.
    fn frame_button(&mut self, action: &str) -> bool {
        match action {
            frame::ACTION_SHOW_CHARACTER => self.state.show(BankType::Character),
            frame::ACTION_SHOW_ACCOUNT => self.state.show(BankType::Account),
            frame::ACTION_PURCHASE_TAB => {
                let purchased = self.state.shown_contents().map_or(0, |c| c.tabs.len());
                self.state.select_tab(purchased);
            }
            frame::ACTION_PURCHASE => self.ask_purchase(),
            frame::ACTION_DEPOSIT_MONEY => self.open_money_prompt(true),
            frame::ACTION_WITHDRAW_MONEY => self.open_money_prompt(false),
            frame::ACTION_MONEY_CANCEL | frame::ACTION_SETTINGS_CANCEL => self.state.prompt = None,
            frame::ACTION_INCLUDE_REAGENTS => {
                self.state.include_reagents = !self.state.include_reagents;
            }
            _ => return false,
        }
        true
    }

    /// Purchase on an affordable prompt asks `CONFIRM_BUY_*_BANK_TAB` first.
    fn ask_purchase(&mut self) {
        let Some(prompt) = self.frame_state().purchase.filter(|p| p.can_afford) else {
            return;
        };
        let text = match self.state.shown {
            BankType::Character => "Do you want to purchase a Bank tab for:",
            BankType::Account => "Do you want to purchase a Warband Bank tab for:",
        };
        self.purchase_confirmation = Some(confirm_purchase(BUY_BANK_TAB_POPUP, text, prompt.cost));
    }

    /// Deposit / Withdraw open the amount prompt while their button is enabled.
    fn open_money_prompt(&mut self, deposit: bool) {
        let enabled = self.frame_state().money.is_some_and(|money| {
            if deposit {
                money.can_deposit
            } else {
                money.can_withdraw
            }
        });
        if !enabled {
            return;
        }
        self.state.prompt = Some(if deposit {
            BankPrompt::DepositMoney
        } else {
            BankPrompt::WithdrawMoney
        });
        self.text_edits = cleared(frame::MONEY_BOXES);
    }

    /// Accept on the amount prompt; an empty amount sends nothing.
    fn money_request(&mut self, texts: &InputTexts) -> Option<BankRequest> {
        let deposit = match self.state.prompt.take()? {
            BankPrompt::DepositMoney => true,
            BankPrompt::WithdrawMoney => false,
            BankPrompt::TabSettings { .. } => return None,
        };
        let copper = money_input(texts, frame::MONEY_BOXES);
        (copper > 0).then_some(BankRequest::Money {
            bank: self.state.shown,
            copper,
            deposit,
        })
    }

    /// Deposit All; "Include tradeable reagents" applies to the Warband bank only.
    fn auto_deposit_request(&self) -> Option<BankRequest> {
        if self.state.shows_purchase_prompt() {
            return None;
        }
        let bank = self.state.shown;
        Some(BankRequest::AutoDeposit {
            bank,
            include_reagents: self.state.include_reagents && bank == BankType::Account,
        })
    }

    fn settings_request(&mut self, texts: &InputTexts) -> Option<BankRequest> {
        let Some(BankPrompt::TabSettings { tab, flags }) = self.state.prompt.take() else {
            return None;
        };
        let icon = self.state.shown_contents()?.tabs.get(tab)?.icon;
        Some(BankRequest::UpdateTab {
            bank: self.state.shown,
            tab: tab as u8,
            name: text(texts, frame::TAB_NAME_BOX).trim().to_string(),
            icon,
            deposit_flags: flags,
        })
    }

    /// Settings checkboxes, side tabs and right-clicked filled slots.
    fn indexed_click(&mut self, action: &str, click: Click) -> Option<BankRequest> {
        if let Some(bit) = index(action, frame::ACTION_SETTINGS_FLAG_PREFIX) {
            if let Some(BankPrompt::TabSettings { flags, .. }) = &mut self.state.prompt {
                *flags ^= bit as u32;
            }
            return None;
        }
        if let Some(side) = index(action, frame::ACTION_TAB_PREFIX) {
            self.select_side_tab(side, click);
            return None;
        }
        let slot = index(action, frame::ACTION_SLOT_PREFIX)?;
        if !click.right || self.state.shows_purchase_prompt() {
            return None;
        }
        let bank = self.state.shown;
        let tab = self.state.selected_tab(bank);
        self.state
            .shown_contents()?
            .tabs
            .get(tab)?
            .slots
            .get(slot)?
            .as_ref()?;
        Some(BankRequest::Withdraw {
            bank,
            tab: tab as u8,
            slot: slot as u8,
        })
    }

    /// Left selects a tab; right also opens its settings (`BankPanelTabMixin:OnClick`).
    fn select_side_tab(&mut self, side: usize, click: Click) {
        self.state.select_tab(side);
        if !click.right {
            return;
        }
        let Some(settings) = self.state.shown_contents().and_then(|c| c.tabs.get(side)) else {
            return;
        };
        let name = settings.name.clone();
        self.state.prompt = Some(BankPrompt::TabSettings {
            tab: side,
            flags: settings.deposit_flags,
        });
        self.text_edits = vec![(frame::TAB_NAME_BOX, name)];
    }

    pub fn frame_state(&self) -> BankFrameState {
        let bank = &self.state;
        let Some(contents) = bank.shown_contents().filter(|_| bank.is_open()) else {
            return BankFrameState::default();
        };
        let shown = bank.shown;
        let selected = bank.selected_tab(shown);
        let purchasing = selected >= contents.tabs.len();
        let account = shown == BankType::Account;
        BankFrameState {
            visible: true,
            title: bank_title(shown).into(),
            account,
            header: contents
                .tabs
                .get(selected)
                .map(|tab| tab.name.clone())
                .unwrap_or_default(),
            tabs: side_tabs(contents, selected),
            purchase_tab: contents.next_tab_cost.map(|_| purchasing),
            slots: self.slots(contents.tabs.get(selected)),
            purchase: self.purchase_prompt(contents, purchasing),
            money: contents.money.map(|stored| MoneyFrameView {
                money: stored,
                can_withdraw: stored > 0,
                can_deposit: self.money > 0,
            }),
            deposit_all_label: if account {
                "Deposit All Warbound Items".into()
            } else {
                "Deposit All Reagents".into()
            },
            include_reagents: account.then_some(bank.include_reagents),
            prompt: bank.prompt.map(|prompt| prompt_view(prompt, account)),
        }
    }

    /// The selected tab's grid; none while the purchase prompt shows.
    fn slots(&self, tab: Option<&shared::protocol::BankTabView>) -> Vec<Option<SlotItem>> {
        tab.map_or_else(Vec::new, |tab| {
            tab.slots
                .iter()
                .map(|slot| slot.as_ref().map(|stack| self.icons.slot_item(stack)))
                .collect()
        })
    }

    fn purchase_prompt(
        &self,
        contents: &BankContents,
        purchasing: bool,
    ) -> Option<PurchasePromptView> {
        let cost = contents.next_tab_cost.filter(|_| purchasing)?;
        Some(PurchasePromptView {
            title: bank_title(contents.bank).into(),
            text: purchase_text(contents.bank).into(),
            cost,
            can_afford: self.money >= cost,
        })
    }
}

fn side_tabs(contents: &BankContents, selected: usize) -> Vec<SideTab> {
    contents
        .tabs
        .iter()
        .enumerate()
        .map(|(index, tab)| SideTab {
            icon_fdid: tab.icon,
            selected: index == selected,
        })
        .collect()
}

fn prompt_view(prompt: BankPrompt, account: bool) -> BankPromptView {
    match prompt {
        BankPrompt::DepositMoney => BankPromptView::Money { deposit: true },
        BankPrompt::WithdrawMoney => BankPromptView::Money { deposit: false },
        BankPrompt::TabSettings { flags, .. } => BankPromptView::TabSettings {
            flags,
            name_prompt: if account {
                "Enter Warband Bank Tab Name:".into()
            } else {
                "Enter Bank Tab Name:".into()
            },
        },
    }
}
