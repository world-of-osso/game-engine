# Native guilds

Guild state is server-authoritative. Contracts: [ranks/lifecycle](../../specs/guild-ranks.md), [charters](../../specs/guild-charters.md), [vault](../../specs/guild-bank-frame.md), [tabards](../../specs/guild-tabards.md).

## Lifecycle

`GuildRanksSession` holds roster/permissions, profile, pending invitation and reply pacing. The native `guild_ranks.rs` canvas forwards exact requests; profile updates replace display state. Loading preserves early replies; logout clears them. Modified member clicks reach the native action consumer and on-screen hierarchy-filtered menu. Native LineEdit limits are applied only when changed, before synchronizing text: resetting the same limit resets Godot's caret and reverses typing.

## Charter flow

`guild_charters.rs` forwards authenticated petitioner-role openings to Registrar requests. The registrar replaces its gossip greeting, and its NPC portrait uses the common M2 portrait renderer. Purchase replies close the window; using actual bag item5863 opens PetitionFrame. Owners offer to the selected player's name or rename; recipients render server can_sign. Signature rows and button pacing use the authoritative quota/state. Registration is an NPC service, not a shortcut in Communities.

Server persistence links owned item GUIDs to petitions and signing account IDs. Purchase commits money/item/petition together. Turn-in consumes the actual charter and atomically creates rank0 owner, lowest-rank founders, guild profile and real date. Four distinct signing accounts are required by realm policy; verification therefore used five accounts but never more than two rendered clients. No world.db mutations, vendor fabrication or admin-created guild substitutes.

## Guild tabard data flow

The server stores an optional five-ID design in an additive guild table, preserving legacy rank/vault records. Vendor validation rechecks live NPC role/range, leader status, installed catalog IDs and personal funds; design/payment commit together. Guild profile, a separate replicated design component and character-selection roster entries carry the authoritative appearance without changing owned equipment.

The tabard controller opens the source screen on the vendor reply. Draft changes rebuild the detached player model through the shared equipment compositor; held rotate buttons change model yaw without rebuilding UI. Save stays pending until the typed reply; closing clears the vendor canvas. Communities uses its profile design and installed palette; the no-design branch remains distinct. The [contract](../../specs/guild-tabards.md) records source tiers and controls.

Verified2026-10-14: wire3, server4 and native6 focused tests cover optional roster serialization, durable design/payment, vendor rejection, account delivery, replicated removal, source body tiers and additive arrow hover. Actual character-selection preparation additionally retains all six registered torso layers. Native vendor and production character-selection fixtures completed in both skins with simulated input/replies. Pixel inspection found [BLP palette red/blue reversal](../formats/blp-format.md#palettized-bgra-correction-2026-10-14); corrected both-skin vendor/three-tier/Communities and tier0 character-selection captures passed and fourteen1920×1080 PNGs were inspected. Live networking is a separate proof boundary.

Body textures require authenticated [product/build receipts](../../specs/product-isolated-model-assets.md), not just flat BLP files. All7274 catalog body IDs were matched to the active70391 root/content keys before lead-only publication; missing receipts reproduced a character-selection failure before publication.

## Proof and boundaries

Verified2026-10-11 on leased guilds branches: Retail Aldwin Laughlin, actual purchase/rename/offer/sign/registration in both skins; petition reopen, charter consumption, five-founder roster restart and persisted multiline profile/date. Exact revisions/commands and PNG acceptance are in the linked contracts and canonical `data/diagnostics/guilds4-2026-10-11/`. Vault state/proof stays in [banks](banks.md#guild-lifecycle-and-charters).

GuildInviteFrame has Join/Decline, guild and inviter. Guild tabard appearance now has the separate data flow above; invitation achievement display remains a named gap. Legacy creation dates remain unknown. Independent artifact verification was unavailable because the configured backend's OAuth session expired; native tests/build and inspected private live evidence are recorded separately.

## Sources

- [Charter contract](../../specs/guild-charters.md) — local Retail XML/Lua and pinned TrinityCore PetitionsHandler.cpp/defaults.
- [Lifecycle contract](../../specs/guild-ranks.md) — commands, authority and native regression fixtures.
- [Banks](banks.md) — bank wire/state and native projection boundaries.
- [Tabard contract](../../specs/guild-tabards.md) — Retail GuildUtil/TabardFrame, pinned TrinityCore registration and WoWModelViewer item/tier references; implementation and focused test inventory.

## See Also

- [Native UI automation](../../specs/native-ui-automation.md) — native interaction boundaries.
