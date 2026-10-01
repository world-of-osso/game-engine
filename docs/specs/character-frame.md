# Character Frame

The Retail `CharacterFrame` with its `PaperDollFrame`, opened by C and the micro menu, over the live server equipment. Equipment stays server-owned: every equip and unequip is an inventory request (shared-protocol `protocol/inventory_messages.rs`); sheet stats are the owner-only replicated `UnitStats`, `CombatRatings` and `DerivedStats` (game-server `sheet_visibility.rs`).

References:
- CF.xml / CF.lua = `Blizzard_UIPanels_Game/Mainline/CharacterFrame.xml` / `.lua`
- PDF.xml / PDF.lua = `Blizzard_UIPanels_Game/Mainline/PaperDollFrame.xml` / `.lua`
- Model: `godot/ui-model/src/character_frame/`; host: `godot/rust/src/character_frame.rs`

## What it must do

- [x] C (`ToggleCharacter("PaperDollFrame")`) and `CharacterMicroButton` toggle the frame; Escape closes it with every bag (`CloseAllWindows`). Micro-menu buttons without a native window log "Micro menu <button> is not converted"; Spellbook and Main Menu open theirs.
- [x] Expanded 540×424 `ButtonFrameTemplate` at the left UI panel slot, toplevel: a press raises it over the bags and merchant, as `toplevel="true"` windows do.
- [x] 18 `PaperDollItemSlotButton`s (no Ranged) with empty-slot textures, item icons, quality borders, and the slot on the cursor dimmed.
- [x] Hover shows the shared native GameTooltip (`tooltip_sources.rs` `paperdoll_tooltip`, `SetInventoryItem`, `ANCHOR_RIGHT`, Shift comparison); an empty slot shows its white slot name; nothing while the cursor holds an item.
- [x] Equip: drag a bag item onto a slot, click-drop it, or right-click it in the bag (`UseContainerItem` → `EquipItem`). Unequip: drag a slot to a bag slot. A wrong slot shows the server's `ERR_WRONG_SLOT` text.
- [x] A drop on a slot raised over another window equips once and does nothing to the window underneath: with the MerchantFrame under the Hands slot, a bag item released there sends one `SwapItem` to `Equipment(Hands)` and no `SellItem` (ui-ownership `E1_EQUIP_OVER_MERCHANT`). No layout puts a bag under the frame: both keep fixed Retail anchors and the HUD scale follows the window width.
- [x] Model preview of the local player in `CharacterModelScene`, re-dressed when the replicated appearance changes, weapons sheathed, left-drag rotates; a cursor item dropped on it auto-equips.
- [x] Level line `PLAYER_LEVEL` with spec and class in the class colour; average item level from level 10.
- [x] `AttributesCategory`: Strength, Agility, Intellect, Stamina and Armor as `BreakUpLargeNumbers` integers, under the item level, or at the pane top with 5 more between lines below level 10; hidden until the stats arrive.
- [x] The server replicates `UnitStats` / `CombatRatings` / `DerivedStats` only to the owning connection; equipping updates them.
- [x] With a known spec only its primary stat shows (`ChrSpecialization.PrimaryStatPriority` mapped as TrinityCore `Player::GetPrimaryStat`); without one all three show.
- [x] `EnhancementsCategory` under the last attribute (11 lower below level 10): Critical Strike, Haste, Mastery, Versatility, Leech, Avoidance and Speed from the server's `DerivedStats` percentages, `format("%d%%", value + 0.5)`, each hidden at exactly 0. No content item carries a tertiary stat, so Leech, Avoidance and Speed stay hidden in play.
- [ ] Dodge, Parry and Block (tank role, shield) and red negative haste are not built.
- [ ] Not built: stat tooltips, Stagger and mana regen (need a role), the Reputation and Currency tabs, title and equipment-manager sidebars, slot flyouts.
