# Native guilds

Guild state is server-authoritative. Contracts: [ranks/lifecycle](../../specs/guild-ranks.md), [charters](../../specs/guild-charters.md), [vault](../../specs/guild-bank-frame.md).

## Lifecycle

`GuildRanksSession` holds roster/permissions, profile, pending invitation and reply pacing. The native `guild_ranks.rs` canvas forwards exact requests; profile updates replace display state. Loading preserves early replies; logout clears them. Modified member clicks reach the native action consumer and on-screen hierarchy-filtered menu. Native LineEdit limits are applied only when changed, before synchronizing text: resetting the same limit resets Godot's caret and reverses typing.

## Charter flow

`guild_charters.rs` forwards authenticated petitioner-role openings to Registrar requests. The registrar replaces its gossip greeting, and its NPC portrait uses the common M2 portrait renderer. Purchase replies close the window; using actual bag item5863 opens PetitionFrame. Owners offer to the selected player's name or rename; recipients render server can_sign. Signature rows and button pacing use the authoritative quota/state. Registration is an NPC service, not a shortcut in Communities.

Server persistence links owned item GUIDs to petitions and signing account IDs. Purchase commits money/item/petition together. Turn-in consumes the actual charter and atomically creates rank0 owner, lowest-rank founders, guild profile and real date. Four distinct signing accounts are required by realm policy; verification therefore used five accounts but never more than two rendered clients. No world.db mutations, vendor fabrication or admin-created guild substitutes.

## Proof and boundaries

Verified2026-10-11 on leased guilds branches: Retail Aldwin Laughlin, actual purchase/rename/offer/sign/registration in both skins; petition reopen, charter consumption, five-founder roster restart and persisted multiline profile/date. Exact revisions/commands and PNG acceptance are in the linked contracts and canonical `data/diagnostics/guilds4-2026-10-11/`. Vault state/proof stays in [banks](banks.md#guild-lifecycle-and-charters).

GuildInviteFrame has Join/Decline, guild and inviter. Tabard/achievement payloads are not present; they remain a data gap rather than fabricated display values. Legacy creation dates remain unknown. Independent artifact verification was unavailable because the configured backend's OAuth session expired; native tests/build and inspected private live evidence are recorded separately.

## Sources

- [Charter contract](../../specs/guild-charters.md) — local Retail XML/Lua and pinned TrinityCore PetitionsHandler.cpp/defaults.
- [Lifecycle contract](../../specs/guild-ranks.md) — commands, authority and native regression fixtures.
- [Banks](banks.md) — bank wire/state and native projection boundaries.

## See Also

- [Native UI automation](../../specs/native-ui-automation.md) — native interaction boundaries.
