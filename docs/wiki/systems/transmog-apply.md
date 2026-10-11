# Native transmog application

Retail Mainline's three-region TransmogFrame stages account-collected appearances, previews a detached character, and applies server-authoritative prices. Modern and Forever share the same elements and behavior; Forever changes art only. [Contract](../../specs/transmog-apply.md).

## State and rendering

`TransmogBook` keeps pending choices separate from `TransmogState.applied`. It sends equipped GUIDs with the batch, disables duplicate application, preserves refusal text, and drops stale pending choices when equipment changes. Exact costs come from the service; unavailable quotes disable paid application, not free restoration.

The tracked workspace retains the canonical shared-protocol sibling patch. Leased tests/builds select the matching protocol checkout with `DEPOT_SIBLING_SHARED_PROTOCOL` (engine helper) or an untracked `.cargo/config.toml` override (server); task-specific protocol paths never enter tracked manifests.

The native host receives service/failure messages through the account/network bridge. Pending preview uses `request_detached_visual`, never the replicated world appearance. Accepted equipment uses the existing self/observer appearance rebuild. Escape and interaction/session closure cancel the detached request and free the frame/scene.

## Native proof — verified 2026-10-11

Private UDP55087, isolated `fb_transmog_a`/`fb_transmog_b`, Hashom's authored gossip, physical Bronze Shortsword2850 GUID123:

- Modern: source821 Gladius pending → native Apply → Applied; copper100000→98500. Fresh-process relog preserved Applied821 and98500. A second native1181 Deadman Blade application reduced gold to97000; observer received and attached the rebuilt existing player visual.
- Forever: same redb after private-server restart preserved Applied1181/97000. Native821 application reduced gold to95500; self and observer visual rebuilds attached. Fresh-process relog preserved Applied821, zero pending changes and95500.
- Evidence: canonical `data/diagnostics/transmog-apply-2026-10-11/`; final `modern-applied-ui.log`, `modern-relog-ui.log`, `forever-relog-final-ui.log`, both observer logs and inspected PNGs. User PNGs: `/syncthing/AgentShared/2026-10-11/transmog-apply/`. Observer screenshot framing is weak; existing-visual rebuild/attachment logs provide the stronger update evidence.

Independent verifier could not authenticate (Claude OAuth expired); no independent PASS. Native build/check and targeted behavioral evidence are recorded in the handoff.

## Fixture readiness

Bag question-mark icons appear before item metadata finishes loading. The old right-click branch classified absent metadata as a non-equippable item and sent Use instead of Equip. `container_item_request` now requires resolved metadata and propagates its explicit error. The regression covers loading refusal, sword Equip and Hearthstone Use. Waiting for metadata then using native bag right-click equipped2850 correctly; waiting for an icon alone is insufficient.

InteractTarget is unbound and keyboard soft interaction disabled by default. Fixtures must explicitly configure both; Z otherwise toggles Run/Walk. Isolated fixtures used the authored Anywhere arc. Realm token files are shared across config homes: don't launch a second account using the first account's saved token; use separate owned token snapshots or authenticate credentials. Never touch another realm's token.

## Remaining boundaries

Saved outfits and Retail3D wardrobe thumbnails remain unimplemented. Illusions require missing authentic local illusion data. Non-override dynamic prices and conditional source eligibility remain the [server contract](../../../../game-server/docs/specs/transmog-apply.md)'s explicit data/policy gaps. Proof covers Retail-definition items in both art skins, not cross-product item-ID translation or every slot.

## Sources

- [Native host](../../../godot/rust/src/transmog.rs), [book](../../../godot/ui-model/src/transmog.rs), [behavioral tests](../../../godot/ui-model/src/transmog_tests.rs).
- [Bag request selection](../../../godot/ui-model/src/game/bag_data.rs), [native caller](../../../godot/rust/src/bag_cursor.rs).
- Local Retail `Blizzard_Transmog.xml`/`.lua`; pinned TrinityCore a352b1fa `TransmogrificationHandler.cpp` (target GetSellPrice, free restoration), retained in canonical diagnostics.

## See Also

- [Server transmog](../../../../game-server/docs/wiki/systems/transmog-apply.md) — pricing, transactions and persisted modifiers.
- [[offline-asset-closure]] — authenticate existing local assets before publishing receipts; filenames alone are not provenance.
