# Disenchant item cursor

Verified2026-10-10 on companion `disenchant` branches, unmerged. [Contract](../../specs/disenchant.md). Native spellbook/action-bar spell13262 enters an item spell cursor before sending the server cast.

## Content

ItemTarget stores a pending SpellCastIntent and spell icon. Occupied bag clicks choose the authoritative inventory GUID; empty slots retain targeting. Sending exits the cursor. Escape, right-click, screen reset or game-menu input cancellation clear it. World left-clicks are consumed without substituting a unit target. Bag input intercepts targeting before pickup, use, sale, mail/trade or repair actions. The existing cursor overlay displays the spell icon; no new frames/art or inventory mutation are introduced.

The new optional target_item_guid is None in ordinary account/IPC/ground cast constructors. CastFailed text uses local Retail GlobalStrings: Item cannot be disenchanted; Your Enchanting skill is not high enough to disenchant that; Spell not learned. The skill refusal exists generically on the protocol, but current authentic metadata never requires nonzero skill; see [server pipeline](../../../../game-server/docs/wiki/systems/disenchant.md).

A LootResponse whose source handle is the local player opens the existing LootFrame. Its first non-auto response honors configured auto-loot with Shift inversion by sending LootUnit(auto=true). The returned auto response does not resend, avoiding a loop. Server owns disposal, eligibility, loot rolls and collection persistence.

## Proof

`spell_targeting` native UI-model tests5/5 pass: item GUID/empty-slot/cancel2 and ground targeting3. The extension builds against the companion protocol with all constructor updates. Pure tests and compilation are not a rendered pointer-routing/live realm proof; no server/service was started. Ledger: `/home/osso/.worktrees/handoff-disenchant.md`.

## Sources
- [Contract](../../specs/disenchant.md)
- `godot/ui-model/src/spell_targeting.rs` — cursor state and tests
- `godot/rust/src/spells/casting.rs`, `spells/ground_target.rs`, `bag_cursor.rs`, `loot.rs` — native activation/input/loot
- Local `data/db2/12.1.0.69933/GlobalStrings.csv` rows10154/12429/16795
- Local Retail `Blizzard_UIPanels_Game/Mainline/ContainerFrame.lua` spell-targeting item-click branch

## See Also
- [[godot-conversion]] — native bag/cursor input
- [[spells]] — spellbook/action-bar casts
