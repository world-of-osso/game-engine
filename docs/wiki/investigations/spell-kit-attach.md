# Spell kit attachment ordering

verified: 2026-10-10

## Cause

The [no-install certification](../../../../data/diagnostics/noinstall-2026-10-10/summary.md) has41 non-asset lines:40 spell diagnostics containing165 attachment failures across15 spells/60 owners, plus one absent local-player visual. Every requested attachment exists in its real local M2/SKA1 lookup. Every original owner eventually reports failed visual loading. These are downstream asset-chain failures, not absent anchors or wrong AttachmentID mapping.

The receipt-recovery Northshire capture proves an independent ordering defect: spell10848/kit989 fails attachment19 at `northshire-mount/native.log:170`, before display5233's visual succeeds at line368. `spawn_due` consumed the ready effect before the asynchronous owner model was attached. Held aura effects never retried after owner completion.

## Correction

Pending kit models wait for owner visual completion as well as effect availability and StartDelay. They retain their original due time; held effects join their current phase and expired one-shots do not restart. An explicit DB2 -1 attachment still uses the unit root without requiring a model. Settled missing owners report owner unavailability rather than claiming a missing anchor. Actual missing attachments still fail explicitly.

No attachment remapping/fallback was added. Unsupported effect types do not explain these type2 failures; no genuine missing-anchor gap was found in the165 baseline occurrences. Receipt/qualified skin gaps remain separate asset work.

DB2 AttachmentID names an M2 attachment (TrinityCore `SpellVisualKitModelAttach.dbc` structure). Native nodes use the model's attachment lookup, not the attachment-array row number. Real legacy model124640 owns attachment19 at lookup19/bone4; HD owners use their SKA1 lookup. Runtime readiness is a client scheduling correction, not a claim about undocumented Retail streaming timing.

## Proof

- RED: one real-DB2 scheduling regression fails because an available effect is considered due while its owner is loading. `ordering-red.log`, exit101.
- GREEN/native acceptance: pending.
- Classification: `data/diagnostics/spell-kit-attach-2026-10-10/baseline-{owners,classification}.json` in canonical game-engine.

## Sources

- [Model spawn scheduling](../../../godot/rust/src/spell_effects/models.rs)
- [Owner visual state](../../../godot/rust/src/world.rs)
- [Real-data regression](../../../godot/rust/src/spell_effects_tests.rs)
- [TrinityCore AttachmentID field](https://trinitycore.info/files/DBC/335/spellvisualkitmodelattach)

## See Also

- [Spell visuals](../systems/spell-visuals.md)
- [M2 format](../formats/m2-format.md)
- [Behavior contract](../../specs/spell-visuals.md)
