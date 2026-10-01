use bevy::prelude::*;
use game_engine::network_runtime::messages::MessageReceivers;
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use shared::protocol::{
    CombatEvent, CombatEventType, CombatLogEntrySnapshot, CombatLogEvent,
    CombatLogEventKindSnapshot, CombatLogKind, CombatLogSnapshot, MissKind,
};

use crate::networking::MAX_COMBAT_LOG;
use crate::sound::{SpellSoundKind, SpellSoundQueue, SpellSoundRequest};
use game_engine::casting_data::CastingState;
use game_engine::floating_combat_text::{
    CombatTextKind, FloatingCombatText, FloatingCombatTextStack,
};
use game_engine::spell_event_data::{OutcomeSound, spell_outcome};
use game_engine::status::{CombatLogEntry, CombatLogEventKind, CombatLogStatusSnapshot};
use game_engine::ui::chat_frame::{CombatLogChat, UNKNOWN_NAME, combat_log_line};
use shared::components::{Npc, Player as NetPlayer};

use crate::networking::LocalPlayer;

/// Floating combat text dedupe. Neither `CombatEvent` nor `CombatLogEvent` carries a hit id,
/// so the same hit cannot be matched across both. Rule: once the connection has delivered any
/// `CombatLogEvent`, it is the only FCT producer and `CombatEvent` feeds the combat log only.
/// Before that, `CombatEvent` (today's melee/death stream) keeps feeding FCT.
/// Retire the `CombatEvent` FCT path when the server stops sending `CombatEvent`.
#[derive(Resource, Default)]
pub(crate) struct CombatTextSource {
    pub(crate) combat_log_seen: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CombatTextProducer {
    CombatEvent,
    CombatLog,
}

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
        CombatEventType::CriticalHit if msg.spell_id == 0 => CombatTextKind::PhysicalDamage,
        CombatEventType::CriticalHit => CombatTextKind::SpellDamage,
        CombatEventType::Interrupt | CombatEventType::Death | CombatEventType::Respawn => {
            return None;
        }
    };
    let crit = msg.event_type == CombatEventType::CriticalHit;
    Some((
        msg.target,
        combat_text(kind, msg.amount.round() as i32, crit),
    ))
}

fn floating_text_from_combat_log(msg: &CombatLogEvent) -> Option<(u64, FloatingCombatText)> {
    const PHYSICAL_SCHOOL: u32 = 1;
    let kind = match msg.kind {
        CombatLogKind::Damage if msg.amount <= 0 && msg.absorbed > 0 => CombatTextKind::Absorb,
        CombatLogKind::Damage if msg.school_mask == PHYSICAL_SCHOOL => {
            CombatTextKind::PhysicalDamage
        }
        CombatLogKind::Damage => CombatTextKind::SpellDamage,
        CombatLogKind::Heal => CombatTextKind::Heal,
        CombatLogKind::Miss(miss) => miss_text_kind(miss),
        _ => return None,
    };
    Some((msg.target?, combat_text(kind, msg.amount, msg.crit)))
}

fn miss_text_kind(miss: MissKind) -> CombatTextKind {
    match miss {
        MissKind::Miss => CombatTextKind::Miss,
        MissKind::Dodge => CombatTextKind::Dodge,
        MissKind::Parry => CombatTextKind::Parry,
        MissKind::Block => CombatTextKind::Block,
        MissKind::Resist => CombatTextKind::Resist,
        MissKind::Immune => CombatTextKind::Immune,
        MissKind::Evade => CombatTextKind::Evade,
        MissKind::Absorb => CombatTextKind::Absorb,
        MissKind::Deflect => CombatTextKind::Deflect,
        MissKind::Reflect => CombatTextKind::Reflect,
    }
}

/// The one conversion both producers use.
fn combat_text(kind: CombatTextKind, amount: i32, crit: bool) -> FloatingCombatText {
    let amount = amount.max(0) as u32;
    if crit {
        FloatingCombatText::critical(kind, amount)
    } else {
        FloatingCombatText::new(kind, amount)
    }
}

fn producer_feeds_combat_text(producer: CombatTextProducer, source: &CombatTextSource) -> bool {
    producer == CombatTextProducer::CombatLog || !source.combat_log_seen
}

fn spell_sound_from_combat_event(
    msg: &CombatEvent,
    mirror: &ReplicationMirrorMap,
) -> Option<SpellSoundRequest> {
    let (outcome, emitter) = spell_outcome(msg)?;
    let kind = match outcome {
        OutcomeSound::Impact => SpellSoundKind::Impact,
        OutcomeSound::Heal => SpellSoundKind::Heal,
        OutcomeSound::Miss => SpellSoundKind::Miss,
        OutcomeSound::Interrupt => SpellSoundKind::Interrupt,
    };
    Some(SpellSoundRequest {
        spell_id: msg.spell_id,
        kind,
        emitter_entity: Some(super::resolve_server_entity(emitter, mirror)?),
    })
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
    source: Res<CombatTextSource>,
    existing_entities: Query<(), ()>,
    mut commands: Commands,
) {
    let feeds_text = producer_feeds_combat_text(CombatTextProducer::CombatEvent, &source);
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
            if feeds_text
                && let Some((target_bits, text)) = floating_text_from_combat_event(&msg)
                && let Some(target) = super::resolve_server_entity(target_bits, &mirror)
            {
                push_floating_text(target, text, &mut stacks, &existing_entities, &mut commands);
            }
        }
    }
}

pub(crate) fn receive_combat_log_events(
    mut receivers: MessageReceivers<CombatLogEvent>,
    mirror: Res<ReplicationMirrorMap>,
    unit_names: Query<(Option<&Npc>, Option<&NetPlayer>)>,
    mut combat_chat: ResMut<CombatLogChat>,
    mut source: ResMut<CombatTextSource>,
    mut stacks: Query<&mut FloatingCombatTextStack>,
    local_player: Query<Entity, With<LocalPlayer>>,
    mut casting: Option<ResMut<CastingState>>,
    existing_entities: Query<(), ()>,
    mut commands: Commands,
) {
    for receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            source.combat_log_seen = true;
            let target = msg
                .target
                .and_then(|bits| super::resolve_server_entity(bits, &mirror));
            let source_entity = msg
                .source
                .and_then(|bits| super::resolve_server_entity(bits, &mirror));
            combat_chat.push(
                game_engine::chat_data::now_timestamp(),
                combat_log_line(
                    &msg,
                    &unit_name(source_entity, &unit_names),
                    &unit_name(target, &unit_names),
                ),
            );
            if msg.kind == CombatLogKind::Interrupt
                && target.is_some_and(|target| local_player.contains(target))
                && let Some(casting) = casting.as_mut()
            {
                casting.interrupt();
            }
            if let Some((_, text)) = floating_text_from_combat_log(&msg)
                && let Some(target) = target
            {
                push_floating_text(target, text, &mut stacks, &existing_entities, &mut commands);
            }
        }
    }
}

/// Display name of a replicated unit: NPC name, then player name, else "Unknown".
fn unit_name(entity: Option<Entity>, names: &Query<(Option<&Npc>, Option<&NetPlayer>)>) -> String {
    let Some((npc, player)) = entity.and_then(|entity| names.get(entity).ok()) else {
        return UNKNOWN_NAME.to_string();
    };
    npc.map(|npc| npc.name.clone())
        .or_else(|| player.map(|player| player.name.clone()))
        .unwrap_or_else(|| UNKNOWN_NAME.to_string())
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
    fn floating_text_from_critical_hit_keeps_school_colour_and_crit_scaling() {
        let (_, crit) =
            floating_text_from_combat_event(&combat_event(CombatEventType::CriticalHit, 99.6, 0))
                .expect("crit text");
        assert_eq!(crit.kind, CombatTextKind::PhysicalDamage);
        assert_eq!(crit.amount, 100);
        assert!(crit.crit);
        assert!(crit.font_scale() > 1.0);
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
    fn spell_sound_adapter_resolves_original_server_emitter() {
        let sound = spell_sound_from_combat_event(
            &combat_event(CombatEventType::Interrupt, 40.0, 2139),
            &mirror_fixture(),
        )
        .unwrap();
        assert_eq!(sound.kind, SpellSoundKind::Interrupt);
        assert_eq!(sound.spell_id, 2139);
        assert_eq!(sound.emitter_entity, Some(Entity::from_bits(101)));
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
        app.init_resource::<CombatTextSource>();
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

    fn combat_log(
        kind: CombatLogKind,
        target: Option<u64>,
        amount: i32,
        crit: bool,
    ) -> CombatLogEvent {
        CombatLogEvent {
            source: Some(1),
            target,
            spell_id: Some(133),
            school_mask: 4,
            amount,
            overflow: 0,
            absorbed: 0,
            resisted: 0,
            blocked: 0,
            crit,
            glancing: false,
            periodic: false,
            kind,
        }
    }

    struct CombatTextApp {
        app: App,
        target: Entity,
        server: Entity,
    }

    fn combat_text_app() -> CombatTextApp {
        use game_engine::network_runtime::messages::Inbox;
        let mut app = App::new();
        let target = app.world_mut().spawn_empty().id();
        let server = Entity::from_bits(303);
        let mut mirror = ReplicationMirrorMap::default();
        mirror.insert(server, target);
        app.insert_resource(mirror);
        app.init_resource::<CombatLogStatusSnapshot>();
        app.init_resource::<SpellSoundQueue>();
        app.init_resource::<CombatTextSource>();
        app.init_resource::<CastingState>();
        app.init_resource::<CombatLogChat>();
        app.init_resource::<Inbox<CombatLogEvent>>();
        app.init_resource::<Inbox<CombatEvent>>();
        CombatTextApp {
            app,
            target,
            server,
        }
    }

    fn deliver<M: lightyear::prelude::Message>(app: &mut App, messages: Vec<M>) {
        use game_engine::network_runtime::messages::Inbox;
        app.insert_resource(Inbox::new(messages));
    }

    fn stack_texts(app: &App, entity: Entity) -> Vec<FloatingCombatText> {
        app.world()
            .get::<FloatingCombatTextStack>(entity)
            .map(|stack| stack.texts.clone())
            .unwrap_or_default()
    }

    #[test]
    fn combat_log_spell_crit_produces_one_large_spell_coloured_text_that_expires() {
        let CombatTextApp {
            mut app,
            target,
            server,
        } = combat_text_app();
        deliver(
            &mut app,
            vec![combat_log(
                CombatLogKind::Damage,
                Some(server.to_bits()),
                1234,
                true,
            )],
        );
        app.world_mut()
            .run_system_once(receive_combat_log_events)
            .unwrap();
        let texts = stack_texts(&app, target);
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].kind, CombatTextKind::SpellDamage);
        assert_eq!(texts[0].display_text(), "1234");
        assert!(texts[0].crit);
        assert_eq!(texts[0].font_scale(), 1.5);
        let mut stack = app
            .world_mut()
            .get_mut::<FloatingCombatTextStack>(target)
            .unwrap();
        stack.tick(texts[0].lifetime);
        assert_eq!(stack.active_count(), 0);
    }

    #[test]
    fn combat_log_maps_heal_physical_miss_and_absorb_kinds() {
        let heal = combat_log(CombatLogKind::Heal, Some(2), 500, false);
        let physical = CombatLogEvent {
            school_mask: 1,
            ..combat_log(CombatLogKind::Damage, Some(2), 80, false)
        };
        let immune = combat_log(CombatLogKind::Miss(MissKind::Immune), Some(2), 0, false);
        let absorbed = CombatLogEvent {
            absorbed: 300,
            ..combat_log(CombatLogKind::Damage, Some(2), 0, false)
        };
        let cases = [
            (heal, CombatTextKind::Heal, "500"),
            (physical, CombatTextKind::PhysicalDamage, "80"),
            (immune, CombatTextKind::Immune, "Immune"),
            (absorbed, CombatTextKind::Absorb, "Absorb"),
        ];
        for (event, kind, text) in cases {
            let (target, fct) = floating_text_from_combat_log(&event).expect("text");
            assert_eq!(target, 2);
            assert_eq!(fct.kind, kind);
            assert_eq!(fct.display_text(), text);
        }
        for kind in [
            CombatLogKind::AuraApplied,
            CombatLogKind::Death,
            CombatLogKind::Energize,
        ] {
            assert!(floating_text_from_combat_log(&combat_log(kind, Some(2), 5, false)).is_none());
        }
    }

    #[test]
    fn combat_event_stops_feeding_text_once_combat_log_arrives() {
        let CombatTextApp {
            mut app,
            target,
            server,
        } = combat_text_app();
        let mut melee = combat_event(CombatEventType::MeleeDamage, 42.0, 0);
        melee.target = server.to_bits();
        deliver(&mut app, vec![melee.clone()]);
        app.world_mut()
            .run_system_once(receive_combat_events)
            .unwrap();
        assert_eq!(
            stack_texts(&app, target).len(),
            1,
            "CombatEvent feeds FCT before log"
        );
        let mut log = combat_log(CombatLogKind::Damage, Some(server.to_bits()), 42, false);
        log.school_mask = 1;
        deliver(&mut app, vec![log]);
        app.world_mut()
            .run_system_once(receive_combat_log_events)
            .unwrap();
        deliver(&mut app, vec![melee]);
        app.world_mut()
            .run_system_once(receive_combat_events)
            .unwrap();
        assert_eq!(
            stack_texts(&app, target).len(),
            2,
            "same hit from both producers adds one"
        );
        assert_eq!(
            app.world()
                .resource::<CombatLogStatusSnapshot>()
                .entries
                .len(),
            2,
            "CombatEvent still feeds the combat log"
        );
    }

    #[test]
    fn combat_log_interrupt_on_local_player_shows_interrupted_cast_bar() {
        use game_engine::casting_data::{ActiveCast, CastType};
        let CombatTextApp {
            mut app,
            target,
            server,
        } = combat_text_app();
        app.world_mut().entity_mut(target).insert(LocalPlayer);
        app.world_mut()
            .resource_mut::<CastingState>()
            .apply_server_cast(ActiveCast {
                spell_name: "Fireball".into(),
                spell_id: 133,
                icon_fdid: 0,
                cast_type: CastType::Cast,
                interruptible: true,
                duration: 2.5,
                elapsed: 1.0,
            });
        deliver(
            &mut app,
            vec![combat_log(
                CombatLogKind::Interrupt,
                Some(server.to_bits()),
                0,
                false,
            )],
        );
        app.world_mut()
            .run_system_once(receive_combat_log_events)
            .unwrap();
        let casting = app.world().resource::<CastingState>();
        assert!(casting.active.is_none());
        assert!(casting.interrupted.is_some());
        assert!(stack_texts(&app, target).is_empty());
    }
}
