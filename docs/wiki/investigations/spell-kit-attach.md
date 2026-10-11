# Spell kit attachment ordering

verified: 2026-10-10

## Cause

The [no-install certification](../../../../data/diagnostics/noinstall-2026-10-10/summary.md) has41 non-asset lines:40 spell diagnostics containing165 attachment failures across15 spells/60 owners, plus one absent local-player visual. Every requested attachment exists in its real local M2/SKA1 lookup. Every original owner eventually reports failed visual loading. At the actual error boundary,68 occurrences still had pending owner requests;97 had already failed. These are owner readiness/asset-chain failures, not absent anchors or wrong AttachmentID mapping.

The receipt-recovery Northshire capture proves an independent ordering defect: spell10848/kit989 fails attachment19 at `northshire-mount/native.log:170`, before display5233's visual succeeds at line368. `spawn_due` consumed the ready effect before the asynchronous owner model was attached. Held aura effects never retried after owner completion.

## Correction

Pending kit models wait for owner visual completion as well as effect availability and StartDelay. They retain their original due time; held effects join their current phase and expired one-shots do not restart. An explicit DB2 -1 attachment still uses the unit root without requiring a model. Settled missing owners report owner unavailability rather than claiming a missing anchor. Actual missing attachments still fail explicitly.

No attachment remapping/fallback was added. Unsupported effect types do not explain these type2 failures; no genuine missing-anchor gap was found in the165 baseline occurrences. Receipt/qualified skin gaps remain separate asset work.

The local Retail `SpellVisualKitModelAttach.AttachmentID` values resolve through their target M2/SKA1 attachment lookup. Native nodes use the model's attachment lookup, not the attachment-array row number. Real legacy model124640 owns attachment19 at lookup19/bone4; HD owners use their SKA1 lookup. Runtime readiness is a client scheduling correction, not a claim about undocumented Retail streaming timing.

## Proof

- RED: one real-DB2 scheduling regression fails because an available effect is considered due while its owner is loading. `ordering-red.log`, exit101.
- GREEN at `a4a3dcaff`:7/7 focused native Rust tests; real local DB2 spell10848 and M2 owner124640. Native extension/CLI build exit0. Changed-file rustfmt check passes.
- Private read-only canonical prebuilt server/admin, UDP54218, own fb_spell_kit_attach/KitAttach, extracted-only client.14 affected spells produce fresh own SpellGo plus active own kit models in each configured skin;28 PNGs individually inspected. Same baseline kit plus exact own authored anchor paths are proved for10 spells in both skins:10848,19750,55474,55701,80676,81245,81502,147164,266099,397765.
- `81500`/`84841` select different kits for this player than the baseline NPCs. `18950`/`41634` have active own models but the Forever observer missed their node-name paths. `11977` produced no active-kit observation; not labeled a data gap. Persistent earlier auras overlap captures; this is attachment acceptance, not isolated per-spell pixel parity.
- Scoped final native log:0 spell errors,0 missing-anchor diagnostics,0 unavailable-owner diagnostics;16 NPC receipt errors all name142989.m2. Original41 non-asset lines and this Northshire cast sample are different scenario coverage; no whole no-install certification claimed.
- Original upstream receipt/skin failures remain asset work. No genuine missing-anchor/fallback-table gap was found; no fabricated mapping/origin fallback added.
- Classification and receipts: canonical `data/diagnostics/spell-kit-attach-2026-10-10/`, including baseline-ordering.json, acceptance-matrix.json, live-casts.json, screenshot-inspection.json, executable hashes and native logs. User-visible originals: `/syncthing/AgentShared/2026-10-10/spell-kit-attach/`.
- Invalid setup/partial captures retained separately: server data link/offline learning/stale socket, read-only and overlong Wayland runtime paths, read-only PNG destination, GPU-memory exhaustion. Final single-client run waited for GPU capacity; all owned processes stopped.

## Sources

- [Model spawn scheduling](../../../godot/rust/src/spell_effects/models.rs)
- [Owner visual state](../../../godot/rust/src/world.rs)
- [Real-data regression](../../../godot/rust/src/spell_effects_tests.rs)
- Local Retail69933 `data/db2/12.1.0.69933/SpellVisualKitModelAttach.csv` and `SpellVisualKitEffect.csv`; real M2/SKA1 owner files recorded in baseline-classification.json.

## See Also

- [Spell visuals](../systems/spell-visuals.md)
- [M2 format](../formats/m2-format.md)
- [Behavior contract](../../specs/spell-visuals.md)
