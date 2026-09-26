//! Frame clicks → state changes and server requests. Retail clicks (BF.lua,
//! GB.lua): right-click a slot moves its stack to the bags, right-click a bank tab
//! opens its settings, Purchase asks `CONFIRM_BUY_*_TAB` first.

use std::collections::HashMap;

use bevy::prelude::MouseButton;
use game_engine::bank_data::{
    BankPrompt, BankRequest, BankState, GuildBankMode, GuildBankRequest, GuildBankState, money_text,
};
use game_engine::ui::popup::PopupSpec;
use game_engine::ui::screens::bank_art::MoneyBoxNames;
use game_engine::ui::screens::bank_frame_component as bank_ui;
use game_engine::ui::screens::guild_bank_frame_component as guild_ui;
use shared::protocol::BankType;

/// Text of the frame edit boxes, by name.
pub type InputTexts = HashMap<&'static str, String>;

pub const BUY_BANK_TAB_POPUP: &str = "CONFIRM_BUY_BANK_TAB";
pub const BUY_GUILD_BANK_TAB_POPUP: &str = "CONFIRM_BUY_GUILDBANK_TAB";

/// What a click did besides changing the frame state.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome<R> {
    pub requests: Vec<R>,
    pub popup: Option<PopupSpec>,
    /// Edit box texts to set.
    pub texts: Vec<(&'static str, String)>,
    pub close: bool,
}

impl<R> Outcome<R> {
    fn request(request: R) -> Self {
        Self {
            requests: vec![request],
            popup: None,
            texts: Vec::new(),
            close: false,
        }
    }

    fn none() -> Self {
        Self {
            requests: Vec::new(),
            popup: None,
            texts: Vec::new(),
            close: false,
        }
    }
}

fn text<'a>(texts: &'a InputTexts, name: &str) -> &'a str {
    texts.get(name).map_or("", String::as_str)
}

/// Copper typed into a gold / silver / copper entry; empty boxes count as zero.
pub fn money_input(texts: &InputTexts, boxes: MoneyBoxNames) -> u64 {
    let part = |name| text(texts, name).trim().parse::<u64>().unwrap_or(0);
    part(boxes.gold) * 10_000 + part(boxes.silver) * 100 + part(boxes.copper)
}

fn cleared(boxes: MoneyBoxNames) -> Vec<(&'static str, String)> {
    boxes
        .all()
        .into_iter()
        .map(|name| (name, String::new()))
        .collect()
}

fn index(action: &str, prefix: &str) -> Option<usize> {
    action.strip_prefix(prefix)?.parse().ok()
}

fn confirm_purchase(key: &str, text: &str, cost: u64) -> PopupSpec {
    PopupSpec {
        key: key.into(),
        text: format!("{text}\n{}", money_text(cost)),
        accept_label: "Yes".into(),
        cancel_label: Some("No".into()),
        timeout: None,
        confirm_text: None,
    }
}

pub fn bank_action(
    action: &str,
    button: MouseButton,
    bank: &mut BankState,
    texts: &InputTexts,
) -> Outcome<BankRequest> {
    let shown = bank.shown;
    let tab = bank.selected_tab(shown);
    match action {
        bank_ui::ACTION_CLOSE => Outcome {
            close: true,
            ..Outcome::none()
        },
        bank_ui::ACTION_SHOW_CHARACTER => {
            bank.show(BankType::Character);
            Outcome::none()
        }
        bank_ui::ACTION_SHOW_ACCOUNT => {
            bank.show(BankType::Account);
            Outcome::none()
        }
        bank_ui::ACTION_PURCHASE_TAB => {
            let purchased = bank.shown_contents().map_or(0, |c| c.tabs.len());
            bank.select_tab(purchased);
            Outcome::none()
        }
        bank_ui::ACTION_PURCHASE => {
            let Some(cost) = bank.shown_contents().and_then(|c| c.next_tab_cost) else {
                return Outcome::none();
            };
            // CONFIRM_BUY_CHARACTER_BANK_TAB / CONFIRM_BUY_ACCOUNT_BANK_TAB.
            let text = match shown {
                BankType::Character => "Do you want to purchase a Bank tab for:",
                BankType::Account => "Do you want to purchase a Warband Bank tab for:",
            };
            Outcome {
                popup: Some(confirm_purchase(BUY_BANK_TAB_POPUP, text, cost)),
                ..Outcome::none()
            }
        }
        bank_ui::ACTION_DEPOSIT_MONEY | bank_ui::ACTION_WITHDRAW_MONEY => {
            bank.prompt = Some(if action == bank_ui::ACTION_DEPOSIT_MONEY {
                BankPrompt::DepositMoney
            } else {
                BankPrompt::WithdrawMoney
            });
            Outcome {
                texts: cleared(bank_ui::MONEY_BOXES),
                ..Outcome::none()
            }
        }
        bank_ui::ACTION_MONEY_ACCEPT => {
            let deposit = bank.prompt == Some(BankPrompt::DepositMoney);
            bank.prompt = None;
            let copper = money_input(texts, bank_ui::MONEY_BOXES);
            if copper == 0 {
                return Outcome::none();
            }
            Outcome::request(BankRequest::Money {
                bank: shown,
                copper,
                deposit,
            })
        }
        bank_ui::ACTION_MONEY_CANCEL | bank_ui::ACTION_SETTINGS_CANCEL => {
            bank.prompt = None;
            Outcome::none()
        }
        bank_ui::ACTION_AUTO_DEPOSIT => Outcome::request(BankRequest::AutoDeposit {
            bank: shown,
            include_reagents: bank.include_reagents && shown == BankType::Account,
        }),
        bank_ui::ACTION_INCLUDE_REAGENTS => {
            bank.include_reagents = !bank.include_reagents;
            Outcome::none()
        }
        bank_ui::ACTION_SETTINGS_ACCEPT => settings_accept(bank, texts),
        _ => bank_indexed_action(action, button, bank, tab),
    }
}

fn settings_accept(bank: &mut BankState, texts: &InputTexts) -> Outcome<BankRequest> {
    let Some(BankPrompt::TabSettings { tab, flags }) = bank.prompt.take() else {
        return Outcome::none();
    };
    let Some(icon) = bank
        .shown_contents()
        .and_then(|contents| contents.tabs.get(tab))
        .map(|tab| tab.icon)
    else {
        return Outcome::none();
    };
    Outcome::request(BankRequest::UpdateTab {
        bank: bank.shown,
        tab: tab as u8,
        name: text(texts, bank_ui::TAB_NAME_BOX).trim().to_string(),
        icon,
        deposit_flags: flags,
    })
}

fn bank_indexed_action(
    action: &str,
    button: MouseButton,
    bank: &mut BankState,
    tab: usize,
) -> Outcome<BankRequest> {
    if let Some(bit) = index(action, bank_ui::ACTION_SETTINGS_FLAG_PREFIX) {
        if let Some(BankPrompt::TabSettings { flags, .. }) = &mut bank.prompt {
            *flags ^= bit as u32;
        }
        return Outcome::none();
    }
    if let Some(side) = index(action, bank_ui::ACTION_TAB_PREFIX) {
        bank.select_tab(side);
        if button != MouseButton::Right {
            return Outcome::none();
        }
        // Right-click opens the tab settings (BF.lua `BankPanelTabMixin:OnClick`).
        let Some(settings) = bank.shown_contents().and_then(|c| c.tabs.get(side)) else {
            return Outcome::none();
        };
        let name = settings.name.clone();
        bank.prompt = Some(BankPrompt::TabSettings {
            tab: side,
            flags: settings.deposit_flags,
        });
        return Outcome {
            texts: vec![(bank_ui::TAB_NAME_BOX, name)],
            ..Outcome::none()
        };
    }
    match index(action, bank_ui::ACTION_SLOT_PREFIX) {
        Some(slot) if button == MouseButton::Right && !bank.shows_purchase_prompt() => {
            let filled = bank
                .shown_contents()
                .and_then(|c| c.tabs.get(tab))
                .and_then(|t| t.slots.get(slot))
                .is_some_and(Option::is_some);
            if !filled {
                return Outcome::none();
            }
            Outcome::request(BankRequest::Withdraw {
                bank: bank.shown,
                tab: tab as u8,
                slot: slot as u8,
            })
        }
        _ => Outcome::none(),
    }
}

/// The info box shows the selected tab's text when info mode or the tab changes.
fn info_text(guild: &GuildBankState) -> Vec<(&'static str, String)> {
    if guild.mode != GuildBankMode::Info {
        return Vec::new();
    }
    let text = guild
        .contents
        .as_ref()
        .and_then(|c| c.tabs.get(guild.tab))
        .map(|tab| tab.text.clone())
        .unwrap_or_default();
    vec![(guild_ui::INFO_BOX, text)]
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

pub fn guild_bank_action(
    action: &str,
    button: MouseButton,
    guild: &mut GuildBankState,
    texts: &InputTexts,
) -> Outcome<GuildBankRequest> {
    let tab = guild.tab;
    match action {
        guild_ui::ACTION_CLOSE => Outcome {
            close: true,
            ..Outcome::none()
        },
        guild_ui::ACTION_BUY_TAB_TAB => {
            let purchased = guild.contents.as_ref().map_or(0, |c| c.tabs.len());
            guild.select_tab(purchased);
            Outcome::none()
        }
        guild_ui::ACTION_BUY_TAB => {
            let Some(cost) = guild.contents.as_ref().and_then(|c| c.next_tab_cost) else {
                return Outcome::none();
            };
            Outcome {
                popup: Some(confirm_purchase(
                    BUY_GUILD_BANK_TAB_POPUP,
                    "Do you want to purchase a Guild Bank tab for:",
                    cost,
                )),
                ..Outcome::none()
            }
        }
        guild_ui::ACTION_DEPOSIT_MONEY | guild_ui::ACTION_WITHDRAW_MONEY => {
            guild.prompt = Some(if action == guild_ui::ACTION_DEPOSIT_MONEY {
                BankPrompt::DepositMoney
            } else {
                BankPrompt::WithdrawMoney
            });
            Outcome {
                texts: cleared(guild_ui::MONEY_BOXES),
                ..Outcome::none()
            }
        }
        guild_ui::ACTION_MONEY_ACCEPT => {
            let deposit = guild.prompt == Some(BankPrompt::DepositMoney);
            guild.prompt = None;
            let copper = money_input(texts, guild_ui::MONEY_BOXES);
            if copper == 0 {
                return Outcome::none();
            }
            Outcome::request(GuildBankRequest::Money { copper, deposit })
        }
        guild_ui::ACTION_MONEY_CANCEL => {
            guild.prompt = None;
            Outcome::none()
        }
        guild_ui::ACTION_SAVE_INFO => Outcome::request(GuildBankRequest::SetTabText {
            tab: tab as u8,
            text: text(texts, guild_ui::INFO_BOX).to_string(),
        }),
        _ => guild_indexed_action(action, button, guild),
    }
}

fn guild_indexed_action(
    action: &str,
    button: MouseButton,
    guild: &mut GuildBankState,
) -> Outcome<GuildBankRequest> {
    if let Some(mode) = action
        .strip_prefix(guild_ui::ACTION_MODE_PREFIX)
        .and_then(mode_for)
    {
        let requests = guild.set_mode(mode).into_iter().collect();
        return Outcome {
            requests,
            texts: info_text(guild),
            ..Outcome::none()
        };
    }
    if let Some(side) = index(action, guild_ui::ACTION_TAB_PREFIX) {
        let requests = guild.select_tab(side).into_iter().collect();
        return Outcome {
            requests,
            texts: info_text(guild),
            ..Outcome::none()
        };
    }
    match index(action, guild_ui::ACTION_SLOT_PREFIX) {
        Some(id) if button == MouseButton::Right && id > 0 => {
            let slot = id - 1;
            let filled = guild
                .contents
                .as_ref()
                .and_then(|c| c.tabs.get(guild.tab))
                .and_then(|t| t.slots.get(slot))
                .is_some_and(Option::is_some);
            if !filled {
                return Outcome::none();
            }
            Outcome::request(GuildBankRequest::Withdraw {
                tab: guild.tab as u8,
                slot: slot as u8,
            })
        }
        _ => Outcome::none(),
    }
}
