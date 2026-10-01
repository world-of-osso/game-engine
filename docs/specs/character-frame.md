# Character Frame

The Retail `CharacterFrame` with its `PaperDollFrame`, opened by C and the micro menu, over the live server equipment. Equipment stays server-owned: every equip and unequip is an inventory request (shared-protocol `protocol/inventory_messages.rs`); sheet stats are the owner-only replicated `UnitStats` and `CombatRatings` (game-server `sheet_visibility.rs`).

References:
- CF.xml / CF.lua = `Blizzard_UIPanels_Game/Mainline/CharacterFrame.xml` / `.lua`
- PDF.xml / PDF.lua = `Blizzard_UIPanels_Game/Mainline/PaperDollFrame.xml` / `.lua`
- Model: `godot/ui-model/src/character_frame/`; host: `godot/rust/src/character_frame.rs`

## What it must do

- [x] C (`ToggleCharacter("PaperDollFrame")`) and `CharacterMicroButton` toggle the frame; Escape closes it with every bag (`CloseAllWindows`). Micro-menu buttons without a native window log "Micro menu <button> is not converted"; Spellbook and Main Menu open theirs.
- [x] Expanded 540×424 `ButtonFrameTemplate` at the left UI panel slot, toplevel: a press raises it over the bags and merchant, as `toplevel="true"` windows do.
- [x] 18 `PaperDollItemSlotButton`s (no Ranged) with empty-slot textures, item icons, quality borders, and the slot on the cursor dimmed.
- [x] Hover shows the shared item tooltip (`ANCHOR_RIGHT`); an empty slot shows its slot name.
- [x] Equip: drag a bag item onto a slot, click-drop it, or right-click it in the bag (`UseContainerItem` → `EquipItem`). Unequip: drag a slot to a bag slot. A wrong slot shows the server's `ERR_WRONG_SLOT` text.
- [x] A drop on a slot that covers another window's bag slot equips once and sends nothing for the covered slot (ui-ownership `E1_EQUIP_OVER_BAG`).
- [x] Model preview of the local player in `CharacterModelScene`, re-dressed when the replicated appearance changes, weapons sheathed, left-drag rotates; a cursor item dropped on it auto-equips.
- [x] Level line `PLAYER_LEVEL` with spec and class in the class colour; average item level from level 10.
- [x] `AttributesCategory`: Strength, Agility, Intellect, Stamina and Armor as `BreakUpLargeNumbers` integers, under the item level, or at the pane top with 5 more between lines below level 10; hidden until the stats arrive.
- [x] The server replicates `UnitStats` / `CombatRatings` only to the owning connection; equipping updates them.
- [ ] All three primary stats show: Retail hides the two that are not the spec's primary stat, and the client has no `ChrSpecialization` primary-stat data.
- [ ] No `EnhancementsCategory`: crit, haste, mastery and versatility are percentages the server derives (`CharacterStats`, with base crit, auras and the mastery coefficient); `CombatRatings` carries only ratings.
- [ ] Not built: stat tooltips, Stagger and mana regen (need a role), the Reputation and Currency tabs, title and equipment-manager sidebars, slot flyouts.
