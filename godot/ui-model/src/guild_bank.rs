//! Portable GuildBankFrame session for the native host, ported from the Bevy client's
//! `scenes/bank_frame` and `networking/bank`: the server's `GuildBankContents` and
//! `GuildBankLog` for the open Guild Vault drive the shared GuildBankFrame and backpack,
//! and frame and bag clicks become the `GuildBankChannel` requests the server answers
//! (docs/specs/guild-bank-frame.md). Bags, money and vault contents change only through
//! the server's replies.

use shared::protocol::{
    GuildBankContents, GuildBankFailed, GuildBankLog, GuildBankTabView, InventoryDelta,
    InventorySnapshot, NpcRole,
};

use crate::bag_frame_component::{BagFrameState, bag_frame_screen};
use crate::bank::{
    BagStacks, InputTexts, ItemIcons, cleared, confirm_purchase, index, money_input,
};
use crate::bank_data::{BankPrompt, GuildBankMode, GuildBankRequest, GuildBankState, log_line};
use crate::guild_bank_frame_component::{
    self as frame, BuyTabView, GuildBankFrameState, GuildBankModeView, GuildSideTab,
    guild_bank_frame_screen,
};
use crate::merchant::Click;
use crate::popup::PopupSpec;

pub const BUY_GUILD_BANK_TAB_POPUP: &str = "CONFIRM_BUY_GUILDBANK_TAB";

/// The GuildBankFrame and the backpack it deposits from.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeGuildBankView {
    pub frame: GuildBankFrameState,
    pub bags: BagFrameState,
}

pub fn native_guild_bank_screen(
    ctx: &ui_toolkit::screen::SharedContext,
) -> ui_toolkit::widget_def::Element {
    let view = ctx
        .get::<NativeGuildBankView>()
        .expect("NativeGuildBankView must be in SharedContext");
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(view.frame.clone());
    shared.insert(view.bags.clone());
    let mut elements = guild_bank_frame_screen(&shared);
    elements.extend(bag_frame_screen(&shared));
    elements
}

/// What the host sends for a frame input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuildBankEffect {
    /// A guild bank request to the open vault `object`.
    Request {
        object: u64,
        request: GuildBankRequest,
    },
    /// `CloseInteraction { npc }`: the player closed the frame.
    CloseInteraction { npc: u64 },
}

/// The open Guild Vault's frame, the bags it deposits from and the player's money.
#[derive(Clone, Debug, Default)]
pub struct GuildBankSession {
    pub state: GuildBankState,
    pub inventory: BagStacks,
    pub icons: ItemIcons,
    /// Copper, from the local player's replicated `Gold`.
    pub money: u64,
    /// The last refusal of the open vault (Retail UI error text).
    pub error: Option<String>,
    /// `CONFIRM_BUY_GUILDBANK_TAB` waiting for Yes / No.
    pub purchase_confirmation: Option<PopupSpec>,
    /// Edit box texts the host must set (cleared money boxes, the tab info).
    pub text_edits: Vec<(&'static str, String)>,
}

impl GuildBankSession {
    /// `InteractionOpened`: only the guild banker role opens the frame; contents of
    /// this vault that arrived first are kept.
    pub fn open_role(&mut self, object: u64, role: NpcRole) -> bool {
        if role != NpcRole::GuildBanker {
            return false;
        }
        self.state.open(object);
        self.error = None;
        self.purchase_confirmation = None;
        true
    }

    /// Contents of the open (or opening) vault; a change while a log is shown queries
    /// it again (Retail `GUILDBANKLOG_UPDATE`).
    pub fn apply_contents(&mut self, contents: GuildBankContents) -> Vec<GuildBankEffect> {
        if self
            .state
            .object
            .is_some_and(|object| object != contents.object)
        {
            return Vec::new();
        }
        self.state.apply(contents);
        if !self.state.is_open() {
            return Vec::new();
        }
        let mode = self.state.mode;
        let query = self.state.set_mode(mode);
        self.effects(query)
    }

    pub fn apply_log(&mut self, log: GuildBankLog) {
        if self.state.is_open() {
            self.state.apply_log(log);
        }
    }

    /// A refusal for the open vault: the Retail error text to show.
    pub fn apply_failed(&mut self, failed: GuildBankFailed) -> Option<&'static str> {
        if self.state.object != Some(failed.object) {
            return None;
        }
        let message = failed.error.message();
        self.error = Some(message.into());
        Some(message)
    }

    pub fn apply_inventory(&mut self, snapshot: InventorySnapshot) {
        self.inventory.apply_snapshot(&snapshot);
    }

    pub fn apply_inventory_delta(&mut self, delta: InventoryDelta) {
        self.inventory.apply_delta(&delta);
    }

    /// `InteractionClosed` from the server: close without a request.
    pub fn close_for(&mut self, object: u64) -> bool {
        if self.state.object != Some(object) {
            return false;
        }
        self.close_frame();
        true
    }

    /// The close button or Escape: close the frame and end the interaction.
    pub fn close(&mut self) -> Vec<GuildBankEffect> {
        let Some(npc) = self.state.object else {
            return Vec::new();
        };
        self.close_frame();
        vec![GuildBankEffect::CloseInteraction { npc }]
    }

    /// Another frame or a reset ends the interaction; the server already knows.
    pub fn close_frame(&mut self) {
        self.state.close();
        self.error = None;
        self.purchase_confirmation = None;
        self.text_edits.clear();
    }

    fn effects(&self, request: Option<GuildBankRequest>) -> Vec<GuildBankEffect> {
        let Some(object) = self.state.object else {
            return Vec::new();
        };
        request
            .map(|request| GuildBankEffect::Request { object, request })
            .into_iter()
            .collect()
    }

    /// Side tab click: a purchased tab, or the Guild Master's buy tab after them.
    pub fn select_tab(&mut self, tab: usize) -> Vec<GuildBankEffect> {
        if !self.state.is_open() {
            return Vec::new();
        }
        let query = self.state.select_tab(tab);
        self.text_edits.extend(self.info_text());
        self.effects(query)
    }

    /// Bottom tab click: Guild Bank / Log / Money Log / Info.
    pub fn set_mode(&mut self, mode: GuildBankMode) -> Vec<GuildBankEffect> {
        if !self.state.is_open() {
            return Vec::new();
        }
        let query = self.state.set_mode(mode);
        self.text_edits.extend(self.info_text());
        self.effects(query)
    }

    /// The info box shows the selected tab's text when info mode or the tab changes.
    fn info_text(&self) -> Vec<(&'static str, String)> {
        if self.state.mode != GuildBankMode::Info {
            return Vec::new();
        }
        let text = self
            .selected_tab()
            .map(|tab| tab.text.clone())
            .unwrap_or_default();
        vec![(frame::INFO_BOX, text)]
    }

    fn selected_tab(&self) -> Option<&GuildBankTabView> {
        self.state.contents.as_ref()?.tabs.get(self.state.tab)
    }

    /// Right-click on a bag slot, or a cursor item dropped on the vault: deposit it
    /// into the shown tab.
    pub fn deposit_bag(&mut self, bag: u8, slot: u8) -> Vec<GuildBankEffect> {
        if self.state.mode != GuildBankMode::Bank || self.state.prompt.is_some() {
            return Vec::new();
        }
        if self.selected_tab().is_none_or(|tab| !tab.viewable) {
            return Vec::new();
        }
        let Some(item) = self.inventory.slot(bag, slot) else {
            return Vec::new();
        };
        let request = GuildBankRequest::Deposit {
            tab: self.state.tab as u8,
            item_guid: item.item_guid,
        };
        self.effects(Some(request))
    }

    /// Yes / No on `CONFIRM_BUY_GUILDBANK_TAB`.
    pub fn confirm_purchase(&mut self, accepted: bool) -> Vec<GuildBankEffect> {
        if self.purchase_confirmation.take().is_none() || !accepted {
            return Vec::new();
        }
        self.effects(Some(GuildBankRequest::BuyTab))
    }

    /// A click on a GuildBankFrame action (GB.lua): right-click a slot moves its stack
    /// to the bags, Purchase asks first.
    pub fn click(
        &mut self,
        action: &str,
        click: Click,
        texts: &InputTexts,
    ) -> Vec<GuildBankEffect> {
        if !self.state.is_open() {
            return Vec::new();
        }
        match action {
            frame::ACTION_CLOSE => self.close(),
            frame::ACTION_BUY_TAB_TAB => {
                let purchased = self.state.contents.as_ref().map_or(0, |c| c.tabs.len());
                self.select_tab(purchased)
            }
            frame::ACTION_BUY_TAB => {
                if let Some(buy) = self.frame_state().buy.filter(|buy| buy.can_afford) {
                    self.purchase_confirmation = Some(confirm_purchase(
                        BUY_GUILD_BANK_TAB_POPUP,
                        "Do you want to purchase a Guild Bank tab for:",
                        buy.cost,
                    ));
                }
                Vec::new()
            }
            frame::ACTION_DEPOSIT_MONEY | frame::ACTION_WITHDRAW_MONEY => {
                let deposit = action == frame::ACTION_DEPOSIT_MONEY;
                if deposit || self.frame_state().can_withdraw {
                    self.state.prompt = Some(if deposit {
                        BankPrompt::DepositMoney
                    } else {
                        BankPrompt::WithdrawMoney
                    });
                    self.text_edits = cleared(frame::MONEY_BOXES);
                }
                Vec::new()
            }
            frame::ACTION_MONEY_ACCEPT => {
                let deposit = match self.state.prompt.take() {
                    Some(BankPrompt::DepositMoney) => true,
                    Some(BankPrompt::WithdrawMoney) => false,
                    _ => return Vec::new(),
                };
                let copper = money_input(texts, frame::MONEY_BOXES);
                if copper == 0 {
                    return Vec::new();
                }
                self.effects(Some(GuildBankRequest::Money { copper, deposit }))
            }
            frame::ACTION_MONEY_CANCEL => {
                self.state.prompt = None;
                Vec::new()
            }
            frame::ACTION_SAVE_INFO => {
                if self
                    .frame_state()
                    .info
                    .is_none_or(|(_, editable)| !editable)
                {
                    return Vec::new();
                }
                let text = texts.get(frame::INFO_BOX).cloned().unwrap_or_default();
                let tab = self.state.tab as u8;
                self.effects(Some(GuildBankRequest::SetTabText { tab, text }))
            }
            _ => self.indexed_click(action, click),
        }
    }

    fn indexed_click(&mut self, action: &str, click: Click) -> Vec<GuildBankEffect> {
        if let Some(mode) = action
            .strip_prefix(frame::ACTION_MODE_PREFIX)
            .and_then(mode_for)
        {
            return self.set_mode(mode);
        }
        if let Some(side) = index(action, frame::ACTION_TAB_PREFIX) {
            return self.select_tab(side);
        }
        match index(action, frame::ACTION_SLOT_PREFIX) {
            Some(id) if click.right && id > 0 && self.state.mode == GuildBankMode::Bank => {
                let slot = id - 1;
                let filled = self
                    .selected_tab()
                    .and_then(|t| t.slots.get(slot))
                    .is_some_and(Option::is_some);
                if !filled {
                    return Vec::new();
                }
                let tab = self.state.tab as u8;
                self.effects(Some(GuildBankRequest::Withdraw {
                    tab,
                    slot: slot as u8,
                }))
            }
            _ => Vec::new(),
        }
    }

    pub fn frame_state(&self) -> GuildBankFrameState {
        let guild = &self.state;
        let Some(contents) = guild.contents.as_ref().filter(|_| guild.is_open()) else {
            return GuildBankFrameState::default();
        };
        let selected = guild.tab;
        let buying = selected >= contents.tabs.len();
        let tab = contents.tabs.get(selected);
        let mut state = GuildBankFrameState {
            visible: true,
            title: contents.guild_name.clone(),
            mode: mode_view(guild.mode),
            tabs: contents
                .tabs
                .iter()
                .enumerate()
                .map(|(index, tab)| GuildSideTab {
                    icon_fdid: tab.icon,
                    selected: index == selected,
                })
                .collect(),
            buy_tab: contents
                .next_tab_cost
                .filter(|_| contents.is_leader)
                .map(|_| buying),
            money: contents.money,
            withdraw_limit: contents.withdraw_money_remaining,
            can_withdraw: contents.withdraw_money_remaining != Some(0) && contents.money > 0,
            money_prompt: match guild.prompt {
                Some(BankPrompt::DepositMoney) => Some(true),
                Some(BankPrompt::WithdrawMoney) => Some(false),
                _ => None,
            },
            ..GuildBankFrameState::default()
        };
        match guild.mode {
            GuildBankMode::Bank => self.bank_mode(&mut state, contents, tab, buying),
            GuildBankMode::Log | GuildBankMode::MoneyLog => {
                if let Some(tab) = tab.filter(|_| guild.mode == GuildBankMode::Log) {
                    state.tab_title = Some((format!("{} Log", tab.name), String::new(), ""));
                }
                state.log_lines = guild
                    .log
                    .iter()
                    .flat_map(|log| &log.entries)
                    .map(log_line)
                    .collect();
            }
            GuildBankMode::Info => {
                if let Some(tab) = tab.filter(|tab| tab.viewable) {
                    state.tab_title = Some((format!("{} Info", tab.name), String::new(), ""));
                    state.info = Some((tab.text.clone(), contents.is_leader));
                }
            }
        }
        state
    }

    fn bank_mode(
        &self,
        state: &mut GuildBankFrameState,
        contents: &GuildBankContents,
        tab: Option<&GuildBankTabView>,
        buying: bool,
    ) {
        if buying {
            match contents.next_tab_cost.filter(|_| contents.is_leader) {
                Some(cost) => {
                    state.buy = Some(BuyTabView {
                        cost,
                        purchased: contents.tabs.len(),
                        can_afford: self.money >= cost,
                    });
                }
                // NO_GUILDBANK_TABS
                None => {
                    state.error_message =
                        Some("Your guild has not purchased any guild bank space.".into());
                }
            }
            return;
        }
        let Some(tab) = tab.filter(|tab| tab.viewable) else {
            return;
        };
        let (suffix, color) = access(tab);
        state.tab_title = Some((tab.name.clone(), suffix.into(), color));
        state.locked = suffix == "(Locked)";
        state.limit_text = Some(remaining_text(tab));
        state.slots = tab
            .slots
            .iter()
            .map(|slot| slot.as_ref().map(|stack| self.icons.slot_item(stack)))
            .collect();
    }
}

fn mode_for(key: &str) -> Option<GuildBankMode> {
    Some(match key {
        "bank" => GuildBankMode::Bank,
        "log" => GuildBankMode::Log,
        "moneylog" => GuildBankMode::MoneyLog,
        "info" => GuildBankMode::Info,
        _ => return None,
    })
}

fn mode_view(mode: GuildBankMode) -> GuildBankModeView {
    match mode {
        GuildBankMode::Bank => GuildBankModeView::Bank,
        GuildBankMode::Log => GuildBankModeView::Log,
        GuildBankMode::MoneyLog => GuildBankModeView::MoneyLog,
        GuildBankMode::Info => GuildBankModeView::Info,
    }
}

/// GB.lua:384-396 access suffix and its colour.
fn access(tab: &GuildBankTabView) -> (&'static str, &'static str) {
    let withdrawals = tab.withdrawals_per_day != Some(0);
    const RED: &str = "1.0,0.125,0.125,1.0";
    match (tab.can_deposit, withdrawals) {
        (false, false) => ("(Locked)", RED),
        (false, true) => ("(Withdraw Only)", RED),
        (true, false) => ("(Deposit Only)", RED),
        (true, true) => ("(Full Access)", "0.125,1.0,0.125,1.0"),
    }
}

/// GB.lua:412-421: `STACKS` above zero, `NONE` at zero, `UNLIMITED` below.
fn remaining_text(tab: &GuildBankTabView) -> String {
    let stacks = match tab.remaining_withdrawals {
        None => "Unlimited".to_string(),
        Some(0) => "None".to_string(),
        Some(1) => "1 Stack".to_string(),
        Some(n) => format!("{n} Stacks"),
    };
    format!("Remaining Daily Withdrawals for {}:  {stacks}", tab.name)
}
