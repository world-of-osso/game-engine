# Guild roster context Whisper

The Communities guild roster opens Mainline `COMMUNITIES_GUILD_MEMBER` (`Blizzard_Communities/CommunitiesMemberList.lua:27–30,718–747`; entries in `Blizzard_UnitPopup/Standard/UnitPopupMenus.lua:152–172`), whose Interact section contains Whisper before Promote to Guildmaster. This slice adds only Whisper, reusing the existing chat input and whisper route; [guild system](../wiki/systems/guilds.md) owns implementation/proof details.

## What it must do

- [ ] Both skins show `Whisper` (authenticated local-CASC70448 GlobalStrings13134) for an online roster member who is not the local player, before `Promote to Guildmaster` when both are present. Visibility follows Whisper CanShow (`Blizzard_UnitPopup/Standard/UnitPopupButtons.lua:404–433`, `UnitPopupSharedUtils.lua:100–110`) and does not depend on guild rank authority or pending guild replies.
- [ ] Self, offline and unknown-profile members get no Whisper entry, and the action does nothing for them.
- [ ] Activating Whisper closes the menu and requests one chat open for the captured member, with the existing `/w <name> ` prefill. It sends no guild or network request and makes no local roster change; any delivery uses the existing whisper route.
- [ ] Existing Promote/Remove/Demote/Promote to Guildmaster entries and actions are unchanged.

## How it works

- [Guild system](../wiki/systems/guilds.md).
- [Roster leadership context confirmation](guild-context-leadership.md).

## Implementation inventory

- `godot/ui-model/src/guild_rank_frame.rs`: existing roster context menu.
- `godot/ui-model/src/guild_ranks.rs`: reducer eligibility and one-shot chat-open intent.
- `godot/rust/src/guild_ranks.rs`, `godot/rust/src/chat.rs`: host consumes the intent through the existing `open_chat_input`.
- `godot/ui-model/tests/guild_context_whisper_tests.rs`: observable menu/intent controls.

## Tests asserting this spec

- `guild_context_whisper_shows_for_online_other_member_before_guildmaster_both_skins`.
- `guild_context_whisper_hidden_for_self_offline_and_unknown_profile_both_skins`.
- `guild_context_whisper_opens_existing_chat_prefill_without_guild_request_both_skins`.

## Known gaps (current cycle)

- [ ] `CHAT_WHISPER_HEADER_ABSENT`: Retail `ChatFrameUtil.SendTell` (`ChatFrameUtil.lua:367–386`) opens chat with `SLASH_WHISPER1` = `/cw` (GlobalStrings23451) and `ParseText(0)` switches to whisper mode with header `Tell %s: ` (9951). The engine has no `/cw` parsing or sticky header mode, so it uses the existing `/w <name> ` prefill (the same as R-key Reply). Header-mode parity is a separate chat slice.
- [ ] Ignored tests/spec first, then actual RED → committed implementation → targeted GREEN, independent gate and native proof after landing.

## Out of scope

Invite submenu, Target, Add Friend, Ignore, Report, Copy Character Name, View Houses, voice, Leave, BNet whisper and mobile presence (not represented). Mainline `COMMUNITIES_GUILD_MEMBER` has no Promote/Demote/Remove entries; removing or reordering the existing entries needs a separate decision.
