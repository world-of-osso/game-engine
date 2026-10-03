//! Client bank state: the server's `BankContents` (character and Warband bank) for
//! the open banker and `GuildBankContents` / `GuildBankLog` for the open Guild Vault,
//! the tab and mode the frames show, and the player actions the frames ask the
//! network layer to send (`BankRequest`, `GuildBankRequest`).

#[cfg(not(godot_host))]
use bevy::prelude::*;
use shared::protocol::{BankContents, BankType, GuildBankContents, GuildBankLog, GuildBankLogKind};

/// A prompt shown over a bank frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BankPrompt {
    /// `BANK_MONEY_DEPOSIT` / `GUILDBANK_DEPOSIT` amount entry.
    DepositMoney,
    /// `BANK_MONEY_WITHDRAW` / `GUILDBANK_WITHDRAW` amount entry.
    WithdrawMoney,
    /// Tab name and deposit assignments (Retail `BankPanelTabSettingsMenu`).
    TabSettings { tab: usize, flags: u32 },
}

/// The open bank frame (`npc` = banker server entity bits; `None` = closed).
#[cfg_attr(not(godot_host), derive(Resource))]
#[derive(Clone, Debug, PartialEq)]
pub struct BankState {
    pub npc: Option<u64>,
    pub character: Option<BankContents>,
    pub account: Option<BankContents>,
    /// Retail top tab: `Enum.BankType.Character` or `Account`.
    pub shown: BankType,
    /// Selected bank tab of the character and the Warband bank.
    pub tabs: [usize; 2],
    /// CVar `bankAutoDepositReagents` ("Include tradeable reagents").
    pub include_reagents: bool,
    pub prompt: Option<BankPrompt>,
}

impl Default for BankState {
    fn default() -> Self {
        Self {
            npc: None,
            character: None,
            account: None,
            shown: BankType::Character,
            tabs: [0; 2],
            include_reagents: false,
            prompt: None,
        }
    }
}

/// Highest tab index a frame can select: the purchase tab after the bought ones
/// while one can be bought.
fn last_selectable(purchased: usize, purchasable: bool) -> usize {
    if purchasable {
        purchased
    } else {
        purchased.saturating_sub(1)
    }
}

fn bank_index(bank: BankType) -> usize {
    match bank {
        BankType::Character => 0,
        BankType::Account => 1,
    }
}

impl BankState {
    pub fn is_open(&self) -> bool {
        self.npc.is_some()
    }

    /// The banker role opened: a fresh frame on the character bank. Contents the
    /// server sent for this opening may already be here (they can arrive in the same
    /// network batch as `InteractionOpened`), so they are kept.
    pub fn open(&mut self, npc: u64) {
        *self = Self {
            npc: Some(npc),
            character: self.character.take(),
            account: self.account.take(),
            include_reagents: self.include_reagents,
            ..Self::default()
        };
    }

    pub fn close(&mut self) {
        *self = Self {
            include_reagents: self.include_reagents,
            ..Self::default()
        };
    }

    pub fn apply(&mut self, contents: BankContents) {
        let bank = contents.bank;
        let last = last_selectable(contents.tabs.len(), contents.next_tab_cost.is_some());
        match bank {
            BankType::Character => self.character = Some(contents),
            BankType::Account => self.account = Some(contents),
        }
        let selected = &mut self.tabs[bank_index(bank)];
        *selected = (*selected).min(last);
    }

    pub fn contents(&self, bank: BankType) -> Option<&BankContents> {
        match bank {
            BankType::Character => self.character.as_ref(),
            BankType::Account => self.account.as_ref(),
        }
    }

    pub fn shown_contents(&self) -> Option<&BankContents> {
        self.contents(self.shown)
    }

    pub fn selected_tab(&self, bank: BankType) -> usize {
        self.tabs[bank_index(bank)]
    }

    /// Top tab click: a prompt of the other bank closes.
    pub fn show(&mut self, bank: BankType) {
        if self.shown != bank {
            self.prompt = None;
        }
        self.shown = bank;
    }

    /// Side tab click: a purchased tab, or the purchase tab after them (`tabs.len()`)
    /// while one can be bought.
    pub fn select_tab(&mut self, tab: usize) {
        let Some(contents) = self.shown_contents() else {
            return;
        };
        if tab <= last_selectable(contents.tabs.len(), contents.next_tab_cost.is_some()) {
            self.tabs[bank_index(self.shown)] = tab;
        }
    }

    /// The purchase tab is selected (Retail selects it while no tab is bought).
    pub fn shows_purchase_prompt(&self) -> bool {
        self.shown_contents()
            .is_some_and(|contents| self.selected_tab(self.shown) >= contents.tabs.len())
    }
}

/// A bank frame action for the server, sent to the open banker.
#[cfg_attr(not(godot_host), derive(Message))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BankRequest {
    Deposit {
        bank: BankType,
        tab: u8,
        item_guid: u64,
    },
    Withdraw {
        bank: BankType,
        tab: u8,
        slot: u8,
    },
    PurchaseTab {
        bank: BankType,
    },
    Money {
        bank: BankType,
        copper: u64,
        deposit: bool,
    },
    AutoDeposit {
        bank: BankType,
        include_reagents: bool,
    },
    UpdateTab {
        bank: BankType,
        tab: u8,
        name: String,
        icon: u32,
        deposit_flags: u32,
    },
}

/// Retail `GuildBankFrame` bottom tabs (GB.lua:40-42).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GuildBankMode {
    #[default]
    Bank,
    Log,
    MoneyLog,
    Info,
}

/// The open guild bank frame (`object` = vault server entity bits; `None` = closed).
#[cfg_attr(not(godot_host), derive(Resource))]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GuildBankState {
    pub object: Option<u64>,
    pub contents: Option<GuildBankContents>,
    /// Selected bank tab (`GetCurrentGuildBankTab` - 1).
    pub tab: usize,
    pub mode: GuildBankMode,
    /// The last log the server sent for the shown mode.
    pub log: Option<GuildBankLog>,
    pub prompt: Option<BankPrompt>,
}

impl GuildBankState {
    pub fn is_open(&self) -> bool {
        self.object.is_some()
    }

    /// The Guild Vault role opened; contents of this vault that arrived first are kept.
    pub fn open(&mut self, object: u64) {
        let contents = self.contents.take().filter(|c| c.object == object);
        *self = Self {
            object: Some(object),
            contents,
            ..Self::default()
        };
    }

    pub fn close(&mut self) {
        *self = Self::default();
    }

    /// Contents of another vault than the open one are ignored.
    pub fn apply(&mut self, contents: GuildBankContents) {
        if self.object.is_some_and(|object| object != contents.object) {
            return;
        }
        let last = last_selectable(contents.tabs.len(), contents.next_tab_cost.is_some());
        self.tab = self.tab.min(last);
        self.contents = Some(contents);
    }

    /// A log for the shown mode and tab; others are stale answers.
    pub fn apply_log(&mut self, log: GuildBankLog) {
        let wanted = match self.mode {
            GuildBankMode::Log => Some(self.tab as u8),
            GuildBankMode::MoneyLog => None,
            GuildBankMode::Bank | GuildBankMode::Info => return,
        };
        if log.tab == wanted {
            self.log = Some(log);
        }
    }

    /// The log to query when switching to `mode` (Retail `QueryGuildBankLog`).
    pub fn set_mode(&mut self, mode: GuildBankMode) -> Option<GuildBankRequest> {
        self.mode = mode;
        self.log = None;
        self.log_query()
    }

    /// Side tab click: a purchased tab, or the Guild Master's buy tab after them.
    pub fn select_tab(&mut self, tab: usize) -> Option<GuildBankRequest> {
        let contents = self.contents.as_ref()?;
        if tab > last_selectable(contents.tabs.len(), contents.next_tab_cost.is_some()) {
            return None;
        }
        self.tab = tab;
        self.log = None;
        self.log_query()
    }

    fn log_query(&self) -> Option<GuildBankRequest> {
        let purchased = self.contents.as_ref().map_or(0, |c| c.tabs.len());
        if self.mode == GuildBankMode::Log && self.tab >= purchased {
            return None;
        }
        match self.mode {
            GuildBankMode::Log => Some(GuildBankRequest::QueryLog {
                tab: Some(self.tab as u8),
            }),
            GuildBankMode::MoneyLog => Some(GuildBankRequest::QueryLog { tab: None }),
            GuildBankMode::Bank | GuildBankMode::Info => None,
        }
    }
}

/// A guild bank frame action for the server, sent to the open vault.
#[cfg_attr(not(godot_host), derive(Message))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuildBankRequest {
    Deposit { tab: u8, item_guid: u64 },
    Withdraw { tab: u8, slot: u8 },
    Money { copper: u64, deposit: bool },
    BuyTab,
    SetTabInfo { tab: u8, name: String, icon: u32 },
    SetTabText { tab: u8, text: String },
    QueryLog { tab: Option<u8> },
}

/// Retail `GetMoneyString` without coin textures: "12g 3s 5c", zero parts left out.
pub fn money_text(copper: u64) -> String {
    let parts = [
        (copper / 10_000, "g"),
        (copper / 100 % 100, "s"),
        (copper % 100, "c"),
    ];
    let text: Vec<String> = parts
        .iter()
        .filter(|(amount, _)| *amount > 0)
        .map(|(amount, unit)| format!("{amount}{unit}"))
        .collect();
    if text.is_empty() {
        "0c".into()
    } else {
        text.join(" ")
    }
}

/// Retail `GUILD_BANK_LOG_TIME` age: the largest whole unit (`RecentTimeDate`).
pub fn log_age(seconds: u64) -> String {
    let (amount, unit) = match seconds {
        s if s >= 86_400 => (s / 86_400, "day"),
        s if s >= 3_600 => (s / 3_600, "hour"),
        s => ((s / 60).max(1), "min"),
    };
    let plural = if amount == 1 || unit == "min" {
        ""
    } else {
        "s"
    };
    format!("( {amount} {unit}{plural} ago )")
}

/// One Retail log line (GB.lua:750-817): `GUILDBANK_DEPOSIT_FORMAT` "%s deposited %s",
/// `GUILDBANK_WITHDRAW_FORMAT` "%s withdrew %s", `GUILDBANK_BUYTAB_MONEY_FORMAT`
/// "%s purchased a guild bank tab for %s", `GUILDBANK_REPAIR_MONEY_FORMAT`
/// "%s withdrew %s for repairs" (GB.lua:797-798); items read "name x count".
pub fn log_line(entry: &shared::protocol::GuildBankLogEntry) -> String {
    let item = if entry.count > 1 {
        format!("{} x {}", entry.item_name, entry.count)
    } else {
        entry.item_name.clone()
    };
    let money = money_text(entry.copper);
    let text = match entry.kind {
        GuildBankLogKind::DepositItem => format!("{} deposited {item}", entry.actor),
        GuildBankLogKind::WithdrawItem => format!("{} withdrew {item}", entry.actor),
        GuildBankLogKind::DepositMoney => format!("{} deposited {money}", entry.actor),
        GuildBankLogKind::WithdrawMoney => format!("{} withdrew {money}", entry.actor),
        GuildBankLogKind::BuyTab => {
            format!("{} purchased a guild bank tab for {money}", entry.actor)
        }
        GuildBankLogKind::RepairMoney => format!("{} withdrew {money} for repairs", entry.actor),
    };
    format!("{text} {}", log_age(entry.seconds_ago))
}

#[cfg(test)]
#[path = "bank_data_tests.rs"]
mod tests;
