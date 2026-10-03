# Pet Frame and Pet Action Bar

The hunter pet's PetFrame and PetActionBar in the Godot client. The server owns the pet and its bar; the contract is shared-protocol `protocol/pet_messages.rs` (`PetSpells`, `PetClearSpells`, client `PetAction`, all on `CombatChannel`) and the replicated `UnitSummonedBy` and `UnitFlags::PET_IN_COMBAT` (`components/unit_frames.rs`).

References (`~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`): `Blizzard_UnitFrame/Mainline/PetFrame.xml`, `PlayerFrame.xml:467-474`, `Shared/PlayerFrameTemplates.xml`; `Blizzard_ActionBar/Mainline/PetActionBar.xml`, `Shared/PetActionBar.lua`, `Mainline/ActionButtonTemplate.xml:198-207`, `Shared/ActionButton.lua:1671-1717`, `Shared/ActionBar.lua`; `Blizzard_EditMode/Mainline/EditModePresetLayouts.lua`, `Shared/EditModeManager.lua:630-671`; `Blizzard_SharedXML/LayoutFrame.lua:275-361`.

## What it must do

- [x] The local player's pet is the replicated unit whose `UnitSummonedBy` is the local player's server entity.
- [x] PetFrame (120×49, TargetofTarget art) hangs under PlayerFrame as the first `PlayerBottomManagedFrameContainer` child: TOP at PlayerFrame BOTTOM + (30 + 7.5, 25), i.e. left edge 438.5 left of the screen centre, bottom 226. Portrait 37×37 at (5, 5) in a circle mask, gold name 68×10 at (44, 5), health bar 70×10 at (44, 17), power bar 74×7 at (40, 28) with the `TargetofTarget-PortraitOn-Bar-<power>` fill (Focus for hunter pets). Retail PetFrame has no level text; none is drawn.
- [x] PetActionBar shows while a `PetSpells` bar exists and its pet is replicated; `PetClearSpells` hides it. `PetSpells`/`PetClearSpells` keep their channel order.
- [x] Ten 30×30 small buttons 2 px apart (318×30 after `UpdateGridLayout`; the XML 509×43 is pre-layout), BOTTOMLEFT at screen BOTTOM + (−281, 95): stacked on MainActionBar with `BOTTOM_ACTION_BARS_SPACER_Y` 5.
- [x] Buttons render whatever the server sends: command/reaction tokens with `PET_*_TEXTURE` icons (listfile FDIDs 132152, 132328, 136106, 457329, 524348, 132110, 132311, 132277), spells with their `SpellMisc` icon, empty slots blank.
- [x] Checked: the command equal to `command_state`, the reaction equal to `react_state`, Attack while the pet has `PET_IN_COMBAT` (checked texture at alpha 0.5 and the Flash toggling every 0.4 s).
- [x] Click or Ctrl-N sends `PetAction` with the packed button: Attack and spells with the current target, Follow/Stay/stances without one. Move To enters ground targeting; the next left click on visible terrain/WMO/doodad sends its point in `Position` space (inverse of unit placement under the WorldUnits root); right click or Escape cancels; a click on the sky keeps targeting.
- [x] `dump-ui-tree` shows `PetFrame`, `PetActionBar`, `PetActionButton1..10` with `...CheckedTexture`/`...Flash` visible or hidden; `GameClient.pet_bar_state()` reports pet, states, buttons, checked, Move To and sent actions.
- [ ] Not built: autocast overlay and right-click autocast toggle, `PetAttackModeTexture` pulse, pet auras, pet cast bar, ground reticle cursor, usable/range tinting, pet bar cooldowns, warlock pets below a class bar.

## Tests asserting this spec

- `godot/core/tests/input_bindings_data.rs`: `bonus_action_buttons_default_to_ctrl_digits_and_shadow_the_main_bar`, `hotkey_text_abbreviates_modifiers`.
- `godot/network/src/pet_wire_tests.rs`: PetSpells/PetClearSpells order over UDP; PetAction send with target and position.
- `godot/network/src/replica/tests.rs`: `unit_summoned_by_reaches_the_replica_and_leaves_it`.
- `godot/ui-model/tests/pet_frame.rs`, `godot/ui-model/tests/pet_action_bar.rs`: geometry, art, icons, checked and flash state, hidden without a pet.
- `godot/rust/src/replicated.rs`: local pet lookup; `account.rs`: PetSpells/PetClearSpells dispatch; `pet_bar.rs`: Move To position inverse.
