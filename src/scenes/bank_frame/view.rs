//! Frame view models from [`BankState`] / [`GuildBankState`] and the player's money.

use game_engine::bag_data::stack_slot;
use game_engine::bank_data::{BankPrompt, BankState, GuildBankMode, GuildBankState, log_line};
use game_engine::ui::screens::bank_art::SlotItem;
use game_engine::ui::screens::bank_frame_component::{
    BankFrameState, BankPromptView, MoneyFrameView, PurchasePromptView, SideTab,
};
use game_engine::ui::screens::guild_bank_frame_component::{
    BuyTabView, GuildBankFrameState, GuildBankModeView, GuildSideTab,
};
use shared::protocol::{BankContents, BankType, GuildBankTabView, ItemStack};

fn slot_item(stack: &ItemStack) -> SlotItem {
    let slot = stack_slot(stack);
    SlotItem {
        icon_fdid: slot.icon_fdid,
        count: slot.count,
        quality_border: "1.0,1.0,1.0,1.0".into(),
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

pub fn bank_frame_state(bank: &BankState, window_open: bool, money: u64) -> BankFrameState {
    let Some(contents) = bank.shown_contents().filter(|_| bank.is_open()) else {
        return BankFrameState::default();
    };
    let shown = bank.shown;
    let selected = bank.selected_tab(shown);
    let purchasing = selected >= contents.tabs.len();
    let tabs = contents
        .tabs
        .iter()
        .enumerate()
        .map(|(index, tab)| SideTab {
            icon_fdid: tab.icon,
            selected: index == selected,
        })
        .collect();
    let account = shown == BankType::Account;
    BankFrameState {
        visible: window_open,
        title: bank_title(shown).into(),
        account,
        header: contents
            .tabs
            .get(selected)
            .map(|tab| tab.name.clone())
            .unwrap_or_default(),
        tabs,
        purchase_tab: contents.next_tab_cost.map(|_| purchasing),
        slots: if purchasing {
            Vec::new()
        } else {
            contents.tabs[selected]
                .slots
                .iter()
                .map(|slot| slot.as_ref().map(slot_item))
                .collect()
        },
        purchase: purchase_prompt(contents, purchasing, money),
        money: contents.money.map(|stored| MoneyFrameView {
            money: stored,
            can_withdraw: stored > 0,
            can_deposit: money > 0,
        }),
        deposit_all_label: if account {
            "Deposit All Warbound Items".into()
        } else {
            "Deposit All Reagents".into()
        },
        include_reagents: account.then_some(bank.include_reagents),
        prompt: bank.prompt.map(|prompt| match prompt {
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
        }),
    }
}

fn purchase_prompt(
    contents: &BankContents,
    purchasing: bool,
    money: u64,
) -> Option<PurchasePromptView> {
    let cost = contents.next_tab_cost.filter(|_| purchasing)?;
    Some(PurchasePromptView {
        title: bank_title(contents.bank).into(),
        text: purchase_text(contents.bank).into(),
        cost,
        can_afford: money >= cost,
    })
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

fn mode_view(mode: GuildBankMode) -> GuildBankModeView {
    match mode {
        GuildBankMode::Bank => GuildBankModeView::Bank,
        GuildBankMode::Log => GuildBankModeView::Log,
        GuildBankMode::MoneyLog => GuildBankModeView::MoneyLog,
        GuildBankMode::Info => GuildBankModeView::Info,
    }
}

pub fn guild_bank_frame_state(
    guild: &GuildBankState,
    window_open: bool,
    money: u64,
) -> GuildBankFrameState {
    let Some(contents) = guild.contents.as_ref().filter(|_| guild.is_open()) else {
        return GuildBankFrameState::default();
    };
    let selected = guild.tab;
    let buying = selected >= contents.tabs.len();
    let tab = contents.tabs.get(selected);
    let mut state = GuildBankFrameState {
        visible: window_open,
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
        buy_tab: contents.next_tab_cost.map(|_| buying),
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
        GuildBankMode::Bank => bank_mode(&mut state, contents, tab, buying, money),
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
    state: &mut GuildBankFrameState,
    contents: &shared::protocol::GuildBankContents,
    tab: Option<&GuildBankTabView>,
    buying: bool,
    money: u64,
) {
    if buying {
        match contents.next_tab_cost.filter(|_| contents.is_leader) {
            Some(cost) => {
                state.buy = Some(BuyTabView {
                    cost,
                    purchased: contents.tabs.len(),
                    can_afford: money >= cost,
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
    let Some(tab) = tab else { return };
    if !tab.viewable {
        return;
    }
    let (suffix, color) = access(tab);
    state.tab_title = Some((tab.name.clone(), suffix.into(), color));
    state.locked = suffix == "(Locked)";
    state.limit_text = Some(remaining_text(tab));
    state.slots = tab
        .slots
        .iter()
        .map(|slot| slot.as_ref().map(slot_item))
        .collect();
}
