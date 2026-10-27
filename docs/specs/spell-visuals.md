# Spell visuals

The Godot client selects Retail DB2 spell visuals and plays their event kits on units. Selection lives in `godot/core/src/spell_visual.rs`; native cast routing lives in `godot/rust/src/spell_effects/`. See [resolution and source evidence](../wiki/systems/spell-visuals.md#local-specialization-selection-2026-10-07).

## What it must do

- [x] Local casts use the latest spellbook specialization from `SpecializationChanged`, mapped from `ChrSpecialization.ID` to zero-based `OrderIndex`, when evaluating `PlayerCondition.ChrSpecializationIndex`. Cast, precast/channel, aura, missile and prefetch selection use the same caster context.
- [x] With the same two-handed sword, Slam 1464 chooses active12.1.5.70391 Arms visual190513 / cast kit278884 and Fury visual97446 / cast kit128672; animations812/818 remain distinct.
- [x] Before specialization is received, skip specialization comparison rather than assuming the first spec. For Slam with a two-handed sword, active12.1.5.70391 chooses visual190513 / kit278884. This follows the reference evaluator; direct Retail startup visual timing remains unverified.
- [x] Other casters keep the existing no-spec selection behavior; never apply the local player's specialization to remote players or NPCs.

## Toy conditional visuals (2026-10-10)

- [x] Evaluate a single caster AuraSpellID/stack requirement from replicated UnitAuras, including its AuraSpellLogic65536 inversion. Spitzy261981 selects visual74073 without aura181943 and refuses that visual when181943 is present.
- [x] Unknown multi-aura logic, ModifierTree and other unsupported requirements remain unavailable, never bypassed. Scoots1280563/ModifierTree303980 remains blocked by absent local data.
- [ ] Procedural cloak kit106563/Type17 payload is not present in the local export; no model mapping is invented.

Behavioral fixture: `godot/core/tests/spell_visual.rs::toyfx6_spitzy_inverted_aura_condition_shows_without_disabling_aura`. Actual journal Spitzy kits/models were captured in both configured skins; precise rear-view attachment alignment and live present-aura exclusion are not certified. Mortar live acceptance remains pending; see the [toy evidence boundary](../wiki/systems/toy-box.md#toy-effects6-conditional-selection-and-summon-coverage-2026-10-10).

## Aura procedural alpha (2026-10-27)

- [ ] Type-1 procedural subtype 14 reads `SpellProceduralEffect.Value_0`, clamps it to 0–1, and applies it to the kit's target while an AuraStart→AuraEnd kit is active. Invisibility 66 / kit 42982 uses 0.5.
- [ ] Overlapping aura alpha modifiers use the minimum alpha, a project policy; removing the last modifier restores the mesh's prior transparency. New or replaced unit visuals receive active modifiers.
- [ ] Cast→OneShot subtype-14 kits remain a named gap because their restore timing is unknown. Other procedural subtypes are unchanged.

## Owner attachment ordering (2026-10-10)

- An available type2 kit model waits while its target unit's visual request is pending. Readiness requires the authored start delay and owner completion; DB2's explicit -1/unit-origin placement does not require a model.
- Owner completion never restarts a kit: held effects join their elapsed phase; expired one-shots remain expired. Cast/aura end still removes pending held effects.
- Distinguish an unavailable owner visual after loading from a loaded model missing its authored attachment. Never invent attachment points or reinterpret model load failure as an attachment-ID mapping gap.

Regression: `spell_effects::tests::spell_kit_attach_waits_for_owner_without_restarting_its_timeline`, using local spell10848/kit989 and display5233/model124640, attachment19/bone4. [Investigation and evidence](../wiki/investigations/spell-kit-attach.md).

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

## Active70391 sound regression

- [x] Battle Shout6673 resolves authored cast visual190617/kit279043/SoundKit368264 while keeping unit sound38 and both Human caster-voice assertions. Source chain and evidence: [active Battle Shout sound](../wiki/systems/spell-visuals.md#active-battle-shout-sound-chain--2026-10-15). This is a fixture refresh, not a sound-runtime change.

Active70391 refresh at4fe654f81: genuine core/native Slam REDs and Battle Shout sound RED → full core spell_visual17/17, native specialization5/5 (three Slam plus two existing icon controls). Core/native all-target check, workspace fmt and manual Rust readability pass; logs warning-free. No runtime changes or rendered/audio acceptance claim. Proof ledger: class-completeness6-2026-10-12/slam-proof-ledger.json; source receipts slam-active-source.json and battle-shout-active-source.json.
