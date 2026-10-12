# Soft Interact

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Retail soft interact (`SoftTargetInteract`): the interactable NPC or game object the player faces, the cursor icon above it, and the Interact With Target key. Code: `godot/rust/src/soft_interact.rs`, `godot/rust/src/game_objects.rs`, `godot/core/src/client_options_data.rs` (`SoftTargetOptions`), `godot/ui-model/src/ui/screens/options_menu_active_sections.rs` (Interact Key Icons).

References:
- CVar help text: Retail `Wow.exe` strings. Defaults: wow-ui-sim `src/cvars.yaml:1309-1341`, which the Kiosk keyboard reset restores (`Blizzard_Gamepad/Core.lua:83-88`).
- `Blizzard_NamePlates.lua:185-209` and `Blizzard_NamePlates.xml:286-296` (`SoftTargetFrame`)
- `Bindings_Standard.xml:1171-1173` (`INTERACTTARGET`)
- `Blizzard_SettingsDefinitions_Frame/Controls.lua:72-95` (Enable Interact Key, Interact Key binding), `Accessibility.lua:176-221` (Interact Key Icons); labels from GlobalStrings `INTERACT_ICONS_*`
- TrinityCore a352b1fa `Position::HasInLine` (Position.cpp:192), Object.h:371 (`isInFront`), ObjectDefines.h:24, 39-40

## What it must do

- [x] Enable Interact Key (Controls) is off by default. Saved as `hud.softTargetInteract`.
  - Retail `softTargetInteract` defaults to 1, gamepad only. The checkbox switches it between 3 (Any) and 1.
  - While it is off there is no soft interact target and no icon.
- [x] Candidates:
  - living or lootable NPCs whose hover cursor is neither the pointer nor Attack
  - drawn mailboxes, Guild Vaults, chairs, meeting stones/rituals, chests and goobers
- [x] Settings, saved under `hud.softTarget` with Retail's CVar names and defaults (wow-ui-sim `cvars.yaml:1316-1330`): `softTargetInteractArc` 0, `softTargetInteractRange` 10, `softTargetIconEnemy` 0, `softTargetIconInteract` 1, `softTargetIconGameObject` 0, `softTargetLowPriorityIcons` 0.
  - A file saved without them reads the defaults. An arc other than 0-2 fails to load, and a negative range fails validation.
  - Arc and range have no Options control, as in Retail (the gamepad preset sets Arc 1, Range 20, Blizzard_Gamepad/Core.lua:20-33; there is no gamepad support).
- [x] Selection picks the nearest candidate within the arc and within interact range.
  - Arc (Wow.exe CVar help: "0 = No yaw arc allowance, must be directly in front. 1 = Must be in front yaw arc. 2 = Can be anywhere in targeting area."). Retail publishes no angles.
    - 0: modelled as TrinityCore `HasInLine` (Position.cpp:192): the target is in the front half, and the facing line passes within its size. A unit's size is 1.5 yd (`DEFAULT_PLAYER_COMBAT_REACH`); a game object's is 0.389 yd (`DEFAULT_PLAYER_BOUNDING_RADIUS`).
    - 1: modelled as TrinityCore's in-front test, `isInFront` with its default arc `M_PI` (Object.h:371): the front half.
    - 2: no direction test.
  - Range: `softTargetInteractRange` is "limited to ... individual interact ranges". Every client interaction is 5 yd (`INTERACTION_DISTANCE`), so the range is the smaller of the setting and 5 yd.
- [x] Accessibility "Interact Key Icons" (`INTERACT_ICONS_OPTION`, Accessibility.lua:176-221): "NPCs Only (Default)", "Show All", "Show None".
  - Choosing one sets the four icon CVars as Retail's `SetValue` does: Default enemy 0, interact 1, game object 0, low priority 0; Show All all 1; Show None all 0.
  - The shown choice follows Retail's `GetValue`: Show All when all four are on, Show None when all are off, else Default.
  - Retail shows a dropdown; this client shows the three choices side by side.
- [x] Unit icon: `SetUnitCursorTexture`, the unit's hover cursor art (Buy, Speak, Trainer, Taxi, ...), when `softTargetIconInteract` is on.
  - A lootable corpse's Loot icon also needs `softTargetLowPriorityIcons` ("Show interact icons even when there is other visual indicators, such as quest or loot effects").
  - Size: 19 px (`SoftTargetNameplateSize`).
  - Position: bottom 8 px into the top of the unit's plate, or at the plate anchor when it has no plate.
- [x] Game object icon: the world text icon, when `softTargetIconGameObject` is on ("Show icon for soft interact game objects (interactable objects you cannot normally target)").
  - Art: the object's cursor. Retail sends the template `IconName` (TrinityCore `GameObjectTemplate::IconName`, GameObjectData.h:67); world.db leaves it empty for mailboxes and chairs, so the type default applies: Mail (`interface/cursor/mail.blp`) over a mailbox, the Interact gears (`interface/cursor/interact.blp`) over a chair or Guild Vault, as the Bevy client maps them (`cursor_for_interaction`, `src/rendering/ui/wow_cursor.rs`). The pointer hover cursor uses the same mapping.
  - Size: 32 px (`SoftTargetWorldtextSize` 32 × `SoftTargetWorldtextNearScale` 1). Scaling between `SoftTargetWorldtextNearDist` 4 and `SoftTargetWorldtextFarDist` 40 is unpublished and not applied.
  - Position: its bottom centre on the top centre of the object's M2 bounding box.
- [x] Interact With Target (`interact_target`) is unbound by default: the Retail tutorial warns when no key is assigned (Blizzard_Tutorials_Frame_Tutorials.lua:94-99).
  - Pressing it interacts with the soft target through the right-click paths: use the object, loot, attack, or `InteractNpc`.
  - It does not change the hard target.
  - Player immunity (`UNIT_FLAG_IMMUNE_TO_PC`, 0x100) prevents the attack path even for neutral quest givers/enders. NPC-only immunity does not prevent player attacks. Source: TrinityCore `WorldObject::IsValidAttackTarget`; concrete TDB Lord Autumnvale127080 flags320 regression in `auto_attack::tests`.
- [ ] Enemy and friend soft targets (`softTargetIconEnemy` is saved but nothing reads it), `SoftTargetInteractRangeIsHard`, world text icon distance scaling, soft target tooltips.

## Quest objects

- [x] Replicated Type 3 chests and Type 10 goobers render through build-pinned GameObjectDisplayInfo, show the Interact cursor on hover, and right-click sends `UseGameObject` with the object ID (not `InteractNpc`). The server owns model-box reach and objective/loot validation.
- [x] Generic decorations remain non-interactive; spell-focus admission is unchanged. Prior chest/goober exclusion was unimplemented type admission, not a documented crash mitigation.
- [x] Both-skin UDP fixture: `account::meetingstones_tests::quest_objects_and_meetingstones_udp_use_and_combat_answers_both_skins`; cursor contract: `game_objects::tests::quest_objects_show_interact_cursor_and_render_without_changing_decoration_cursor`.

- [ ] Native quest-credit live fixture: `godot/tests/quest_objectives_live.gd`, using authored `QOBJ_SCENARIO` JSON, private `GODOT_TEST_SERVER`, `fb_*` `QF_ACCOUNT`, own `QF_ADMIN` socket and `QF_SHOTS`. Real screen ray-picked right-click → GO use/loot; real bag right-click → item spell; real quest dialog → reward. Admin only creates characters, sets level and travels/revives; it never fills objective counters. Both-skin rendered acceptance remains pending until the fixture succeeds.

## Zone population proof —2026-10-11

Private UDP5916 both Modern/Forever skins use the native Interact Key and physical registry-pointer input to accept/reward Drustvar48677, Isle of Dorn82747 and Terokkar9961; neutral Lord Autumnvale127080 now respects authored player immunity instead of auto-attack. No administrative quest credit. Native taxi discovery/flight2037→2034 is also recorded. [Census, copy command and exact proof boundaries](../../../game-server/docs/zone-population-counts.md) own the dataset;23 inspected PNGs in `/syncthing/AgentShared/2026-10-11/zone-population/`. Appearance/receipt/surface/navigation gaps remain explicit; no complete-world visual certification.

## Proof

- `soft_interact::tests` with world.db data (Northshire vendors Brother Danil 79950, Dermot Johns 79951 and Godric Rothgar 79952, Marshal McBride 79970, the Goldshire Mailbox 26784 and Wooden Chair 26246): selection at arc 0/1/2 and range settings; icons with Default, Show All and Show None.
- `game_objects::tests`: Mail/Interact cursors by type; the icon anchor on top of the Goldshire Mailbox (26784, 199999.m2) and a Goldshire Wooden Chair (26246, 198115.m2).
- `client_options_data_tests`: Retail defaults, old files, arc/range round trip and rejection; the dropdown's `SetValue`/`GetValue`.
- `options_policy::interact_key_icons_choice_projects_and_commits_the_icon_cvars`.
- `godot/tests/world_soft_interact_live.gd` on a private server: the Buy icon appears above Brother Danil, and E opens his MerchantFrame.
- `godot/tests/world_soft_interact_object_live.gd` on a private server from a fresh options directory: after Enable Interact Key, the Goldshire Mailbox is the soft target with no icon; after Show All, the 32 px Mail icon stands on its model top.
