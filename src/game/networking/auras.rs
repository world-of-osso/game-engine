//! Replicated `UnitAuras` become the client aura model: `AuraState` for the local
//! player, `UnitAuraState` on every other unit. Timers count down from the
//! `remaining_ms` received with each aura-set change.

use bevy::prelude::*;
use game_engine::buff_data::{AuraCasterLookup, AuraState, UnitAuraState, aura_instances};
use game_engine::network_runtime::messages::MessageSenders;
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use game_engine::player_spells::{ActiveSpecialization, KnownSpells};
use game_engine::spell_catalog::{SpellCatalog, SpellTextContext};
use shared::components::{Npc, Player as NetPlayer, UnitAuras};
use shared::protocol::{CancelAura, CombatChannel};

use crate::networking::LocalPlayer;

/// UI request to cancel one of the local player's buffs.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CancelAuraRequest {
    pub spell_id: u32,
}

pub struct AuraSyncPlugin;

impl Plugin for AuraSyncPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AuraState>()
            .init_resource::<SpellCatalog>()
            .init_resource::<ReplicationMirrorMap>()
            .add_message::<CancelAuraRequest>()
            .add_systems(
                Update,
                (
                    tick_aura_timers,
                    sync_unit_auras,
                    clear_removed_unit_auras,
                    send_cancel_aura_requests,
                )
                    .chain(),
            );
    }
}

type AuraUnits<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Ref<'static, UnitAuras>,
        Option<Ref<'static, LocalPlayer>>,
        Has<UnitAuraState>,
    ),
>;

fn sync_unit_auras(
    mut commands: Commands,
    mut aura_state: ResMut<AuraState>,
    catalog: Res<SpellCatalog>,
    mirrors: Res<ReplicationMirrorMap>,
    units: AuraUnits,
    local: Query<Entity, With<LocalPlayer>>,
    names: Query<(Option<&NetPlayer>, Option<&Npc>)>,
    player: PlayerTextState,
) {
    let name_of = |caster: u64| caster_name(caster, &mirrors, &names);
    let casters = AuraCasterLookup {
        local_player: local
            .iter()
            .next()
            .and_then(|entity| mirrors.main_to_server(entity))
            .map(Entity::to_bits),
        name_of: &name_of,
    };
    let text_ctx = player.context(&units);
    for (entity, auras, local_marker, has_unit_state) in &units {
        let local_added = local_marker.as_ref().is_some_and(Ref::is_added);
        if !auras.is_changed() && !local_added && !catalog.is_changed() && !player.is_changed() {
            continue;
        }
        let instances = aura_instances(&auras.auras, catalog.data(), &casters, &text_ctx);
        if local_marker.is_some() {
            aura_state.auras = instances;
            if has_unit_state {
                commands.entity(entity).remove::<UnitAuraState>();
            }
        } else {
            commands
                .entity(entity)
                .insert(UnitAuraState { auras: instances });
        }
    }
}

/// What description conditions test: the player's spells, spec and own auras.
#[derive(bevy::ecs::system::SystemParam)]
struct PlayerTextState<'w> {
    known: Option<Res<'w, KnownSpells>>,
    spec: Option<Res<'w, ActiveSpecialization>>,
}

impl PlayerTextState<'_> {
    fn is_changed(&self) -> bool {
        self.known.as_ref().is_some_and(|res| res.is_changed())
            || self.spec.as_ref().is_some_and(|res| res.is_changed())
    }

    fn context(&self, units: &AuraUnits) -> SpellTextContext {
        let local_auras = units
            .iter()
            .find(|(_, _, local, _)| local.is_some())
            .map(|(_, auras, _, _)| auras.auras.iter().map(|view| view.spell_id).collect());
        SpellTextContext::for_player(
            self.known.as_deref(),
            self.spec.as_deref(),
            local_auras.unwrap_or_default(),
        )
    }
}

fn caster_name(
    caster: u64,
    mirrors: &ReplicationMirrorMap,
    names: &Query<(Option<&NetPlayer>, Option<&Npc>)>,
) -> Option<String> {
    let main = mirrors.server_to_main(Entity::try_from_bits(caster)?)?;
    let (player, npc) = names.get(main).ok()?;
    player
        .map(|player| player.name.clone())
        .or_else(|| npc.map(|npc| npc.name.clone()))
}

fn clear_removed_unit_auras(
    mut commands: Commands,
    mut removed: RemovedComponents<UnitAuras>,
    mut aura_state: ResMut<AuraState>,
    local: Query<(), With<LocalPlayer>>,
) {
    for entity in removed.read() {
        if local.contains(entity) {
            aura_state.auras.clear();
        } else if let Ok(mut unit) = commands.get_entity(entity) {
            unit.try_remove::<UnitAuraState>();
        }
    }
}

fn tick_aura_timers(
    time: Res<Time>,
    mut aura_state: ResMut<AuraState>,
    mut units: Query<&mut UnitAuraState>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    if has_timed_aura(&aura_state.auras) {
        aura_state.tick(dt);
    }
    for mut unit in &mut units {
        if has_timed_aura(&unit.auras) {
            unit.tick(dt);
        }
    }
}

fn has_timed_aura(auras: &[game_engine::buff_data::AuraInstance]) -> bool {
    auras.iter().any(|aura| !aura.is_permanent())
}

fn send_cancel_aura_requests(
    mut requests: MessageReader<CancelAuraRequest>,
    mut senders: MessageSenders<CancelAura>,
) {
    for request in requests.read() {
        for mut sender in senders.iter_mut() {
            sender.send::<CombatChannel>(CancelAura {
                spell_id: request.spell_id,
            });
        }
    }
}

#[cfg(test)]
#[path = "auras_tests.rs"]
mod tests;
