# Mount Journal

User-approved2026-10-11: Retail Mainline Collections→Mounts in both skins; Forever modifies art only. [Implementation and proof](../wiki/systems/mounts.md#mount-journal).

## What it must do

- [x] List authentic Mount.db2 identities, selected details/source text, search, collected/not-collected and source filters; five capability-derived type categories and authored hidden/server-only flags. Tests assert filtering and readable vendor/zone/cost text.
- [x] Learn authentic mount items through normal UseItem, consuming one item atomically with ownership; duplicate learning preserves the item. No fabricated starter collection.
- [x] Persist collection/favourites account-wide across characters/restarts; share changes only with the same account. Existing authentic spell-learning results grant account ownership.
- [x] Summon through normal spell/riding/area rules; dismount cancels the real aura. Random favourites use owned usable candidates, never fabricated display mounting; combat/unowned requests refuse.
- [x] Import every local MountXDisplay appearance root into a new cache preserving all prior profiles/choices/geosets/coverage; reject conflicting rows/replacement. Extract missing assets from local CASC only.
- [ ] Both skins show the same Mainline703×606 shell/tab/list layout and a rotating authentic3D preview; native item learning, right-click favourite, summon/dismount/random, and alternate-character persistence must pass the privateUDP5901 fixture.

### Acquisition/game integration — user extension2026-10-11

- [ ] Authentic vendors sell mount teaching items/direct-learn mounts under existing cost, ExtendedCost, faction and reputation requirements; buying/learning is proved natively in both skins.
- [ ] Mount teaching items list/sell/buy under Retail AH Miscellaneous→Mounts; buying/delivery/UseItem adds ownership, with both-skin live proof.
- [ ] Already-collected mounts show “Already known” in item tooltips, vendor entries and AH presentation, reflecting authoritative account state. Teaching-item identities come from the server's authentic Retail ItemEffect links; same-ID Forever items remain separate.
- [ ] Mount items grant no transmog appearance; Mounts shares the Collections frame with the existing Pets/Toys/Appearances integration. Note authored mount-equipment slot/data support without inventing unsupported behaviour.

## How it works

- [Mount systems and journal](../wiki/systems/mounts.md#mount-journal).
- [Authored appearance importer contract](npc-appearance-importer.md).

## Implementation inventory

- `godot/ui-model/src/mount_journal.rs`, `mount_source.rs` — journal state, shared layout and source presentation.
- `godot/rust/src/mount_journal.rs`, `mount_journal_snapshot.rs` — lifecycle, native input, detached preview and read-only observations.
- `godot/rust/src/account.rs`, `godot/network/src/lib.rs` — authoritative catalog/request transport.
- `scripts/import_mount_appearance.py` — strict read-only-base mount-root importer.
- Server `mount_journal.rs`, `persistence_mounts.rs`; protocol `mount_messages.rs` — catalog, ordinary casting and account persistence.

## Tests asserting this spec

- `godot/ui-model/tests/mount_journal.rs`.
- `godot/tests/mount_journal_live.gd`.
- `scripts/test_import_mount_appearance.py`.
- Server `mount_journal_tests.rs`, `item_use_tests.rs`, `item_templates_tests.rs`, `persistence_mounts.rs`, `collection_tests.rs`.
- Protocol `tests/mount_journal.rs`; existing server `auras/mount_tests.rs` owns flight/riding rules.

## Feedback authority

World entry receives one complete catalog. Learning, favourites and mount-state changes replace changed rows only; the retained client view advances SharedContext state and Screen output without reopening the journal. Character changes reset the catalog boundary.

## Known gaps (current cycle)

- [ ] Reliable live account-change/dismount feedback, both-skin vendor/AH acquisition acceptance and final integration/check/fmt/protocol gate.

## Out of scope

Mount equipment, new flight physics, canonical world.db writes, sharedUDP5000 operations and unrelated full suites.
