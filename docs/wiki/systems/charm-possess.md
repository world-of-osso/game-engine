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

Real UDP ignoredRED fails explicitly on missing `shared::components::UnitCharm` codec (0PASS/1FAIL0.03s). At14d61bb20, actualUDP1GREEN0.90s and three source-bound client selection/movement-report regressions GREEN (native helper508.9s); ignored wire case now enabled.

At267edf0ca, both native skins pass actual605 on a published Blackrock Spy, authoritative NPC movement with parked priest and following camera, StopSpellCast cancellation/control restoration, and205364 with the priest still movable. Eight inspected original1920×1080 PNGs live at `/syncthing/AgentShared/charm-possess-2026-10-21/`; receipts/log paths are in the [contract](../../specs/charm-possess.md). No skin-specific behavior was added.

The fixture waits for the roster CanvasLayer before native Down/Enter input. Admin learn-spell requires the character online, so an explicit ready marker/file brackets setup. IPC `spell stop` tests the existing channel-cancel route; Escape's target-clear priority and context-menu clicks are not part of this native proof. Failed setup runs remain in diagnostics. Runtime proof ledger: `/home/osso/.worktrees/handoff-charm-possess.md`.
