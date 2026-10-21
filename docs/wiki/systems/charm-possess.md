# Charm and possession

The client reads `UnitCharm {charmer,possessed}` on a unit, not a summon owner. It retains the selected character's `local_player_id`; a once-per-frame cached `(unit_id,possessed)` selects the controlled unit. Only `possessed=true` changes the effective mover. [Client contract](../../specs/charm-possess.md); [server lifecycle/source authority](../../../../game-server/docs/wiki/systems/charm-possess.md).

## Native control

World movement/camera getters read the effective mover's node, facing, epoch and authoritative position. The character's own charmed body never runs local prediction; NPC-cast charm follows server motion. Changing movers clears the old body's local movement/report state while retaining the user's walk/run toggle. Selected-player metadata and account/UI identity remain unchanged.

Possessed NPCs use replicated PlayerMotion, just like controlled players; ordinary charmed players use server CreatureMotion. Original NPC animation state is invalidated on control changes so release restores its ordinary motion rather than leaving a stale local run clip. Ship-frame reports use the effective mover, not a parked charmer passenger.

The existing pet slot/bar and context menu refer to a charm before a source-designated pet. Generic Dismiss uses PetAction command3; hunter-pet Dismiss2641 is not used for a charm even when the character is a hunter. No new widgets, skin-specific behavior, or fabricated NPC loadout.

## Sources

- TC a352b1fa `Unit::SetCharmedBy`/`RemoveCharmedBy` and `Player::{Charm,Possess}SpellInitialize`, `CharmInfo::{InitCharm,InitPossess}CreateSpells`, `GetFirstControlled`.
- Local Retail `Blizzard_UnitPopupShared/UnitPopupSharedButtonMixins.lua1067-1073`: PetDismiss versus hunter Dismiss spell.
- Existing native movement, pet bar and unit frame consumers listed in the contract.

## Proof status

Real UDP ignoredRED fails explicitly on missing `shared::components::UnitCharm` codec (0PASS/1FAIL0.03s). At14d61bb20, actualUDP1GREEN0.90s and three source-bound client selection/movement-report regressions GREEN (native helper508.9s); ignored wire case now enabled. Rendered native screenshots still pending. Runtime proof ledger is maintained outside tracked data in `/home/osso/.worktrees/handoff-charm-possess.md`.
