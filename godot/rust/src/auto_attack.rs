//! The local player's auto-attack requests. Selecting a unit never attacks it (TrinityCore
//! `HandleSetSelectionOpcode` only sets the selection, MiscHandler.cpp:396-399); the
//! client asks with `AttackSwing` (`CMSG_ATTACK_SWING` → `Unit::Attack`) when the player
//! right-clicks an attackable unit, uses Auto Attack (6603, whose `SPELL_EFFECT_ATTACK`
//! the server leaves to the client, SpellEffects.cpp:170), or casts a spell that
//! initiates combat (`SpellAutoAttack`). While attacking, a new attackable selection
//! becomes the victim (the switch `Unit::Attack` handles, Unit.cpp:5927-5931) and any
//! other selection sends `AttackStop`. The server's `AttackStart` / `AttackStopped`
//! echoes keep the victim in step with server-side stops (death, evade, refusals).

use std::collections::HashMap;

use game_engine_core::spell_catalog::SpellAutoAttack;
use game_engine_network::replica::Replica;
use game_engine_session::SessionScreen;
use shared::components::{Health, Player, UnitFlags};
use shared::faction_reaction::{FactionTemplateEntry, Unit, can_attack};
use shared::protocol::{AttackStart, AttackStopped, SpellGo};

use crate::GameClient;
use crate::frame_error::SessionError;
use crate::replicated::UnitFields;

/// `Auto Attack`.
const AUTO_ATTACK_SPELL: u32 = 6603;

#[derive(Default)]
pub(crate) struct AutoAttack {
    /// The unit the local player auto-attacks, as last requested or echoed.
    victim: Option<u64>,
}

/// `UnitCanAttack(player, unit)`: a living, selectable, attackable unit other than the
/// player whose faction the player may attack (TrinityCore
/// `WorldObject::IsValidAttackTarget`: no dead, `UNIT_FLAG_NON_ATTACKABLE_2` or
/// friendly-reaction target), through the same FactionTemplate rows as the server.
pub(crate) fn player_can_attack(
    replica: &Replica,
    player: u64,
    id: u64,
    templates: &HashMap<u32, FactionTemplateEntry>,
) -> bool {
    let (Some(me), Some(unit)) = (replica.unit(player), replica.unit(id)) else {
        return false;
    };
    let alive = unit
        .get::<Health>()
        .is_none_or(|health| health.current > 0.0);
    let flags = UnitFlags(unit.unit_flags().unwrap_or_default());
    if id == player || !alive || !flags.is_selectable() || !flags.is_attackable() {
        return false;
    }
    let template = |id: Option<u32>| id.and_then(|id| templates.get(&id));
    can_attack(
        Unit {
            template: template(me.faction_template()),
            is_player: true,
        },
        Unit {
            template: template(unit.faction_template()),
            is_player: unit.has::<Player>(),
        },
    )
}

impl GameClient {
    /// `UnitCanAttack("player", unit)` as the server decides it.
    pub(super) fn can_auto_attack(&mut self, id: u64) -> bool {
        let Some(player) = self.world.local_player_id() else {
            return false;
        };
        let Ok(templates) = self.nameplates.templates(&self.data_root) else {
            return false;
        };
        player_can_attack(&self.replica, player, id, templates)
    }

    /// `AttackTarget`: request auto-attack on an attackable `target`, once per victim.
    pub(super) fn start_auto_attack(&mut self, target: u64) -> Result<(), SessionError> {
        if self.auto_attack.victim == Some(target) || !self.can_auto_attack(target) {
            return Ok(());
        }
        self.account.send_attack_swing(target)?;
        self.auto_attack.victim = Some(target);
        Ok(())
    }

    /// `StopAttack`.
    pub(super) fn stop_auto_attack(&mut self) -> Result<(), SessionError> {
        if self.auto_attack.victim.take().is_some() {
            self.account.send_attack_stop()?;
        }
        Ok(())
    }

    fn spell_auto_attack(&self, spell_id: u32) -> SpellAutoAttack {
        self.spells
            .catalog()
            .and_then(|catalog| catalog.get(spell_id))
            .map_or(SpellAutoAttack::None, |spell| spell.auto_attack)
    }

    /// A cast request: Auto Attack attacks the target; an `OnCast` spell attacks it too.
    /// Returns whether the request was Auto Attack, which is never sent as a cast.
    pub(super) fn auto_attack_on_cast(
        &mut self,
        spell_id: u32,
        target: Option<u64>,
    ) -> Result<bool, SessionError> {
        let auto_attack = spell_id == AUTO_ATTACK_SPELL;
        if let Some(target) = target
            && (auto_attack || self.spell_auto_attack(spell_id) == SpellAutoAttack::OnCast)
        {
            self.start_auto_attack(target)?;
        }
        Ok(auto_attack)
    }

    /// The local player's resolved `PostCast` spell attacks its explicit target.
    pub(super) fn auto_attack_post_cast(&mut self, go: &SpellGo) -> Result<(), SessionError> {
        if self.world.local_player_id() != Some(go.caster)
            || self.spell_auto_attack(go.spell_id) != SpellAutoAttack::PostCast
        {
            return Ok(());
        }
        match go.target {
            Some(target) => self.start_auto_attack(target),
            None => Ok(()),
        }
    }

    /// Per frame, after selection input: the auto-attack follows a new selection.
    pub(super) fn follow_selection_with_auto_attack(&mut self) -> Result<(), SessionError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.auto_attack.victim = None;
            return Ok(());
        }
        let Some(victim) = self.auto_attack.victim else {
            return Ok(());
        };
        match self.targeting_target() {
            Some(target) if target == victim => Ok(()),
            Some(target) if self.can_auto_attack(target) => self.start_auto_attack(target),
            _ => self.stop_auto_attack(),
        }
    }

    pub(super) fn receive_attack_start(&mut self, start: &AttackStart) {
        if self.world.local_player_id() == Some(start.attacker) {
            self.auto_attack.victim = Some(start.victim);
        }
    }

    /// A stop of an earlier victim, echoed after the player switched, keeps the new one.
    pub(super) fn receive_attack_stopped(&mut self, stopped: &AttackStopped) {
        let current = stopped.victim.is_none() || stopped.victim == self.auto_attack.victim;
        if self.world.local_player_id() == Some(stopped.attacker) && current {
            self.auto_attack.victim = None;
        }
    }

    /// The local player's auto-attack victim, for automation.
    pub(super) fn auto_attack_victim(&self) -> Option<u64> {
        self.auto_attack.victim
    }
}
