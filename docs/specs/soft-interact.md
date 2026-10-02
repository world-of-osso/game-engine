# Soft Interact

Retail soft interact (`SoftTargetInteract`): the interactable NPC or game object the player faces, the cursor icon above it, and the Interact With Target key. Code: `godot/rust/src/soft_interact.rs`, `godot/rust/src/game_objects.rs`.

References:
- CVar help text: Retail `Wow.exe` strings. Defaults: wow-ui-sim `src/cvars.yaml:1309-1341`, which the Kiosk keyboard reset restores (`Blizzard_Gamepad/Core.lua:83-88`).
- `Blizzard_NamePlates.lua:185-209` and `Blizzard_NamePlates.xml:286-296` (`SoftTargetFrame`)
- `Bindings_Standard.xml:1171-1173` (`INTERACTTARGET`)
- `Blizzard_SettingsDefinitions_Frame/Controls.lua:72-95` (Enable Interact Key, Interact Key binding)
- TrinityCore a352b1fa `Position::HasInLine` (Position.cpp:192), ObjectDefines.h:24, 39-40

## What it must do

- [x] Enable Interact Key (Controls) is on by default. Saved as `hud.softTargetInteract`; a file saved without the key reads as on.
  - Deviates from Retail by user decision on 2026-10-02: Retail `softTargetInteract` defaults to 1, gamepad only. The checkbox switches it between 3 (Any) and 1.
  - While it is off there is no soft interact target and no icon.
- [x] Candidates:
  - living or lootable NPCs whose hover cursor is neither the pointer nor Attack
  - drawn mailboxes, Guild Vaults and chairs
- [x] Selection picks the nearest candidate that is directly in front of the player and within interact range.
  - Arc: `SoftTargetInteractArc` 0, "No yaw arc allowance, must be directly in front".
  - Modelled as `HasInLine`: the target is in the front half, and the facing line passes within its size. A unit's size is 1.5 yd (`DEFAULT_PLAYER_COMBAT_REACH`); a game object's is 0.389 yd (`DEFAULT_PLAYER_BOUNDING_RADIUS`). The exact Retail arc-0 width is not published.
  - Range: `SoftTargetInteractRange` 10 is "limited to ... individual interact ranges". Every client interaction is 5 yd (`INTERACTION_DISTANCE`), so the range is 5 yd.
- [x] Icon: `SetUnitCursorTexture`, the unit's hover cursor art (Buy, Speak, Trainer, Taxi, ...).
  - Size: 19 px (`SoftTargetNameplateSize`).
  - Position: bottom 8 px into the top of the unit's plate, or at the plate anchor when it has no plate.
  - Lootable corpses show no icon (`SoftTargetLowPriorityIcons` 0).
- [x] Game object icon: the world text icon (`SoftTargetIconGameObject` 1, "interactable objects you cannot normally target").
  - On by default, deviating from Retail's 0 by user decision on 2026-10-02. No option.
  - Art: the object's cursor. Retail sends the template `IconName` (TrinityCore `GameObjectTemplate::IconName`, GameObjectData.h:67); world.db leaves it empty for mailboxes and chairs, so the type default applies: Mail (`interface/cursor/mail.blp`) over a mailbox, the Interact gears (`interface/cursor/interact.blp`) over a chair or Guild Vault, as the Bevy client maps them (`cursor_for_interaction`, `src/rendering/ui/wow_cursor.rs`). The pointer hover cursor uses the same mapping.
  - Size: 32 px (`SoftTargetWorldtextSize` 32 × `SoftTargetWorldtextNearScale` 1). Scaling between `SoftTargetWorldtextNearDist` 4 and `SoftTargetWorldtextFarDist` 40 is unpublished and not applied.
  - Position: its bottom centre on the top centre of the object's M2 bounding box.
- [x] Interact With Target (`interact_target`) is unbound by default: the Retail tutorial warns when no key is assigned (Blizzard_Tutorials_Frame_Tutorials.lua:94-99).
  - Pressing it interacts with the soft target through the right-click paths: use the object, loot, attack, or `InteractNpc`.
  - It does not change the hard target.
- [ ] Enemy and friend soft targets, `SoftTargetIconGameObject`/`SoftTargetLowPriorityIcons`/arc/range options, world text icon distance scaling, soft target tooltips.

## Proof

- `soft_interact::tests` (7) with world.db data: Northshire vendors Brother Danil 79950, Dermot Johns 79951 and Godric Rothgar 79952, Marshal McBride 79970, and the Goldshire Mailbox 26784.
- `game_objects::tests`: Mail/Interact cursors by type; the icon anchor on top of the Goldshire Mailbox (26784, 199999.m2) and a Goldshire Wooden Chair (26246, 198115.m2).
- `client_options_data_tests::soft_interact_is_on_by_default_and_for_files_saved_before_it`.
- `godot/tests/world_soft_interact_live.gd` on a private server: the Buy icon appears above Brother Danil, and E opens his MerchantFrame.
