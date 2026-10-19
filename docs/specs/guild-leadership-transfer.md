# Voluntary guild leadership transfer

Mainline `Blizzard_ChatFrameBase/Shared/SlashCommands.lua:793–799` forwards the typed guild-leader command to C_GuildInfo.SetLeader. Use existing native chat input and GuildChannel authority; [guild system](../wiki/systems/guilds.md) owns implementation details.

## What it must do

- [ ] `/gleader <member>` produces one target-only TransferLeadership request, not help or ordinary chat. Existing command case/whitespace handling is retained; no invented buttons or dialogs.
- [ ] Existing authoritative guild replies refresh displayed member ranks and permission controls. Successful transfer is not fabricated from local input.
- [ ] Both skins share the same sourced slash route and unchanged guild geometry.
- [ ] Self-target refusal follows the named server project policy GUILD_SELF_LEADER_TRANSFER_REFUSED_UNSOURCED; no Retail-native self semantics are claimed.

## How it works

- [Guild system](../wiki/systems/guilds.md).

## Implementation inventory

- `godot/ui-model/src/ui/chat_frame.rs`: typed `/gleader` route in the existing chat command parser.
- `godot/rust/src/chat.rs`, `account.rs`, `guild_ranks.rs`: existing input/request/reply boundary.
- `godot/ui-model/tests/native_guild_ranks.rs`: ignored slash regression.

## Tests asserting this spec

- `guild_leadership_transfer_slash_preserves_the_typed_target_without_help_or_chat`.

## Known gaps (current cycle)

- [ ] Behavioral RED/GREEN, coordinated P/S/E gate and real two-member both-skin native authority proof remain pending.

## Out of scope

Disband, inactivity dethroning, authenticator rules, rank reordering, extra guild UI and unrelated chat features. The permitted Retail cache exposes the C API signature but not native implementation; self behavior is explicitly unqualified.
