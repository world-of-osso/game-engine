use bevy::prelude::*;
use game_engine::network_runtime::messages::MessageReceivers;
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use shared::protocol::{
    CombatEvent, CombatEventType, CombatLogEntrySnapshot, CombatLogEventKindSnapshot,
    CombatLogSnapshot,
};

use crate::networking::MAX_COMBAT_LOG;
use crate::sound::{SpellSoundKind, SpellSoundQueue, SpellSoundRequest};
use game_engine::floating_combat_text::{
    CombatTextKind, FloatingCombatText, FloatingCombatTextStack,
};
use game_engine::status::{CombatLogEntry, CombatLogEventKind, CombatLogStatusSnapshot};

pub(crate) fn receive_combat_log_snapshot(
    mut receivers: MessageReceivers<CombatLogSnapshot>,
    mut snapshot: ResMut<CombatLogStatusSnapshot>,
) {
    for receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.entries = msg.entries.into_iter().map(map_combat_entry).collect();
            if snapshot.entries.len() > MAX_COMBAT_LOG {
                let start = snapshot.entries.len() - MAX_COMBAT_LOG;
                snapshot.entries = snapshot.entries.split_off(start);
            }
        }
    }
}

fn map_combat_entry(entry: CombatLogEntrySnapshot) -> CombatLogEntry {
    CombatLogEntry {
        kind: match entry.kind {
            CombatLogEventKindSnapshot::Damage => CombatLogEventKind::Damage,
            CombatLogEventKindSnapshot::Heal => CombatLogEventKind::Heal,
            CombatLogEventKindSnapshot::Interrupt => CombatLogEventKind::Interrupt,
            CombatLogEventKindSnapshot::AuraApplied => CombatLogEventKind::AuraApplied,
            CombatLogEventKindSnapshot::Death => CombatLogEventKind::Death,
        },
        source: entry.source,
        target: entry.target,
        spell: entry.spell,
        amount: entry.amount,
        aura: entry.aura,
        text: entry.text,
    }
}

fn combat_event_to_log_entry(msg: &CombatEvent) -> CombatLogEntry {
    let kind = combat_event_log_kind(msg.event_type.clone());
    let amount = combat_event_log_amount(msg);
    let text = combat_event_log_text(msg);
    CombatLogEntry {
        kind,
        source: msg.attacker.to_string(),
        target: msg.target.to_string(),
        spell: None,
        amount,
        aura: None,
        text,
    }
}

fn combat_event_log_kind(event_type: CombatEventType) -> CombatLogEventKind {
    match event_type {
        CombatEventType::SpellHeal | CombatEventType::PeriodicHeal => CombatLogEventKind::Heal,
        CombatEventType::Interrupt => CombatLogEventKind::Interrupt,
        CombatEventType::Death => CombatLogEventKind::Death,
        CombatEventType::Respawn => CombatLogEventKind::AuraApplied,
        CombatEventType::MeleeDamage
        | CombatEventType::SpellDamage
        | CombatEventType::PeriodicDamage
        | CombatEventType::CriticalHit
        | CombatEventType::Absorb
        | CombatEventType::Miss
        | CombatEventType::Dodge
        | CombatEventType::Parry
        | CombatEventType::Block => CombatLogEventKind::Damage,
    }
}

fn combat_event_log_amount(msg: &CombatEvent) -> Option<i32> {
    match msg.event_type {
        CombatEventType::MeleeDamage
        | CombatEventType::SpellDamage
        | CombatEventType::PeriodicDamage
        | CombatEventType::CriticalHit
        | CombatEventType::SpellHeal
        | CombatEventType::PeriodicHeal
        | CombatEventType::Absorb => Some(msg.amount.round() as i32),
        CombatEventType::Miss
        | CombatEventType::Dodge
        | CombatEventType::Parry
        | CombatEventType::Block
        | CombatEventType::Interrupt
        | CombatEventType::Death
        | CombatEventType::Respawn => None,
    }
}

fn combat_event_log_text(msg: &CombatEvent) -> String {
    let rounded_amount = msg.amount.round() as i32;
    match msg.event_type {
        CombatEventType::MeleeDamage => {
            format!("{} hit {} for {}", msg.attacker, msg.target, rounded_amount)
        }
        CombatEventType::SpellDamage
        | CombatEventType::PeriodicDamage
        | CombatEventType::CriticalHit => {
            format!(
                "{} damaged {} for {}",
                msg.attacker, msg.target, rounded_amount
            )
        }
        CombatEventType::SpellHeal | CombatEventType::PeriodicHeal => {
            format!(
                "{} healed {} for {}",
                msg.attacker, msg.target, rounded_amount
            )
        }
        CombatEventType::Absorb => format!("{} absorbed {}", msg.target, rounded_amount),
        CombatEventType::Miss if msg.spell_id == 0 => {
            format!("{} missed {}", msg.attacker, msg.target)
        }
        CombatEventType::Miss => format!("{} resisted {}", msg.target, msg.spell_id),
        CombatEventType::Dodge => format!("{} dodged {}", msg.target, msg.attacker),
        CombatEventType::Parry => format!("{} parried {}", msg.target, msg.attacker),
        CombatEventType::Block => format!("{} blocked {}", msg.target, msg.attacker),
        CombatEventType::Interrupt => format!("{} interrupted {}", msg.attacker, msg.target),
        CombatEventType::Death => format!("{} died", msg.target),
        CombatEventType::Respawn => format!("{} respawned", msg.target),
    }
}

fn floating_text_from_combat_event(msg: &CombatEvent) -> Option<(u64, FloatingCombatText)> {
    let kind = match msg.event_type {
        CombatEventType::MeleeDamage => CombatTextKind::PhysicalDamage,
        CombatEventType::SpellDamage | CombatEventType::PeriodicDamage => {
            CombatTextKind::SpellDamage
        }
        CombatEventType::SpellHeal | CombatEventType::PeriodicHeal => CombatTextKind::Heal,
        CombatEventType::Absorb => CombatTextKind::Absorb,
        CombatEventType::Miss if msg.spell_id == 0 => CombatTextKind::Miss,
        CombatEventType::Miss => CombatTextKind::Resist,
        CombatEventType::Dodge => CombatTextKind::Dodge,
        CombatEventType::Parry => CombatTextKind::Parry,
        CombatEventType::Block => CombatTextKind::Block,
        CombatEventType::CriticalHit => CombatTextKind::CritDamage,
        CombatEventType::Interrupt | CombatEventType::Death | CombatEventType::Respawn => {
            return None;
        }
    };
    Some((
        msg.target,
        FloatingCombatText::new(kind, msg.amount.max(0.0).round() as u32),
    ))
}

fn spell_sound_from_combat_event(
    msg: &CombatEvent,
    mirror: &ReplicationMirrorMap,
) -> Option<SpellSoundRequest> {
    if msg.spell_id == 0 {
        return None;
    }
    let kind = match msg.event_type {
        CombatEventType::SpellDamage
        | CombatEventType::PeriodicDamage
        | CombatEventType::CriticalHit => SpellSoundKind::Impact,
        CombatEventType::SpellHeal | CombatEventType::PeriodicHeal => SpellSoundKind::Heal,
        CombatEventType::Miss => SpellSoundKind::Miss,
        CombatEventType::Interrupt => SpellSoundKind::Interrupt,
        CombatEventType::MeleeDamage
        | CombatEventType::Absorb
        | CombatEventType::Dodge
        | CombatEventType::Parry
        | CombatEventType::Block
        | CombatEventType::Death
        | CombatEventType::Respawn => return None,
    };
    Some(SpellSoundRequest {
        spell_id: msg.spell_id,
        kind,
        emitter_entity: Some(spell_sound_emitter_entity(msg, kind, mirror)?),
    })
}

fn spell_sound_emitter_entity(
    msg: &CombatEvent,
    kind: SpellSoundKind,
    mirror: &ReplicationMirrorMap,
) -> Option<Entity> {
    let bits = match kind {
        SpellSoundKind::Impact | SpellSoundKind::Heal | SpellSoundKind::Miss => msg.target,
        SpellSoundKind::Interrupt | SpellSoundKind::CastStart => msg.attacker,
    };
    super::resolve_server_entity(bits, mirror)
}

fn push_floating_text(
    entity: Entity,
    text: FloatingCombatText,
    stacks: &mut Query<&mut FloatingCombatTextStack>,
    existing_entities: &Query<(), ()>,
    commands: &mut Commands,
) {
    if existing_entities.get(entity).is_err() {
        debug!("Ignoring floating combat text for missing main entity {entity:?}");
        return;
    }
    if let Ok(mut stack) = stacks.get_mut(entity) {
        stack.push(text);
        return;
    }
    commands
        .entity(entity)
        .insert(FloatingCombatTextStack { texts: vec![text] });
}

pub(crate) fn receive_combat_events(
    mut receivers: MessageReceivers<CombatEvent>,
    mirror: Res<ReplicationMirrorMap>,
    mut snapshot: ResMut<CombatLogStatusSnapshot>,
    mut stacks: Query<&mut FloatingCombatTextStack>,
    mut spell_sounds: Option<ResMut<SpellSoundQueue>>,
    existing_entities: Query<(), ()>,
    mut commands: Commands,
) {
    for receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            let entry = combat_event_to_log_entry(&msg);
            append_combat_entry(&mut snapshot, entry);
            if let Some(request) = spell_sound_from_combat_event(&msg, &mirror)
                && request
                    .emitter_entity
                    .is_some_and(|entity| existing_entities.contains(entity))
                && let Some(queue) = spell_sounds.as_mut()
            {
                queue.requests.push(request);
            }
            if let Some((target_bits, text)) = floating_text_from_combat_event(&msg)
                && let Some(target) = super::resolve_server_entity(target_bits, &mirror)
            {
                push_floating_text(target, text, &mut stacks, &existing_entities, &mut commands);
            }
        }
    }
}

pub(crate) fn append_combat_entry(snapshot: &mut CombatLogStatusSnapshot, entry: CombatLogEntry) {
    snapshot.entries.push(entry);
    if snapshot.entries.len() > MAX_COMBAT_LOG {
        let overflow = snapshot.entries.len() - MAX_COMBAT_LOG;
        snapshot.entries.drain(0..overflow);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn combat_event(event_type: CombatEventType, amount: f32, spell_id: u32) -> CombatEvent {
        CombatEvent {
            attacker: 1,
            target: 2,
            amount,
            spell_id,
            event_type,
        }
    }

    #[test]
    fn floating_text_from_combat_event_maps_avoidance_labels() {
        let cases = [
            (CombatEventType::Miss, 0, CombatTextKind::Miss, "Miss"),
            (CombatEventType::Dodge, 0, CombatTextKind::Dodge, "Dodge"),
            (CombatEventType::Parry, 0, CombatTextKind::Parry, "Parry"),
            (CombatEventType::Block, 0, CombatTextKind::Block, "Block"),
        ];

        for (event_type, spell_id, expected_kind, expected_text) in cases {
            let (_, text) =
                floating_text_from_combat_event(&combat_event(event_type, 0.0, spell_id))
                    .expect("floating text");
            assert_eq!(text.kind, expected_kind);
            assert_eq!(text.display_text(), expected_text);
        }
    }

    #[test]
    fn floating_text_from_spell_miss_uses_resist_label() {
        let (_, text) =
            floating_text_from_combat_event(&combat_event(CombatEventType::Miss, 0.0, 1337))
                .expect("floating text");
        assert_eq!(text.kind, CombatTextKind::Resist);
        assert_eq!(text.display_text(), "Resist");
    }

    #[test]
    fn floating_text_from_damage_and_heal_events_preserves_amount() {
        let (_, damage) =
            floating_text_from_combat_event(&combat_event(CombatEventType::MeleeDamage, 42.3, 0))
                .expect("damage text");
        assert_eq!(damage.kind, CombatTextKind::PhysicalDamage);
        assert_eq!(damage.amount, 42);

        let (_, heal) =
            floating_text_from_combat_event(&combat_event(CombatEventType::SpellHeal, 73.8, 17))
                .expect("heal text");
        assert_eq!(heal.kind, CombatTextKind::Heal);
        assert_eq!(heal.amount, 74);
    }

    #[test]
    fn floating_text_from_critical_hit_uses_crit_damage_scaling() {
        let (_, crit) =
            floating_text_from_combat_event(&combat_event(CombatEventType::CriticalHit, 99.6, 0))
                .expect("crit text");
        assert_eq!(crit.kind, CombatTextKind::CritDamage);
        assert_eq!(crit.amount, 100);
        assert!(crit.kind.font_scale() > CombatTextKind::PhysicalDamage.font_scale());
    }

    #[test]
    fn floating_text_from_non_display_events_is_ignored() {
        assert!(
            floating_text_from_combat_event(&combat_event(CombatEventType::Death, 0.0, 0))
                .is_none()
        );
        assert!(
            floating_text_from_combat_event(&combat_event(CombatEventType::Respawn, 0.0, 0))
                .is_none()
        );
        assert!(
            floating_text_from_combat_event(&combat_event(CombatEventType::Interrupt, 0.0, 17))
                .is_none()
        );
    }

    fn mirror_fixture() -> ReplicationMirrorMap {
        let mut mirror = ReplicationMirrorMap::default();
        mirror.insert(Entity::from_bits(1), Entity::from_bits(101));
        mirror.insert(Entity::from_bits(2), Entity::from_bits(202));
        mirror
    }

    #[test]
    fn spell_sound_from_combat_event_maps_spell_categories() {
        let mirror = mirror_fixture();
        let cases = [
            (
                CombatEventType::SpellDamage,
                133,
                SpellSoundKind::Impact,
                202,
            ),
            (CombatEventType::SpellHeal, 2061, SpellSoundKind::Heal, 202),
            (CombatEventType::Miss, 17, SpellSoundKind::Miss, 202),
            (
                CombatEventType::Interrupt,
                2139,
                SpellSoundKind::Interrupt,
                101,
            ),
        ];
        for (event, spell_id, kind, main_bits) in cases {
            let sound =
                spell_sound_from_combat_event(&combat_event(event, 40.0, spell_id), &mirror)
                    .unwrap();
            assert_eq!(sound.kind, kind);
            assert_eq!(sound.spell_id, spell_id);
            assert_eq!(sound.emitter_entity, Some(Entity::from_bits(main_bits)));
        }
    }

    #[test]
    fn spell_sound_ignores_non_spell_events_and_unmapped_emitters() {
        let mirror = mirror_fixture();
        for event in [
            CombatEventType::MeleeDamage,
            CombatEventType::SpellDamage,
            CombatEventType::Death,
        ] {
            assert!(
                spell_sound_from_combat_event(&combat_event(event, 22.0, 0), &mirror).is_none()
            );
        }
        assert!(
            spell_sound_from_combat_event(
                &combat_event(CombatEventType::SpellDamage, 22.0, 133),
                &ReplicationMirrorMap::default(),
            )
            .is_none()
        );
    }

    #[test]
    fn combat_visuals_use_main_identity_while_logs_keep_server_identity() {
        use game_engine::network_runtime::messages::Inbox;
        let mut app = App::new();
        let target = app.world_mut().spawn_empty().id();
        let unrelated = app.world_mut().spawn_empty().id();
        let server = Entity::from_bits(202);
        let mut mirror = ReplicationMirrorMap::default();
        mirror.insert(server, target);
        app.insert_resource(mirror);
        app.init_resource::<CombatLogStatusSnapshot>();
        app.init_resource::<SpellSoundQueue>();
        let mut mapped = combat_event(CombatEventType::SpellDamage, 42.0, 133);
        mapped.target = server.to_bits();
        let mut unmapped = mapped.clone();
        unmapped.target = unrelated.to_bits();
        app.insert_resource(Inbox::new(vec![mapped, unmapped]));
        app.world_mut()
            .run_system_once(receive_combat_events)
            .unwrap();
        let stack = app.world().get::<FloatingCombatTextStack>(target).unwrap();
        assert_eq!(stack.texts.len(), 1);
        assert_eq!(stack.texts[0].amount, 42);
        assert!(
            app.world()
                .get::<FloatingCombatTextStack>(unrelated)
                .is_none()
        );
        let sounds = &app.world().resource::<SpellSoundQueue>().requests;
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].emitter_entity, Some(target));
        let log = &app.world().resource::<CombatLogStatusSnapshot>().entries;
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].target, server.to_bits().to_string());
        assert_eq!(log[1].target, unrelated.to_bits().to_string());
    }

    #[test]
    fn push_floating_text_ignores_unknown_entity() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.world_mut()
            .run_system_once(
                |mut commands: Commands,
                 mut stacks: Query<&mut FloatingCombatTextStack>,
                 existing_entities: Query<(), ()>| {
                    push_floating_text(
                        Entity::from_bits(999_999),
                        FloatingCombatText::new(CombatTextKind::Miss, 0),
                        &mut stacks,
                        &existing_entities,
                        &mut commands,
                    );
                },
            )
            .expect("push floating text");
        app.update();

        assert!(
            app.world().get_entity(Entity::from_bits(999_999)).is_err(),
            "malformed combat target should not spawn a fake entity"
        );
    }
}
