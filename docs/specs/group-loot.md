# Group loot

Retail Mainline roll prompts and loot history, transported by `LootChannel` and rendered through native RegistryUi. Forever changes art only. Sources: local `GroupLootFrame.lua`/`.xml`, `LootHistory.lua`/`.xml`, `UnitPopupButtons.lua`, `LootConstantsDocumentation.lua`, `SlashCommands.lua:1429`, and `GlobalStrings.csv` (`SLASH_OPEN_LOOT_HISTORY*`, `LOOT_ROLL_*`).

## What it must do
- [ ] Show up to four 277×67 prompts; queue additional rolls. Need is disabled with the server's reason; Greed is replaced by Transmog where eligible; Pass remains available.
- [ ] Send only roll identity and choice; server supplies RNG, eligibility, timeout, resolution and delivery. Confirmation removes the local prompt; expired prompts cannot submit.
- [ ] Announce new choices, authoritative numeric roll results on completion, and winners once in chat. `/loot` and `/lootrolls` toggle the 261×590 loot-history panel with shared native scrolling and participant-roll tooltips.
- [ ] Replay history on world entry; no invented winner or local inventory grant.
- [ ] Leader-only Group Loot/Personal and Uncommon/Rare/Epic threshold actions in the existing unit menu.
- [ ] Same elements, dimensions and behaviors in Modern and Forever; product-local atlas art only.

## How it works
- [Group loot integration](../wiki/systems/group-loot.md)

## Implementation inventory
- `godot/ui-model/src/group_loot.rs`: state, choices and chat deltas.
- `godot/ui-model/src/group_loot_component.rs`: Retail prompts/history.
- `godot/ui-model/src/game/group_state.rs`: policy menu commands.
- `godot/network/src/lib.rs`: ordered roll relay.
- `godot/rust/src/account.rs`, `loot.rs`: protocol dispatch and native host.
- `godot/rust/src/chat.rs`, `tooltip_sources.rs`: slash history and hover content.

## Tests asserting this spec
- `godot/ui-model/src/group_loot_tests.rs`
- `godot/ui-model/src/game/group_state_tests.rs`
- `godot/network/src/group_loot_wire_tests.rs`

## Known gaps (current cycle)
- [ ] Native live proof, both skins; screenshot inspection.
- [ ] Encounter selection/resizing in loot history; main-spec Need animation.

## Out of scope
- Master-loot UI; Forever-only frames or behavior; canonical realm or world-data mutations.
