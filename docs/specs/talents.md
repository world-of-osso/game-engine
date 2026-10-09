# Talents

PlayerSpellsFrame displays and edits the existing server talent configuration. Modern and Forever expose identical elements and behavior; Forever changes art only. [Local graph/layout provenance](../wiki/systems/talents.md).

## Required behavior

- Map class SkillLine (category 7) through SkillLineXTraitTree variant 0; filter local DB2 nodes/entries by spec visibility, retaining ordered choices, prerequisites and hero trees.
- Receive `TraitConfigSnapshot` on the owning account. Display committed total ranks (including grants) and server unspent class/spec points. Each new snapshot replaces pending edits, closes the choice flyout and clears errors.
- Left-click stages one rank. Right-click refunds one purchased rank, never a grant. Tiered nodes follow DB2 entry order in both directions, not the sorted wire IDs. Choice nodes open a flyout; choosing stages exactly one entry. Hero selection uses the same choice flow.
- Reject unavailable edits locally: entry maxima, required/sufficient prerequisite edges, conditions, earlier-row currency gates, level budgets, selected hero subtree and available points. The server remains authoritative.
- Apply Changes sends one `CommitTraitConfig` containing the specialization and complete pending entry list with total ranks. No node click sends a request or casts a spell.
- Undo/Reset discard pending edits and restore committed state. Undo replaces Reset at their shared Retail anchor while changes exist.
- A failed `TraitCommitResult` displays its supplied reason on the page and through the existing world error text path. Only a snapshot changes committed ranks; success alone does not assume acceptance state.
- Preserve the local icons, shapes, /10 coordinates and pan offsets; missing art stays missing. Shared spell hover remains available.

## Retail references (local source)

All paths relative to `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:

- `Blizzard_SharedTalentUI/Blizzard_TalentButtonSpend.lua:5-54`: left purchase, right refund; `CanPurchaseRank` requires availability and affordability.
- `Blizzard_SharedTalentUI/Blizzard_TalentButtonSelect.lua:47-77,98-100`: choice selection and right-click refund.
- `Blizzard_SharedTalentUI/Blizzard_TalentButtonTierTrackTemplates.lua:116-142`: tier allocation traverses ordered entry IDs and requires previous tiers complete.
- `Blizzard_SharedTalentUI/Blizzard_SharedTalentFrame.lua:1639-1674,1773-1782`: commit/rollback and staged operations.
- `Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentsFrame.lua:1133-1192`: ApplyConfig, CommitConfigInternal, rollback currency/button refresh.
- `Blizzard_PlayerSpells/ClassTalents/Blizzard_ClassTalentsFrame.xml:204-223,247-322`: currency displays, Apply button and shared Undo/Reset anchor.

## Implementation

`godot/core/src/talent_data.rs` loads the visual graph. `talent_data/rule_loader.rs` reads raw CSV rule metadata; `rule_data.rs` and `rules.rs` port the existing game-server trait data/evaluator without changing server validation. Keep this port aligned with `game-server/crates/server/src/trait_config.rs`; do not invent alternate client talent rules.

`godot/ui-model/src/talent_editor.rs` owns committed/pending allocations, point deltas and requests. The account owns one editor per connection; actual specialization changes clear it until the new snapshot. A current-spec echo from a rejected request preserves that configuration. A grouped TalentChannel relay preserves spec/result/snapshot order across typed receiver buffers. `talents.rs` projects it into the shared page. NetworkBridge receives both server talent messages and the account sends commits on `TalentChannel`.

## Proof

- `godot/ui-model/tests/talentwire.rs`: snapshots, pending purchase/refund/undo, impossible edits, full commits, failure reasons, replacement snapshots, choice switching, gates and hero availability.
- `godot/ui-model/tests/talents.rs`: shared elements, pending rank labels, class/spec point counters, Apply/Undo and choice flyout in both skins.
- `godot/network/src/wire_tests.rs`: real UDP commit payload, snapshot/result delivery, and production-ordered login/commit/spec-switch traffic accumulated in one worker batch.
- `godot/rust/src/talents_account_tests.rs`: loading-time snapshots, existing error text events, specialization replacement and same-spec echoes.
- `GODOT_TALENT_PENDING=1` stages one valid class and one valid spec purchase in the offline Talents preview using production model operations. Native evidence lives in `data/diagnostics/talentwire-2026-10-09/`.

## Offline class/spec evidence

- `GODOT_PREVIEW_CLASS` and `GODOT_PREVIEW_SPEC` select explicit DB2 class/spec IDs together. Unknown, Initial, mismatched or missing data must fail explicitly, never substitute Mage.
- With neither input set, retain the existing Frost Mage spellbook and Arcane Mage Talents snapshots unchanged.
- Explicit spellbook previews project catalog class-line and selected-spec membership as a capture snapshot, not a player's learned-spell state or proof of spell execution. Talents use the production local graph/rules and granted-only initial configuration; no fabricated nodes or icon art.
- `godot/tests/capture_classbooks.gd` batches the offline Forever pages at real 1920×1080 from a diagnostic manifest, records per-page errors and geometry, and never connects to a server.
- Coverage/evidence: `data/diagnostics/classbook-shots-2026-10-09/coverage.md` and `coverage.json`. Counts describe local data presence only; missing icon art remains blank.

## Exclusions and known gaps

Loadout save/import/export and server/protocol changes are out of scope. Existing stacked hero previews remain; only the chosen subtree is purchasable. Missing local icon assets remain empty. Offline captures do not prove live persistence or reconnect behavior; server persistence already belongs to the existing talent system.
