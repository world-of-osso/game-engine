# Mount Journal

User-approved2026-10-11: Retail Mainline Collections→Mounts in both skins; Forever modifies art only. [Implementation and proof](../wiki/systems/mounts.md#mount-journal).

## What it must do

- [x] List authentic Mount.db2 identities, selected details/source text, search, collected/not-collected and source filters; five capability-derived type categories and authored hidden/server-only flags. Tests assert filtering and readable vendor/zone/cost text. The fixed90px Retail Filter dropdown opens a content-fit shared menu; its authored ten-pixel spacer and row actions remain intact.
- [x] Learn authentic mount items through normal UseItem, consuming one item atomically with ownership; duplicate learning preserves the item. No fabricated starter collection.
- [x] Persist collection/favourites account-wide across characters/restarts; share changes only with the same account. Existing authentic spell-learning results grant account ownership.
- [x] Summon through normal spell/riding/area rules; dismount cancels the real aura. Random favourites use owned usable candidates, never fabricated display mounting; combat/unowned requests refuse.
- [x] Import every local MountXDisplay appearance root into a new cache preserving all prior profiles/choices/geosets/coverage; reject conflicting rows/replacement. Extract missing assets from local CASC only.
- [x] Both skins show the same Mainline703×606 shell/tab/list layout and a rotating authentic3D preview; native item learning, right-click favourite, summon/dismount/random, and alternate-character persistence must pass the privateUDP5901 fixture.
- [x] Right-click Mount/Dismount and Favorite entries use the shared Retail menu compositor at the cursor (MenuUtil.CreateContextMenu), fit rendered labels and preserve authoritative enabled states. RightInset contains the full-size ModelScene and ShadowOverlayTemplate; the authored MountDisplay YesMountsTex remains its backdrop (Blizzard_MountCollection.xml:443-466), never a substitute for the model.

### Acquisition/game integration — user extension2026-10-11

- [x] Authentic vendors sell mount teaching items under authored gold/ExtendedCost, faction and minimum reputation requirements; buying/learning is proved natively in both skins. TDB1210 has no direct-learn mount offers; no route is fabricated. [Cost provenance and unavailable data](../../../game-server/docs/mount-vendor-costs.md).
- [x] Mount teaching items list/sell/buy under Retail AH Miscellaneous→Mounts; buying/delivery/UseItem adds ownership, with both-skin live proof.
- [x] Already-collected mounts show “Already known” in item tooltips, vendor entries and AH presentation, reflecting authoritative account state. Teaching-item identities come from the server's authentic Retail ItemEffect links; same-ID Forever items remain separate.
- [x] Mount items grant no transmog appearance; Mounts reuses the Collections shell and existing Pets/Toys navigation. Existing Appearances collection/service authority remains unchanged. Mount equipment remains out of scope (Retail AH class15/subclass6 exists; no equipment functionality is invented).

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
- `godot/tests/mount_journal_live.gd`, `mount_auction_live.gd`; both-skin paid vendor, native favourite/mount/random, alternate-character and two-account AH/mail/learn proofs.
- `scripts/test_import_mount_appearance.py`.
- Server `mount_journal_tests.rs`, `item_use_tests.rs`, `item_templates_tests.rs`, `persistence_mounts.rs`, `collection_tests.rs`.
- Protocol `tests/mount_journal.rs`; existing server `auras/mount_tests.rs` owns flight/riding rules.

## Feedback authority

World entry receives one complete catalog. Learning, favourites and mount-state changes replace changed rows only; the retained client view advances SharedContext state and Screen output without reopening the journal. Character changes reset the catalog boundary.

## Known gaps (current cycle)

- [x] Reliable live account-change/dismount feedback and both-skin vendor/AH acquisition acceptance.
- [x] Final integration/check/fmt/protocol gate (2026-10-12, after merging master53c6e7a8f/server99f61aec/protocola429549).
- Detached previews need a verified Retail model receipt. Canonical receipts initially cover509 of1,971 MountXDisplay mounts (120 of686 model FDIDs). The2026-10-12 local-CASC candidate stages all1,971 mount chains,566 new M2 receipts and their dependencies under the existing [product-isolated publication](product-isolated-model-assets.md) contract; canonical publication remains lead-owned. [Staging and native census boundaries](../wiki/systems/mounts.md#mount-asset-closure-staging--2026-10-12) distinguish authored asset completeness from absent Mount metadata and renderer acceptance.
- Unavailable authored cost metadata omits78 ExtendedCost offers rather than making them free; other121 have complete cost data. Vendor PlayerCondition evaluation and reputation price discounts remain existing server boundaries, not new invented behavior.

## Out of scope

Mount equipment, new flight physics, canonical world.db writes, sharedUDP5000 operations and unrelated full suites.
