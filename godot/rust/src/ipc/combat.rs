//! `combat log` and `combat recap` in the original response text (src/ipc/format.rs
//! `format_combat_log`, `format_combat_recap`) over the received `CombatEvent`s, mapped
//! as the original `combat_event_to_log_entry` does (src/game/networking/
//! messages_combat.rs): source and target are server entity bits.
use std::collections::VecDeque;

use game_engine_network::ipc_wire::{Request, Response};
use shared::protocol::{CombatEvent, CombatEventType};

/// The original `MAX_COMBAT_LOG`.
pub(crate) const COMBAT_LOG_KEEP: usize = 200;

impl crate::GameClient {
    pub(crate) fn combat_request(&mut self, request: Request) -> Result<Response, Request> {
        Ok(Response::Text(match request {
            Request::CombatLog { lines } => format_combat_log(&self.ipc_combat_events, lines),
            Request::CombatRecap { target } => {
                format_combat_recap(&self.ipc_combat_events, target.as_deref())
            }
            request => return self.character_request(request),
        }))
    }

    /// Keeps the newest `COMBAT_LOG_KEEP` combat events for IPC.
    pub(crate) fn record_combat_event(&mut self, event: &CombatEvent) {
        if self.ipc_combat_events.len() == COMBAT_LOG_KEEP {
            self.ipc_combat_events.pop_front();
        }
        self.ipc_combat_events.push_back(event.clone());
    }
}

fn format_combat_log(events: &VecDeque<CombatEvent>, lines: u16) -> String {
    let selected: Vec<_> = events
        .iter()
        .rev()
        .take(usize::from(lines).max(1))
        .collect();
    if selected.is_empty() {
        return "combat_log: 0\n-".into();
    }
    let text: Vec<_> = selected.iter().map(|event| format_entry(event)).collect();
    format!("combat_log: {}\n{}", selected.len(), text.join("\n"))
}

fn format_combat_recap(events: &VecDeque<CombatEvent>, target: Option<&str>) -> String {
    let target = target.unwrap_or("current").to_ascii_lowercase();
    let filtered: Vec<_> = events
        .iter()
        .rev()
        .filter(|event| {
            target == "current"
                || event.target.to_string() == target
                || event.attacker.to_string() == target
        })
        .take(10)
        .collect();
    if filtered.is_empty() {
        return format!("combat_recap target={target}: 0\n-");
    }
    let text: Vec<_> = filtered.iter().map(|event| format_entry(event)).collect();
    format!(
        "combat_recap target={target}: {}\n{}",
        filtered.len(),
        text.join("\n")
    )
}

fn format_entry(event: &CombatEvent) -> String {
    let amount = event_amount(event).map_or_else(|| "-".into(), |amount| amount.to_string());
    format!(
        "{} src={} dst={} spell=- amount={amount} aura=- text={}",
        event_kind(event),
        event.attacker,
        event.target,
        event_text(event),
    )
}

fn event_kind(event: &CombatEvent) -> &'static str {
    match event.event_type {
        CombatEventType::SpellHeal | CombatEventType::PeriodicHeal => "heal",
        CombatEventType::Interrupt => "interrupt",
        CombatEventType::Death => "death",
        CombatEventType::Respawn => "aura",
        CombatEventType::MeleeDamage
        | CombatEventType::SpellDamage
        | CombatEventType::PeriodicDamage
        | CombatEventType::CriticalHit
        | CombatEventType::Absorb
        | CombatEventType::Miss
        | CombatEventType::Dodge
        | CombatEventType::Parry
        | CombatEventType::Block => "damage",
    }
}

fn event_amount(event: &CombatEvent) -> Option<i32> {
    match event.event_type {
        CombatEventType::MeleeDamage
        | CombatEventType::SpellDamage
        | CombatEventType::PeriodicDamage
        | CombatEventType::CriticalHit
        | CombatEventType::SpellHeal
        | CombatEventType::PeriodicHeal
        | CombatEventType::Absorb => Some(event.amount.round() as i32),
        CombatEventType::Miss
        | CombatEventType::Dodge
        | CombatEventType::Parry
        | CombatEventType::Block
        | CombatEventType::Interrupt
        | CombatEventType::Death
        | CombatEventType::Respawn => None,
    }
}

fn event_text(event: &CombatEvent) -> String {
    let (attacker, target) = (event.attacker, event.target);
    let amount = event.amount.round() as i32;
    match event.event_type {
        CombatEventType::MeleeDamage => format!("{attacker} hit {target} for {amount}"),
        CombatEventType::SpellDamage
        | CombatEventType::PeriodicDamage
        | CombatEventType::CriticalHit => format!("{attacker} damaged {target} for {amount}"),
        CombatEventType::SpellHeal | CombatEventType::PeriodicHeal => {
            format!("{attacker} healed {target} for {amount}")
        }
        CombatEventType::Absorb => format!("{target} absorbed {amount}"),
        CombatEventType::Miss if event.spell_id == 0 => format!("{attacker} missed {target}"),
        CombatEventType::Miss => format!("{target} resisted {}", event.spell_id),
        CombatEventType::Dodge => format!("{target} dodged {attacker}"),
        CombatEventType::Parry => format!("{target} parried {attacker}"),
        CombatEventType::Block => format!("{target} blocked {attacker}"),
        CombatEventType::Interrupt => format!("{attacker} interrupted {target}"),
        CombatEventType::Death => format!("{target} died"),
        CombatEventType::Respawn => format!("{target} respawned"),
    }
}
