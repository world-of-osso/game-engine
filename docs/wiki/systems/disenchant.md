# Disenchant item cursor

Verified2026-10-10: gameplay implementation merged to engine/server master; private live proof and read-only diagnostics on `disenchant-live`. [Contract](../../specs/disenchant.md). Native spellbook/action-bar spell13262 enters an item spell cursor before sending the server cast.

## Content

ItemTarget stores a pending SpellCastIntent and spell icon. Occupied bag clicks choose the authoritative inventory GUID; empty slots retain targeting. Sending exits the cursor. Escape, right-click, screen reset or game-menu input cancellation clear it. World left-clicks are consumed without substituting a unit target. Bag input intercepts targeting before pickup, use, sale, mail/trade or repair actions. The existing cursor overlay displays the spell icon; no new frames/art or inventory mutation are introduced.

The new optional target_item_guid is None in ordinary account/IPC/ground cast constructors. CastFailed text uses local Retail GlobalStrings: Item cannot be disenchanted; Your Enchanting skill is not high enough to disenchant that; Spell not learned. The skill refusal exists generically on the protocol, but current authentic metadata never requires nonzero skill; see [server pipeline](../../../../game-server/docs/wiki/systems/disenchant.md).

A LootResponse whose source handle is the local player opens the existing LootFrame. Its first non-auto response honors configured auto-loot with Shift inversion by sending LootUnit(auto=true). The returned auto response does not resend, avoiding a loop. Server owns disposal, eligibility, loot rolls and collection persistence.

## Proof

`spell_targeting` native UI-model tests5/5 pass: item GUID/empty-slot/cancel2 and ground targeting3. The extension builds against the companion protocol with all constructor updates. Pure tests and compilation are not a rendered pointer-routing/live realm proof; no server/service was started. Ledger: `/home/osso/.worktrees/handoff-disenchant.md`.

## Private live proof — 2026-10-10

Engine `588689217` (base `b516eacd`), prebuilt server `2aa9da0`, shared protocol `0e1a9713`. Private UDP5617, fresh `fb_disenchant_proof` account/character31; canonical world.db used only through a read-only backup. No shared realm, canonical build or DB writes by this agent.

| Requirement | Forever | Modern |
|---|---|---|
| Spellbook waits for a bag target | Sent list unchanged before item click | Sent list unchanged before item click |
| Eligible item consumed |727 Notched Shortsword removed |816 Small Hand Blade removed |
| TDB material + manual LootFrame | Entry3:10940 Strange Dust×2 | Entry3:10938 Lesser Magic Essence×2 |
| Appearance collected |214 absent→present |242 absent→present |
| Ineligible item retained | Grey25 refused/retained | Grey25 refused/retained |

Appearance proof reads `tooltip_state().appearance_collection`, the sorted IDs from the account's authoritative `AppearanceCollectionUpdate`, not the misleading bag/equipment-only IPC `item info appearance_known`. GDS snapshot mutation-isolation fixture RED→GREEN; authoritative replacement/deduplication unit test1/1; native build passes. Appearance214/216/242 remain after real logout/re-entry. This proves collection data, not a native wardrobe UI, which remains absent. No separate action-bar, cancellation or auto-loot/Shift proof.

PNG evidence: `/syncthing/AgentShared/2026-10-10/disenchant-live/`. Durable snapshots, inventory replies, logs and census: `data/diagnostics/disenchant-live-2026-10-10/`; handoff `/home/osso/.worktrees/handoff-disenchant-live.md`.

### Stale TDB material gap

Read-only census:180 rows/100 entries;72 direct-item rows across50 entries reference16 materials absent from `content_item` and both Retail `Item.csv`/`ItemSparse.csv`. Forty entries wholly unavailable; ten partially unavailable. No reference-type rows. Exact row IDs/counts live in `disenchant-census.json` under the evidence directory.

Live753 selects entry14:11082/11083/11084 all absent. Pre-fix server disposes it and collects216, but generates empty personal loot. The native LootFrame hides empty slots; that unclaimed server loot session then rejects the next eligible754 cast with “Another action is in progress.” Relog clears the runtime lock;754 remains. Entry28's11178 is also absent. This is incompatible TDB/build data, not permission to fabricate retired materials. [Server branch519a34d](../../../../game-server/docs/wiki/systems/disenchant.md#stale-material-compatibility--2026-10-10) refuses wholly unavailable entries at validation and actual-empty rolls before success/disposal. Targeted8/8, native check and format pass; no post-fix rendered run or master merge. Missing materials remain unavailable.

## Sources
- [Contract](../../specs/disenchant.md)
- `godot/ui-model/src/spell_targeting.rs` — cursor state and tests
- `godot/rust/src/spells/casting.rs`, `spells/ground_target.rs`, `bag_cursor.rs`, `loot.rs` — native activation/input/loot
- Local `data/db2/12.1.0.69933/GlobalStrings.csv` rows10154/12429/16795
- Local Retail `Blizzard_UIPanels_Game/Mainline/ContainerFrame.lua` spell-targeting item-click branch

## See Also
- [[godot-conversion]] — native bag/cursor input
- [[spells]] — spellbook/action-bar casts
