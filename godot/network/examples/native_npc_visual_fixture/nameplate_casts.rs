//! `nameplate-casts`: the fixture server drives the enemy NPC's cast bar the way the game
//! server does — replicated `CastState` for casts and channels, `SpellFailure` for an
//! interrupt by the player and for a cast failing on completion, `SpellGo` for a cast
//! resolving — each step on a marker the observing GDScript prints.
use super::*;
use shared::casting::CastState;
use shared::protocol::{CombatChannel, SpellFailure, SpellGo};
use shared::spell_data::CastFailReason;

pub(super) const FIREBALL: u32 = 133;
pub(super) const FROSTBOLT: u32 = 116;
pub(super) const ARCANE_MISSILES: u32 = 5143;

fn cast(spell_id: u32, name: &str, target: Entity, interruptible: bool) -> CastState {
    // Cold catalog/icon loading can consume the old three-second cast before observation.
    // The probe owns completion through CAST_KICK/RESOLVE/FAIL markers.
    let mut cast = CastState::normal(spell_id, target.to_bits(), 120.0, interruptible);
    cast.spell_name = name.into();
    cast
}

/// Applies the step named by `line`; `true` when the line was a cast marker.
pub(super) fn step(app: &mut App, line: &str, player: Entity, npc: Entity) -> Result<bool, String> {
    let link = app
        .world_mut()
        .resource::<Incoming>()
        .selected_link
        .ok_or("cast step before character selection")?;
    let failure = |spell_id, reason, failed_by: Option<Entity>| SpellFailure {
        caster: npc.to_bits(),
        spell_id,
        reason,
        failed_by: failed_by.map(Entity::to_bits),
    };
    match line {
        "FIXTURE CAST_START" => {
            app.world_mut()
                .entity_mut(npc)
                .insert(cast(FIREBALL, "Fireball", player, true));
        }
        "FIXTURE CAST_KICK" => {
            app.world_mut().entity_mut(npc).remove::<CastState>();
            let kick = failure(FIREBALL, CastFailReason::InterruptedCombat, Some(player));
            send::<_, CombatChannel>(app, link, kick)?;
        }
        "FIXTURE CAST_SHIELDED" => {
            app.world_mut()
                .entity_mut(npc)
                .insert(cast(FROSTBOLT, "Frostbolt", player, false));
        }
        "FIXTURE CAST_RESOLVE" => {
            app.world_mut().entity_mut(npc).remove::<CastState>();
            let go = SpellGo {
                caster: npc.to_bits(),
                target: Some(player.to_bits()),
                hit_targets: vec![player.to_bits()],
                spell_id: FROSTBOLT,
            };
            send::<_, CombatChannel>(app, link, go)?;
        }
        "FIXTURE CAST_CHANNEL" => {
            let mut channel = CastState::channel(ARCANE_MISSILES, player.to_bits(), 3.0, 1.0, true);
            channel.spell_name = "Arcane Missiles".into();
            app.world_mut().entity_mut(npc).insert(channel);
        }
        "FIXTURE CAST_CHANNEL_END" => {
            app.world_mut().entity_mut(npc).remove::<CastState>();
        }
        "FIXTURE CAST_FAIL_START" => {
            app.world_mut()
                .entity_mut(npc)
                .insert(cast(FIREBALL, "Fireball", player, true));
        }
        "FIXTURE CAST_FAIL" => {
            app.world_mut().entity_mut(npc).remove::<CastState>();
            let failed = failure(FIREBALL, CastFailReason::OutOfRange, None);
            send::<_, CombatChannel>(app, link, failed)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
