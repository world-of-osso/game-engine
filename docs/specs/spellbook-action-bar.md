# Spellbook, Action Bar and Casting

The Retail 12.x spellbook, main action bar, casting bar and cast feedback in the Godot client. The server owns known spells, the action bar and every cast; the contract is shared-protocol `protocol/spell_messages.rs` (`KnownSpellsSnapshot`, `SpellsLearned`, `SpellsUnlearned`, `SpecializationChanged` and `ActionBarSnapshot` on `TalentChannel`; `SpellCooldownUpdate`, `CastFailed`, `CombatLogEvent` on `CombatChannel`), client-to-server `SpellCastIntent { spell_id, spell, target_entity }` (`core_messages.rs`), and the replicated `CastState` component (`casting.rs`).

Which spells a character knows is server truth (game-server `class_progression.rs`, [class progression](../../../game-server/docs/wiki/systems/class-progression.md)): auto-learned class-line `SkillLineAbility` rows (AcquireMethod 1, 2, 4) whose `SpellLevels.SpellLevel` is at most the level, the Initial spec below level 10, and the active spec's `SpecializationSpells` up to the level (TrinityCore `Player::LearnSkillRewardedSpells`, `Player::LearnSpecializationSpells`, `Player.cpp:25407`, `:30651`). There are no class trainers in 12.x for these.

References (under `/syncthing/Sync/Projects/wow/reference-addons.new/wow-ui-source/Interface/AddOns/`, 12.0.0.65727):
- SBF.xml = `Blizzard_PlayerSpells/SpellBook/Blizzard_SpellBookFrame.xml`; SBI.xml/.lua = `Blizzard_SpellBookItem.*`; SBT.xml = `Blizzard_SpellBookTemplates.xml`
- PCG.lua = `Blizzard_PagedContent/Blizzard_PagedCondensedGridContentFrame.lua`; PC.xml = `Blizzard_PagingControls.xml`; TST.xml = `Blizzard_SharedXML/Shared/TabSystem/TabSystemTemplates.xml`
- MAB.xml = `Blizzard_ActionBar/Mainline/MainActionBar.xml`; ABT.xml = `Mainline/ActionButtonTemplate.xml`; EMPL.lua = `Blizzard_EditMode/Mainline/EditModePresetLayouts.lua`
- data: `data/db2/12.1.0.69933` (SkillLine, SkillLineAbility, SpellLevels, SpecializationSpells, SpellMisc, UiTextureAtlasMember, GlobalColor); strings from GlobalStrings.

## What it must do

- [x] Known spells, spec, action bar and cooldowns follow the server messages; leaving the world clears them.
- [x] The bar, casting bar, spellbook and tooltip are authored in UI units on the 768-unit UIParent canvas and scaled by viewport height / 768 (PixelUtil.lua:3-6), like every in-world HUD layer.
- [x] Spellbook (P toggles, Escape closes, close button): 1612×856 evergreen book scaled to fit the viewport (SBF.xml), category tabs at TOPLEFT 70,-19 (TST.xml art, tab on top). Categories: the class (its class line and active spec line as headed groups) then General; an empty category is not shown.
- [x] Pages: two 680×650 views (View1 TOPLEFT 85,-65, View2 TOPRIGHT -50,-65), headers take a row, items fill 3 columns column-first with the fewest rows (PCG.lua), `xPadding` 15, `yPadding` 10, `spacerSize` 20; a group that runs out of height continues in the next view; "Page %d/%d" (`PAGE_NUMBER_WITH_MAX`) with previous/next buttons.
- [x] Items (SBI.xml, 60 high): 36 px icon in a square (active) or circle (passive) border, name in `SystemFont_Large` and `SPELLBOOK_FONT_COLOR`, "Passive" subtext for passives. Spells flagged `SPELL_ATTR0_DO_NOT_DISPLAY` (SpellMisc Attributes_0 0x80) are not listed.
- [x] Spells learned at later levels (the class line's and the active spec's, race-masked) follow the known ones in level order, greyed (`unlearnedTextAlpha` 0.6, icon desaturated and tinted `SPELLBOOK_UNLEARNED_TINT_COLOR`, inactive border) with "Level %d" (`SPELLBOOK_AVAILABLE_AT`); they cannot be cast.
- [x] Clicking a known active spell's icon casts it.
- [x] Authored left-pointer-down on a known active spellbook icon or main action-bar button starts the owned native Effects click before mouse-up at `master × effects × 0.55`; release, right pointer input, keyboard casts, and spellbook reopening do not replay it.
- [x] Main action bar (MAB.xml, EMPL.lua Modern): 12 buttons of 45×45, 2 px apart, BOTTOM y 45, gryphon end caps, `ActionButtonTemplate` art (slot background, slot art, normal/pushed/highlight), hot keys from the bindings, "1".."0","-","=" by default (12 px ArialNarrow grey, TOPRIGHT -4,-5).
- [x] Keys 1..= (bindings `ActionSlot1..12`) and clicks use the slot: a spell sends `SpellCastIntent` with the current target's server entity bits (none without a target). The pushed art shows for 0.15 s.
- [x] Committed HUD “Show Action Bars” hides the entire native main bar without clearing its cached UI or slot snapshot. Bound keys still cast while hidden; restoring the toggle immediately shows the same bar and allows pointer casting again.
- [x] Buttons sweep the running cooldown, or the GCD for spells with `StartRecoveryTime`: a black 0.8 swipe over the icon inset 3 px, draining; countdown numbers for cooldowns of 2 s or longer.
- [x] `CastFailed` shows its Retail text in UIErrorsFrame (`cast_failed_text`: `ERR_GENERIC_NO_TARGET` "You have no target.", `ERR_OUT_OF_RANGE`, `SPELL_FAILED_NOT_READY`, "Not enough rage.", ...; a server `detail` wins).
- [x] The local player's replicated `CastState` shows the casting bar (fill, spell name, remaining time; channels drain), advanced locally between updates and hidden when the server removes it.
- [x] The player's own `CombatLogEvent` damage and misses float over the target (fixed on-screen size, rising and fading over 1.5 s, white for physical, yellow for other schools, "N!" for crits). Each number starts at its own offset in the camera plane from a low-discrepancy sequence, up to `WorldTextStartPosRandomness_v2` (1.0) units sideways and `WorldTextRandomZMax_v2 - WorldTextRandomZMin_v2` (0.7) units up, one unit being 1.5 hit text heights at the target's depth because the text keeps a fixed screen size (wowless retail 12.0.7 `cvars.yaml:1622-1626`), so any four numbers in a row start at least 0.5 units apart; each pops 1.5× and settles over `WorldTextRampDuration_v2` (1 s) with `WorldTextRampPow_v2` 1.9, crits with `WorldTextRampPowCrit_v2` 8. The CVars' engine formulas are unpublished; this mapping is read from their names.
- [x] The server pushes each newly learned active spell (level-up, spec change, talent) into the first empty main bar slot (Retail `SPELL_PUSHED_TO_ACTIONBAR`); the client shows the resulting `ActionBarSnapshot`.
- [x] Hovering a spellbook icon or a main bar button shows the GameTooltip: white name, cost | range (`RAGE_COST` "%s Rage" with rage/runic power in tenths, `MELEE_RANGE` up to 5 yd, `SPELL_RANGE` "%s yd range", min-max bands), cast time | cooldown (`SPELL_CAST_TIME_INSTANT`, `SPELL_CAST_TIME_SEC`, `SPELL_RECAST_TIME_SEC/MIN` with `%.2g`), "Passive" for passives, red "Level N" for future spells, then the description rendered by the shared token renderer (docs/reference/spell-description-tokens.md) in gold, word-wrapped to 280 px; right of spellbook icons, above bar buttons.
- [ ] Description values that scale with attack/spell power stay the renderer's `{?$s1}` marker (the client has no player stats).
- [ ] Not built: drag to/from the bar, other action bars and paging, range/usable/power tinting, charges, the interrupted bar, spellbook search, the settings dropdown, flyouts, pet spells and the minimized book; icon masks (`spellbook-item-spellicon-mask`, `UI-HUD-ActionBar-IconFrame-Mask`); the Retail cast bar art (the shared Bevy bar uses flat colours).

## Assumptions

- Icons missing from the local CASC install show an empty slot (no substitute icon). Several Arms spells' icons (132306, 132400, 970853, 6718291) are not in the local archives.
- The category tab width is estimated from the label length (Retail measures the font string).

## Tests asserting this spec

- `godot/core/tests/spell_catalog.rs`: warrior names/icons, rendered Battle Shout description; level 1 and level 10 Arms spellbook lists with future spells by level.
- `godot/ui-model/tests/spellbook_frame.rs`: column-first grid (PCG.lua examples), view continuation, Level N and cast actions, desaturation, fit scale.
- `godot/ui-model/tests/main_action_bar.rs`: bar geometry, keys, icon, cooldown swipe/countdown, click actions.
- `godot/ui-model/tests/spell_tooltip.rs`: line order, colours, height.
- `godot/rust/src/spell_tooltip.rs` tests: Slam "20 Rage | Melee Range", "Instant"; Charge "8-25 yd range", cast/cooldown formats; word wrap and colour escapes.
- `godot/rust/src/player_spells.rs` and `spells.rs` unit tests: learn order, bar snapshot, cooldown vs GCD, categories, countdown text.
- `godot/tests/spellbook_cast.gd` (live, `127.0.0.1:5000`): a level 1 and a level 10 warrior show the retail lists; hovering Slam shows its tooltip; with `SPELL_CAST=1` next to the Northshire training dummies: Slam without a target fails "You have no target.", Tab targets the dummy, Slam deals damage with the GCD sweep and floating text, a second Slam fails "Spell is not ready yet.", Battle Shout from the spellbook starts the GCD (level 10).
- `godot/tests/world_spell_click_flow.gd` through test-only `native_input_fixture sound-click`: owned UDP local-player snapshots; action-bar and spellbook left-down observe owned Effects active or finished within 40 ms before release at gain `0.44`; right/release/keyboard/reopen do not replay that pointer effect. The same fixture toggles the committed HUD setting off/on, checks the cached bar identity and retained slot, the hidden bound-key `SpellCastIntent`, and the restored pointer click; the quiet-input baseline is sampled after that intentional click. It also passes CastStart request/repeat/inactive/reset/mute/removal audio stages. Scoped `--locked` pass: `/tmp/claude/actionbar-consumer-targeted-green.log` (commit `67e6e430`, base `539f1b86` plus scoped diff); verifier941 is pending, so no independent-pass claim.
- game-server `class_progression_tests.rs`: `spec_spells_wait_for_their_spell_level`, `learned_spells_fill_first_empty_main_bar_slots`.
