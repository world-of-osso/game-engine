# Guild ranks and permissions

Retail guild settings belong in the native guild/communities frame, in Modern and
Forever skins. `godot/ui-model/src/guild_ranks.rs` provides portable request/state
decisions; `guild_rank_frame.rs` renders the native roster/settings canvas and
`godot/rust/src/guild_ranks.rs` connects it to GuildChannel. See [banks](../wiki/systems/banks.md#rank-settings-model) for current data flow.

## What it must do

### Rank controls
- [x] Guild settings → Guild Ranks opens the same Retail functionality in both skins.
- [x] Guild Master alone adds, removes, renames, reorders and configures ranks; rank 0 is immutable, there are 2–10 ranks and names have at most 15 characters.
- [x] Occupied ranks cannot be removed; reordered members retain their rank identity.
- [x] Requests leave selected-rank data unchanged until the authoritative reply arrives.
- [x] Mutating controls wait for authoritative state before sending another write; queued clicks cannot overwrite a prior permission edit using stale flags.

### Permissions and members
- [x] Selected rank shows permission checkboxes for guild/officer chat, invite/remove/promote/demote, MOTD and existing officer-note/info features.
- [x] Per-tab view/deposit/withdraw-stacks controls show server state. Zero denies withdrawals; only Guild Master is unlimited.
- [x] Whole-gold input sends copper/day, sharing the allowance with guild repairs.
- [x] Member context menu shows Promote/Demote only when flags and strict hierarchy allow it; promotion stops below the actor's rank.
- [x] Server refusals display Retail GlobalStrings. `/o` and `/officer` use the Officer channel and its styling.

## How it works

- [Banks](../wiki/systems/banks.md#rank-settings-model) — authoritative model and native integration.
- Shared-protocol `docs/guild-ranks.md` — request/state wire contract.

## Implementation inventory

| File | Role |
|---|---|
| `godot/ui-model/src/guild_ranks.rs` | Selected server state and exact request values |
| `godot/ui-model/src/guild_rank_frame.rs` | Native communities entry, rank widgets and member menu |
| `godot/rust/src/guild_ranks.rs` | Mount, poll clicks, send and apply native state |
| `godot/network/src/lib.rs`, `godot/rust/src/account.rs` | Receive GuildRanksState; send requests on GuildChannel |
| `godot/ui-model/src/game/chat_data.rs` | Officer channel styling classification |
| `godot/ui-model/src/ui/chat_frame.rs` | Officer chat command parsing |

## Tests asserting this spec

- `godot/ui-model/tests/native_guild_ranks.rs`: 5 authoritative model/Officer tests pass.
- `godot/ui-model/tests/guild_rank_widgets.rs`: 9 tests pass in both skins, including
  mounted state, exact click requests, disabled controls, hierarchy, pacing and purchased tabs.
- `godot/network/src/guild_rank_wire_tests.rs`: real loopback UDP request/reply proof passes.
- `godot/rust/src/account.rs`: native state/refusal dispatch test passes.
- Existing `micro_menu` binary: 11 regression tests pass.
- `godot/tests/capture_ui_screen.gd`: native rendered disclaimer bands pass in both skins.

Native build and inspected 1920×1080 captures pass at `1e62844b`.
Exact names, commands, revisions and exclusions: `data/diagnostics/guildranks-2026-10-05/proof.md`.

## Proof boundaries

No requested native rank UI feature remains unimplemented. No live authenticated
server smoke was run; the server slot was read-only. Both capture processes exit 0,
but logs retain texture/font shutdown leaks. Full communities/social functionality,
pixel-perfect Retail parity and clean-resource acceptance are not claimed.

## Out of scope

None of the requested native UI behavior is waived. The partial model is not completion
of the full feature. Forever changes presentation, not Retail mechanics.

## Retail sources

Root: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`.

- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.lua:4,536–620`: rank count/order and occupied deletion.
- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.xml:7`: 15-letter rank names.
- `Blizzard_GuildControlUI/Blizzard_GuildControlUI.lua:137–238,420–459`: tab rights, counts, flags and gold/repair input.
- `Blizzard_Communities/GuildRoster.lua:137–141`: member rank actions.
- `Blizzard_Communities/CommunitiesFrame.xml:265`: GuildControlUI entry point.
- `data/GlobalStrings.csv`: Retail refusal strings.

See [guild bank](guild-bank-frame.md) for vault actions and game-server
`docs/specs/guild-ranks.md` for authority, durability and enforcement.
