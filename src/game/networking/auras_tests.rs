use std::sync::mpsc;
use std::time::Duration;

use bevy::time::TimeUpdateStrategy;
use game_engine::buff_data::DebuffType;
use game_engine::network_runtime::messages::ConnectionSender;
use game_engine::network_runtime::worker::NetworkCommand;
use game_engine::spell_catalog::{CatalogSpell, SpellCatalogData, SpellCatalogState};
use shared::components::AuraView;

use super::*;

const PLAYER_SERVER: u64 = 0x1_0000_0010;
const WOLF_SERVER: u64 = 0x1_0000_0020;

fn catalog() -> SpellCatalog {
    let spell = |id: u32, name: &str, icon: u32| CatalogSpell {
        id,
        name: name.into(),
        icon_fdid: icon,
        ..Default::default()
    };
    SpellCatalog {
        state: SpellCatalogState::Ready(SpellCatalogData::from_spells(vec![
            spell(21562, "Power Word: Fortitude", 135987),
            spell(589, "Shadow Word: Pain", 136207),
            spell(20217, "Blessing of Kings", 135995),
        ])),
    }
}

fn view(instance_id: u32, spell_id: u32, caster: u64, harmful: bool, flags: u16) -> AuraView {
    AuraView {
        instance_id,
        spell_id,
        caster: Some(caster),
        stacks: 0,
        charges: 0,
        duration_ms: if flags & AuraView::FLAG_PASSIVE != 0 {
            0
        } else {
            65_000
        },
        remaining_ms: if flags & AuraView::FLAG_PASSIVE != 0 {
            0
        } else {
            61_000
        },
        harmful,
        dispel_type: if harmful { 1 } else { 0 },
        flags,
    }
}

struct Fixture {
    app: App,
    player: Entity,
    wolf: Entity,
}

fn fixture() -> Fixture {
    let mut app = App::new();
    app.add_plugins(bevy::time::TimePlugin);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(2)));
    app.insert_resource(catalog());
    app.init_resource::<ConnectionSender>();
    app.add_plugins(AuraSyncPlugin);
    let player = app.world_mut().spawn(LocalPlayer).id();
    let wolf = app
        .world_mut()
        .spawn(Npc {
            template_id: 299,
            name: "Young Wolf".into(),
        })
        .id();
    let mut mirrors = app.world_mut().resource_mut::<ReplicationMirrorMap>();
    mirrors.insert(Entity::from_bits(PLAYER_SERVER), player);
    mirrors.insert(Entity::from_bits(WOLF_SERVER), wolf);
    Fixture { app, player, wolf }
}

#[test]
fn local_player_auras_fill_aura_state_without_hidden_passives() {
    let Fixture {
        mut app, player, ..
    } = fixture();
    app.world_mut().entity_mut(player).insert(UnitAuras {
        auras: vec![
            view(1, 21562, PLAYER_SERVER, false, AuraView::FLAG_FROM_PLAYER),
            view(2, 589, WOLF_SERVER, true, 0),
            view(
                3,
                20217,
                PLAYER_SERVER,
                false,
                AuraView::FLAG_PASSIVE | AuraView::FLAG_HIDDEN,
            ),
        ],
    });
    app.update();
    let state = app.world().resource::<AuraState>();
    let buffs: Vec<&str> = state.buffs().map(|aura| aura.name.as_str()).collect();
    assert_eq!(buffs, ["Power Word: Fortitude"]);
    let debuff = state.debuffs().next().expect("one debuff");
    assert_eq!(state.debuffs().count(), 1);
    assert_eq!(debuff.name, "Shadow Word: Pain");
    assert_eq!(debuff.source, "Young Wolf");
    assert_eq!(debuff.debuff_type, DebuffType::Magic);
    assert!(state.buffs().next().unwrap().from_local_player);
    assert!(app.world().get::<UnitAuraState>(player).is_none());
}

#[test]
fn timers_count_down_between_server_updates() {
    let Fixture {
        mut app, player, ..
    } = fixture();
    app.world_mut().entity_mut(player).insert(UnitAuras {
        auras: vec![view(1, 21562, PLAYER_SERVER, false, 0)],
    });
    app.update();
    let first = app.world().resource::<AuraState>().auras[0].remaining;
    assert_eq!(
        app.world().resource::<AuraState>().auras[0].timer_text(),
        "2 m"
    );
    app.update();
    app.update();
    let aura = &app.world().resource::<AuraState>().auras[0];
    assert_eq!(first - aura.remaining, 4.0);
    assert_eq!(aura.timer_text(), "1 m");
}

#[test]
fn other_units_get_unit_aura_state_marking_the_local_players_debuffs() {
    let Fixture { mut app, wolf, .. } = fixture();
    app.world_mut().entity_mut(wolf).insert(UnitAuras {
        auras: vec![
            view(7, 589, WOLF_SERVER, true, 0),
            view(8, 589, PLAYER_SERVER, true, AuraView::FLAG_FROM_PLAYER),
        ],
    });
    app.update();
    let unit = app.world().get::<UnitAuraState>(wolf).expect("wolf auras");
    let order: Vec<(u32, bool)> = unit
        .auras
        .iter()
        .map(|aura| (aura.instance_id, aura.from_local_player))
        .collect();
    assert_eq!(order, [(8, true), (7, false)]);
    assert!(app.world().resource::<AuraState>().auras.is_empty());

    app.world_mut().entity_mut(wolf).remove::<UnitAuras>();
    app.update();
    assert!(app.world().get::<UnitAuraState>(wolf).is_none());
}

#[test]
fn cancel_request_sends_cancel_aura_to_the_server() {
    let Fixture { mut app, .. } = fixture();
    let (sender, commands) = mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));
    app.world_mut()
        .write_message(CancelAuraRequest { spell_id: 21562 });
    app.update();
    assert!(matches!(commands.try_recv(), Ok(NetworkCommand::Apply(_))));
    assert!(commands.try_recv().is_err());
}
