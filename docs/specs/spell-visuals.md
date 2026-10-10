# Spell visuals

The Godot client selects Retail DB2 spell visuals and plays their event kits on units. Selection lives in `godot/core/src/spell_visual.rs`; native cast routing lives in `godot/rust/src/spell_effects/`. See [resolution and source evidence](../wiki/systems/spell-visuals.md#local-specialization-selection-2026-10-07).

## What it must do

- [x] Local casts use the latest spellbook specialization from `SpecializationChanged`, mapped from `ChrSpecialization.ID` to zero-based `OrderIndex`, when evaluating `PlayerCondition.ChrSpecializationIndex`. Cast, precast/channel, aura, missile and prefetch selection use the same caster context.
- [x] With the same two-handed sword, Slam 1464 chooses Arms visual 51946 / cast kit 62428 and Fury visual 97446 / cast kit 128672.
- [x] Before specialization is received, skip specialization comparison rather than assuming the first spec. For Slam with a two-handed sword this chooses visual 51946 / kit 62428. This follows the reference evaluator; direct Retail startup visual timing remains unverified.
- [x] Other casters keep the existing no-spec selection behavior; never apply the local player's specialization to remote players or NPCs.

## Toy conditional visuals (2026-10-10)

- [x] Evaluate a single caster AuraSpellID/stack requirement from replicated UnitAuras, including its AuraSpellLogic65536 inversion. Spitzy261981 selects visual74073 without aura181943 and refuses that visual when181943 is present.
- [x] Unknown multi-aura logic, ModifierTree and other unsupported requirements remain unavailable, never bypassed. Scoots1280563/ModifierTree303980 remains blocked by absent local data.
- [ ] Procedural cloak kit106563/Type17 payload is not present in the local export; no model mapping is invented.

Behavioral fixture: `godot/core/tests/spell_visual.rs::toyfx6_spitzy_inverted_aura_condition_shows_without_disabling_aura`. Native both-skin proof remains pending.

## How it works

- [Spell visual resolution, DB2 rows and references](../wiki/systems/spell-visuals.md#local-specialization-selection-2026-10-07)

## Implementation inventory

- `godot/core/src/spell_visual.rs` — DB2 specialization metadata, condition evaluation and visual/kit selection.
- `godot/rust/src/combat_visuals.rs` — account specialization supplied before combat events and frame synchronization.
- `godot/rust/src/spell_effects.rs` — local specialization identity and prefetch invalidation.
- `godot/rust/src/spell_effects/casts.rs` — local-only caster context used by visual selection and prefetch.
- `godot/rust/src/player_spells.rs` / `account.rs` — existing authoritative specialization reception/state; no protocol change.

## Tests asserting this spec

- `godot/core/tests/spell_visual.rs::specialization_slam_picks_arms_and_fury_cast_kits_with_the_same_two_handed_sword`
- `godot/rust/src/spell_effects/casts.rs::specialization_tests` — fresh spell state, Arms→Fury transition, local-only filtering and missing specialization.

At `82ad03641`, the native/core specialization filter passed (three feature tests plus one existing icon test), then all nine `game-engine-core --test spell_visual` tests passed. Wrong Fury kit was reproduced before the fix. Changed-file rustfmt and whitespace checks passed; no compiler warnings in either GREEN log. Logs: `/tmp/claude/specvisuals.full.log` and `/tmp/claude/specvisuals-regression.full.log`. Tests assert state-to-DB2 selection, not rendered casts; event/prefetch wiring is source-audited and compiled.

## Known gaps (current cycle)

- [ ] Remote-player/NPC specialization is not replicated. Specialization-conditioned selection remains approximate for those casters.
- [ ] Live cast/rendered-image parity for both specs and Retail's fresh-login timing are unverified. Existing `godot/tests/spellcast_anim.gd` requires a live test server; no offline specialization capture path was found.

## Out of scope

Shared-protocol/game-server changes, remote specialization replication, unrelated condition types, renderer changes and cache/builder policy changes.
