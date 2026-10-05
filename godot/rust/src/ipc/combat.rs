//! IPC combat captures read the unfiltered received combat log on the account.
//! Source and target are server entity bits; timestamps are server Unix milliseconds.
use std::collections::VecDeque;

use game_engine_network::ipc_wire::{Request, Response};
use shared::protocol::{CombatLogEvent, CombatLogKind};

impl crate::GameClient {
    pub(crate) fn combat_request(&mut self, request: Request) -> Result<Response, Request> {
        let text = match request {
            Request::CombatLog { lines } => format_combat_log(
                &self.account.combat_log,
                lines,
                crate::chat::spell_namer(self.spells.catalog()),
            ),
            Request::CombatRecap { target } => format_combat_recap(
                &self.account.combat_log,
                target.as_deref(),
                crate::chat::spell_namer(self.spells.catalog()),
            ),
            request => return self.character_request(request),
        };
        Ok(Response::Text(text))
    }
}

fn format_combat_log(
    events: &VecDeque<CombatLogEvent>,
    lines: u16,
    spell_name: impl Fn(u32) -> String,
) -> String {
    let selected: Vec<_> = events
        .iter()
        .rev()
        .take(usize::from(lines).max(1))
        .collect();
    if selected.is_empty() {
        return "combat_log: 0\n-".into();
    }
    let text: Vec<_> = selected
        .iter()
        .map(|event| format_entry(event, &spell_name))
        .collect();
    format!("combat_log: {}\n{}", selected.len(), text.join("\n"))
}

fn format_combat_recap(
    events: &VecDeque<CombatLogEvent>,
    target: Option<&str>,
    spell_name: impl Fn(u32) -> String,
) -> String {
    let target = target.unwrap_or("current").to_ascii_lowercase();
    let filtered: Vec<_> = events
        .iter()
        .rev()
        .filter(|event| {
            target == "current"
                || event.target.is_some_and(|id| id.to_string() == target)
                || event.source.is_some_and(|id| id.to_string() == target)
        })
        .take(10)
        .collect();
    if filtered.is_empty() {
        return format!("combat_recap target={target}: 0\n-");
    }
    let text: Vec<_> = filtered
        .iter()
        .map(|event| format_entry(event, &spell_name))
        .collect();
    format!(
        "combat_recap target={target}: {}\n{}",
        filtered.len(),
        text.join("\n")
    )
}

fn format_entry(event: &CombatLogEvent, spell_name: &impl Fn(u32) -> String) -> String {
    let amount = event_amount(event).map_or_else(|| "-".into(), |amount| amount.to_string());
    let spell = event
        .spell_id
        .map_or_else(|| "-".into(), |id| id.to_string());
    let name = event
        .spell_id
        .map_or_else(|| "-".into(), |id| format!("{:?}", spell_name(id)));
    let extra = event.extra_spell_id.map_or_else(String::new, |id| {
        format!(" extra_spell={id} extra_spell_name={:?}", spell_name(id))
    });
    format!(
        "{} src={} dst={} spell={spell} amount={amount} aura=- text={} timestamp_unix_ms={} spell_name={name}{extra}",
        event_kind(event),
        event.source.map_or_else(|| "-".into(), |id| id.to_string()),
        event.target.map_or_else(|| "-".into(), |id| id.to_string()),
        event_text(event),
        event.timestamp_unix_ms,
    )
}

fn event_kind(event: &CombatLogEvent) -> &'static str {
    match event.kind {
        CombatLogKind::Heal => "heal",
        CombatLogKind::Interrupt => "interrupt",
        CombatLogKind::Death => "death",
        CombatLogKind::AuraApplied | CombatLogKind::AuraRemoved | CombatLogKind::AuraRefreshed => {
            "aura"
        }
        CombatLogKind::Damage | CombatLogKind::Miss(_) | CombatLogKind::Environmental(_) => {
            "damage"
        }
        CombatLogKind::Energize => "energize",
        CombatLogKind::Dispel => "dispel",
        CombatLogKind::CastStart => "cast_start",
        CombatLogKind::CastSuccess => "cast_success",
    }
}

fn event_amount(event: &CombatLogEvent) -> Option<i32> {
    match event.kind {
        CombatLogKind::Damage
        | CombatLogKind::Heal
        | CombatLogKind::Energize
        | CombatLogKind::Environmental(_) => Some(event.amount),
        _ => None,
    }
}

fn event_text(event: &CombatLogEvent) -> String {
    let source = event.source.map_or_else(|| "-".into(), |id| id.to_string());
    let target = event.target.map_or_else(|| "-".into(), |id| id.to_string());
    let amount = event.amount;
    match event.kind {
        CombatLogKind::Damage if event.spell_id.is_none() => {
            format!("{source} hit {target} for {amount}")
        }
        CombatLogKind::Damage => format!("{source} damaged {target} for {amount}"),
        CombatLogKind::Heal => format!("{source} healed {target} for {amount}"),
        CombatLogKind::Miss(kind) => format!("{source} missed {target}: {kind:?}"),
        CombatLogKind::Interrupt => format!("{source} interrupted {target}"),
        CombatLogKind::Death => format!("{target} died"),
        CombatLogKind::Environmental(kind) => format!("{target} took {amount} {kind:?} damage"),
        kind => format!("{source} {kind:?} {target}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn damage(source: u64, spell_id: u32) -> CombatLogEvent {
        CombatLogEvent {
            source: Some(source),
            target: Some(99),
            spell_id: Some(spell_id),
            school_mask: 4,
            amount: 12,
            overflow: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit: false,
            glancing: false,
            periodic: false,
            extra_spell_id: None,
            timestamp_unix_ms: 1791210000123,
            kind: CombatLogKind::Damage,
        }
    }

    fn spell_name(id: u32) -> String {
        match id {
            3110 => "Firebolt".into(),
            686 => "Shadow Bolt".into(),
            19647 => "Spell Lock".into(),
            _ => panic!("unexpected spell {id}"),
        }
    }

    #[test]
    fn combatipc_pet_spell_damage_keeps_spell_id_in_ipc_log() {
        let log = format_combat_log(&VecDeque::from([damage(42, 3110)]), 20, spell_name);
        assert_eq!(
            log,
            "combat_log: 1\ndamage src=42 dst=99 spell=3110 amount=12 aura=- text=42 damaged 99 for 12 timestamp_unix_ms=1791210000123 spell_name=\"Firebolt\""
        );
    }

    #[test]
    fn combatipc_owner_spell_damage_is_in_log_and_recap() {
        let events = VecDeque::from([damage(42, 3110), damage(1, 686)]);
        let log = format_combat_log(&events, 1, spell_name);
        assert_eq!(
            log,
            "combat_log: 1\ndamage src=1 dst=99 spell=686 amount=12 aura=- text=1 damaged 99 for 12 timestamp_unix_ms=1791210000123 spell_name=\"Shadow Bolt\""
        );
        let recap = format_combat_recap(&events, Some("42"), spell_name);
        assert!(
            recap.starts_with("combat_recap target=42: 1\ndamage src=42"),
            "{recap}"
        );
        assert!(recap.ends_with("spell_name=\"Firebolt\""), "{recap}");
    }

    #[test]
    fn combatipc_interrupt_prints_the_extra_spell_id_and_name() {
        let mut event = damage(42, 19647);
        event.kind = CombatLogKind::Interrupt;
        event.extra_spell_id = Some(686);
        let log = format_combat_log(&VecDeque::from([event]), 20, spell_name);
        assert_eq!(
            log,
            "combat_log: 1\ninterrupt src=42 dst=99 spell=19647 amount=- aura=- text=42 interrupted 99 timestamp_unix_ms=1791210000123 spell_name=\"Spell Lock\" extra_spell=686 extra_spell_name=\"Shadow Bolt\""
        );
    }

    #[test]
    fn combatipc_melee_without_spell_keeps_existing_fields() {
        let mut event = damage(42, 3110);
        event.spell_id = None;
        let log = format_combat_log(&VecDeque::from([event]), 20, spell_name);
        assert_eq!(
            log,
            "combat_log: 1\ndamage src=42 dst=99 spell=- amount=12 aura=- text=42 hit 99 for 12 timestamp_unix_ms=1791210000123 spell_name=-"
        );
    }
}
