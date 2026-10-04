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
- [x] Reputation tab (`CharacterFrameTab2` → `ToggleCharacter("ReputationFrame")`, Mainline/CharacterFrame.lua:394-406): hides the paper doll (slots, model, level, stats) and shows the `ReputationFrame` titled "Reputation" in `NORMAL_FONT_COLOR`; Modern narrows the frame to 400 (CharacterFrame.lua:11-15), Forever keeps 631×484. One `ReputationEntryTemplate` per faction of the server's `ReputationStateUpdate` (sent after enter world and on every change), in its order: name and a bar with `FACTION_STANDING_LABEL<n>`, `FACTION_BAR_COLORS`, filled by the progress through the standing, full at Exalted (ReputationFrame.lua:482-503). Before the update arrives the list is empty. Tab1 or `ToggleCharacter("PaperDollFrame")` (C, the micro button) returns to the paper doll; C on the paper doll closes. The update's message is a chat system line, its error a UIErrors line.
- [ ] Reputation: faction headers (the wire has none), the scroll bar (entries past the scroll box bottom are not drawn), detail frame, at-war/inactive/watch toggles, paragon and major factions; Forever's right-pane stone cap stays drawn (Camelot hides it off the paper doll, CharacterFrame.lua:371).
- [ ] Not built: stat tooltips, Stagger and mana regen (need a role), the Currency tab (no currencies on the wire; Retail opens nothing without them, CharacterFrame.lua:67-73), title and equipment-manager sidebars, slot flyouts.

## Forever first slice

- Modern must retain the captured pre-conversion registry bytes, including hidden/empty and populated/equipped states. Atlas-backed character art resolves Blizzard element names through the active skin; the resulting FDID/UV widget representation stays unchanged under Modern.
- Forever must show the existing 18 slots and server-backed stats inside Camelot's 631×484 portrait frame: 398px left pane, 233px right pane, model filling the left pane, centered GearSlot art, and c60 general/stat/stone/class/level backgrounds. Classic inner-border strips and weapon caps are absent. Character/Reputation retain their existing actions and labels on vertically stacked c60 mode-tab chrome; player-portrait/icon content is not added to this view.
- Do not create skills, ranged/ammo slots, pet loyalty/experience, PvP rank, resistance, title/equipment-set, currency or statistics data. No missing-texture fallback, extraction, server change or live-run claim belongs to this slice.
- Sources: `wowforever/AddOns/Blizzard_UIPanels_Game/Camelot/CharacterFrameConstants.lua:4-5`, `CharacterFrame.xml:403-482,566-589`, `PaperDollFrame.xml:67-70,431-470,607-647,773-857`, `PaperDollFrame.lua:1880`; `Blizzard_SharedXML/Mainline/SharedUIPanelTemplates.lua:313-318` derives the tab hit height by subtracting 5px from its atlas art. Retail atlas names come from `retail/AddOns/Blizzard_UIPanels_Game/Mainline/CharacterFrame.xml:81,105,150,229,296` and `Blizzard_SharedXML/Mainline/SharedUIPanelTemplates.xml:76,137,909-935`. Concrete Modern/Forever region contracts and the authentic Modern tree capture live in `godot/ui-model/tests/forever_character_frame.rs` and `tests/fixtures/modern_character_trees.rs`.
