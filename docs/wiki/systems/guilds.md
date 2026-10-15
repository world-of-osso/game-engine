# Native guilds

Guild state is server-authoritative. Contracts: [ranks/lifecycle](../../specs/guild-ranks.md), [charters](../../specs/guild-charters.md), [vault](../../specs/guild-bank-frame.md), [tabards](../../specs/guild-tabards.md).

## Lifecycle

`GuildRanksSession` holds roster/permissions, profile, pending invitation and reply pacing. The native `guild_ranks.rs` canvas forwards exact requests; profile updates replace display state. Loading preserves early replies; logout clears them. Modified member clicks reach the native action consumer and on-screen hierarchy-filtered menu. Native LineEdit limits are applied only when changed, before synchronizing text: resetting the same limit resets Godot's caret and reverses typing.

## Charter flow

`guild_charters.rs` forwards authenticated petitioner-role openings to Registrar requests. The registrar replaces its gossip greeting, and its NPC portrait uses the common M2 portrait renderer. Purchase replies close the window; using actual bag item5863 opens PetitionFrame. Owners offer to the selected player's name or rename; recipients render server can_sign. Signature rows and button pacing use the authoritative quota/state. Registration is an NPC service, not a shortcut in Communities.

Server persistence links owned item GUIDs to petitions and signing account IDs. Purchase commits money/item/petition together. Turn-in consumes the actual charter and atomically creates rank0 owner, lowest-rank founders, guild profile and real date. Four distinct signing accounts are required by realm policy; verification therefore used five accounts but never more than two rendered clients. No world.db mutations, vendor fabrication or admin-created guild substitutes.

## Guild tabard data flow

The server stores an optional five-ID design in an additive guild table, preserving legacy rank/vault records. Vendor validation rechecks live NPC role/range, leader status, installed catalog IDs and personal funds; design/payment commit together. Guild profile, a separate replicated design component and character-selection roster entries carry the authoritative appearance without changing owned equipment.

The tabard controller opens the source screen on the vendor reply. Draft changes rebuild the detached player model through the shared equipment compositor; held rotate buttons change model yaw without rebuilding UI. Save stays pending until the typed reply; closing clears the vendor canvas. Communities uses its profile design and installed palette; the no-design branch remains distinct. Arrows filter each field's IDs through existing catalog validation while preserving the other four fields; missing torso combinations are not offered and no texture fallback is used. The [contract](../../specs/guild-tabards.md) records the104 absent installed texture entries, source tiers and controls.

Verified2026-10-14: wire3, server4 and native6 focused tests cover optional roster serialization, durable design/payment, vendor rejection, account delivery, replicated removal, source body tiers and additive arrow hover. Actual character-selection preparation additionally retains all six registered torso layers. Native vendor and production character-selection fixtures completed in both skins with simulated input/replies. Pixel inspection found [BLP palette red/blue reversal](../formats/blp-format.md#palettized-bgra-correction-2026-10-14); corrected both-skin vendor/three-tier/Communities and tier0 character-selection captures passed and fourteen1920×1080 PNGs were inspected. Final refreshed captures also cover catalog-valid arrow filtering and the extracted roster-fixture setup; both scripts pass and all fourteen PNGs were inspected. All-target checks, formatting and the four sparse-variation model tests pass. Live networking is a separate proof boundary; the full protocol landing gate remains lead-owned.

Body textures require authenticated [product/build receipts](../../specs/product-isolated-model-assets.md), not just flat BLP files. All7274 catalog body IDs were matched to the active70391 root/content keys before lead-only publication; missing receipts reproduced a character-selection failure before publication.

## Proof and boundaries

Verified2026-10-11 on leased guilds branches: Retail Aldwin Laughlin, actual purchase/rename/offer/sign/registration in both skins; petition reopen, charter consumption, five-founder roster restart and persisted multiline profile/date. Exact revisions/commands and PNG acceptance are in the linked contracts and canonical `data/diagnostics/guilds4-2026-10-11/`. Vault state/proof stays in [banks](banks.md#guild-lifecycle-and-charters).

GuildInviteFrame has Join/Decline, guild and inviter. Guild tabard appearance now has the separate data flow above; invitation achievement display remains a named gap. Legacy creation dates remain unknown. Independent artifact verification was unavailable because the configured backend's OAuth session expired; native tests/build and inspected private live evidence are recorded separately.

## Communities metadata sorting — verified2026-10-15

Level/Class headers now select `GuildRosterSort::Level/Class`; `sorted_members` joins the existing profile members by character name once, then sorts by numeric level or the existing class-icon mapping's numeric class ID. It never edits either authoritative snapshot. Missing profiles/classes remain absent, not synthesized. Existing Name/Rank behavior and member-action names remain unchanged.

Retail `CommunitiesMemberList.lua:97–108,798–819,822–836,905–914` supplies the level/classID columns and comparator: the initial numeric override swaps comparator operands, giving ascending order; repeat clicks reverse it. Missing attributes begin first and move last in reverse. Numeric ties have no explicit Retail tie order, so none is claimed. `CommunitiesUtil.lua:173–185` applies the default comparator only when the override returns nil, not on equal numeric values.

Observable header-action RED became seven focused Communities controls GREEN atfac28dbcb. Both native scripts passed in both skins: new physical Level/Class sorting and the existing wheel/stepper/thumb/member-menu/tab/invite regression. Ten inspected1920×1080 before/after PNGs: `AgentShared/2026-10-15/ui-gameplay-panels/communities-roster-sort/`; private exact commands, binary hash and manifest: worktree `logs/ui-gameplay-panels/communities-roster-sort/`. Baseline used the old extension's32 level80 fixture members; the updated fixture varies80/20/60/40 for numeric ordering. This is production UI/input with synthetic offline snapshots, not new live guild authority, tie-order parity or service acceptance. Requirements and remaining service gaps stay in the [contract](../../specs/guild-ranks.md).

## Invitation chrome — verified2026-10-15

`GuildInviteFrame.xml:3` inherits TranslucentFrameTemplate, not StaticPopup. `guild_rank_frame::invitation_chrome` now emits the source black0.9 background inset10px, four32px quadrant corners from407329 and tiled16px strips407328/407330/407331/407332. Source: Mainline `UIPanelTemplates.xml:384–399,437–442,495–500,984–1043`; local community-listfile maps those FDIDs to DialogFrame files. The old static_popup style is removed from this popup only. That chrome-only patch preserved root/content/button rectangles and reducer behavior. The subsequent Decline rectangle correction is tracked by the [invitation contract](../../specs/guild-ranks.md#invitation-and-guild-profile); other content-geometry gaps remain separate.

Actual missing-inset RED at37c31d967 retained the invitation action control PASS. Both controls pass at mergedce5740844; a matching native extension verifies source rectangles and physical Join/Decline/pending pacing in both skins. Four inspected1920×1080 before/after PNGs: `AgentShared/2026-10-15/ui-gameplay-panels/guild-invite-chrome/`; exact commands, binary hashes and manifest live in private worktree `logs/ui-gameplay-panels/guild-invite-chrome/`. The fixture is debug-only and supplies an offline payload; no fresh network/authority acceptance. Inner GuildExtra art, tabard/achievement payload, other content geometry and reputation/expiry/Escape remain separate [contract gaps](../../specs/guild-ranks.md#invitation-and-guild-profile).

## Sources

- [Charter contract](../../specs/guild-charters.md) — local Retail XML/Lua and pinned TrinityCore PetitionsHandler.cpp/defaults.
- [Lifecycle contract](../../specs/guild-ranks.md) — commands, authority and native regression fixtures.
- [Banks](banks.md) — bank wire/state and native projection boundaries.
- [Tabard contract](../../specs/guild-tabards.md) — Retail GuildUtil/TabardFrame, pinned TrinityCore registration and WoWModelViewer item/tier references; implementation and focused test inventory.

## See Also

- [Native UI automation](../../specs/native-ui-automation.md) — native interaction boundaries.
