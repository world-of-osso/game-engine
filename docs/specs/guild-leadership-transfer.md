# Voluntary guild leadership transfer

Mainline `Blizzard_ChatFrameBase/Shared/SlashCommands.lua:793–799` forwards the typed guild-leader command to C_GuildInfo.SetLeader. Use existing native chat input and GuildChannel authority; [guild system](../wiki/systems/guilds.md) owns implementation details.

## What it must do

- [x] `/gleader <member>` produces one target-only TransferLeadership request, not help or ordinary chat. Existing command case/whitespace handling is retained. The separately approved [roster context confirmation](guild-context-leadership.md) owns its sourced menu/dialog, not this slash slice.
- [x] Existing authoritative guild replies refresh displayed member ranks and permission controls. Successful transfer is not fabricated from local input.
- [x] Both skins share the same sourced slash route and unchanged guild geometry.
- [x] Self-target refusal follows the named server project policy GUILD_SELF_LEADER_TRANSFER_REFUSED_UNSOURCED; no Retail-native self semantics are claimed.

## How it works

- [Guild system](../wiki/systems/guilds.md).

## Implementation inventory

- `godot/ui-model/src/ui/chat_frame.rs`: typed `/gleader` route in the existing chat command parser.
- `godot/rust/src/chat.rs`, `account.rs`, `guild_ranks.rs`: existing input/request/reply boundary.
- `godot/ui-model/tests/native_guild_ranks.rs`: slash regression, enabled after actual RED→GREEN.
- `godot/network/examples/native_guild_leadership_peer.rs`: real private authenticated peer and authoritative rank receipts for native proof; no supplied-state injection.

## Tests asserting this spec

- `guild_leadership_transfer_slash_preserves_the_typed_target_without_help_or_chat`.

## Known gaps (current cycle)

None within this slice. gz709 landed the coordinated cohort; two real authenticated members prove typed transfer, immediate rank/permission refresh and fresh-process new-master settings in both skins. See [native acceptance](../wiki/systems/guilds.md#leadership-native-acceptance-2026-10-19). Self refusal remains backend-tested project policy, not a native Retail parity claim.

## Out of scope

Disband, inactivity dethroning, authenticator rules, rank reordering and unrelated chat features. The [sourced context UI](guild-context-leadership.md) is a separate bounded contract. The permitted Retail cache exposes the C API signature but not native implementation; self behavior is explicitly unqualified.
