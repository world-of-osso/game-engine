# Nameplate style

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

This spec defines the requested WoW-reference overhead health and spell bars. Rendering lives in `src/rendering/ui/`; [nameplate design](../wiki/design/nameplate-design.md) distinguishes implemented data paths from its separate target-first design.

## What it must do

- [ ] Pixel-match the supplied reference for every non-glyph nameplate pixel: bar/frame bounds, fill fractions, gaps, endpoints, placement, and colors. Glyph rasterization may differ, but labels must retain white Friz styling at 13px for names and 10px for casts.
- [x] Game settings expose independent Thin/Thick selectors, persisted across launches. Defaults are Thick health and Thick spellbar (the user's reference of 2026-10-04, `data/diagnostics/forever-reference/user-nameplate-reference-2026-10-04.png`, which replaces the earlier Thin spellbar choice).
- [x] One `NameplateStyle` data source drives the renderer (no hard-coded sizes/colors): health bar width/height, health fill color per reaction (hostile/neutral/friendly/player class color toggle), cast bar width/height, cast fill color for normal cast, channel, and uninterruptible casts, border visibility, name/cast font size. Values persist with client options and are editable in Options; the Thin/Thick selectors become presets of it. Addon-style plate content replacement is out of scope (user, 2026-09-24).
- [x] Default fill colours are Retail's: health by the owner's reaction to the local player (`UnitSelectionColor` hostile red, neutral yellow, friendly green), players by class colour (`NamePlateFriendlyFrameOptions.useClassColors`), casts by CastingBar start (1, 0.7, 0), channel (0, 1, 0) and non-interruptible (0.7, 0.7, 0.7) colours. Reaction comes from `FactionTemplate` through `shared::faction_reaction`, the rules the server uses (FactionTemplate 7, the Defias Thug, is neutral to a Human player).
- [x] Thick health bar (user reference 2026-10-04, which overrides Retail's name above the bar): the name sits inside the bar, its left 2px from the body's left end, and the health percent ("100%", rounded up) inside at the right; `NameplateStyle.show_health_value` (`showHealthValue`, default off, user 2026-10-04: "by default only show name and life %") puts `AbbreviateLargeNumbers(health)` before it ("425 K  100%"). The health text sits 2px from the body's right end (left of the Forever level frame); both white Friz at the name font size with a black outline, on the bar's middle line. A name that would come within 6px of the health text is trimmed with an ellipsis. Applies to both HUD presets.
- [x] Thin health bar (10px, lower than a text line): the name is centred above the bar, its bottom 2px above the plate's top edge (Retail `CenteredAboveHealthBar`, `HEALTH_BAR_TO_NAME_ABOVE_SPACING`), and there is no health text.
- [ ] `NAMEPLATE_SCALE = 0.5`: effective health width is 188px; health heights are 20px Thick / 10px Thin; cast heights are 15px Thick (the 2026-10-04 reference's 31px track at 2x) / 6px Thin. These targets derive from a 376px raw interior; bitmap outer bounds vary by frame. This is not pixel verified.
- [x] Active replicated player casts show spell name and normal-fill/channel-drain progress; removal/completion/cancellation hides the bar. Worker-to-main snapshots preserve additions, elapsed progress, and removal.
- [x] Existing UI-disabled, scene-stage, nameplate, health-bar, and distance controls remain effective.
- [ ] A `LocalPlayer` owner has no projected name, health, or cast visual, including when the marker is added after those visuals already exist; removing the marker restores the remote plate.
- [ ] Name, health, and cast visuals share the health-body distance anchor and fade/hide boundary. `HudOptions.nameplate_distance` remains the configured policy; no native retail cap is asserted.
- [ ] Clicking a visible non-local plate targets its owner before mesh raycasting. Registry UI under the cursor retains input precedence.
- [x] A unit whose replicated `UnitFlags` carry `UNIT_FLAG_NOT_SELECTABLE` (0x02000000; triggers such as the Summon Enabler Stalker) has no projected name, health or cast visual, and cannot be clicked, hovered, cursor-highlighted or tab-targeted (Retail). Clearing the flag restores the plate and selection. `sync_not_selectable` keeps the `NotSelectable` marker in step with `UnitFlags`.

### Tap-denied health (Godot client)

- [ ] A creature tapped by neither the selected character nor a current group member uses Retail `(0.9,0.9,0.9)` health tint before reaction/selection colouring, in both skins. Player-controlled units remain exempt. Source: cached `Blizzard_UnitFrame/Shared/CompactUnitFrame.lua:561-563,675-677`; [stable wire design and proof](../wiki/systems/death-flow.md#player-resurrection-offers-and-tap-eligibility). Tests: `godot/rust/src/nameplates.rs` and `replicated.rs` (`rezrtap` filter).

### Retail visibility and occlusion (Godot client)

Retail decides plate visibility in the engine from CVars; the default UI only exposes them (`Blizzard_SettingsDefinitions_Frame/Nameplates.lua:454,460,480,513`, `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns`). Defaults are the engine CVar table (wow-ui-sim `src/cvars.yaml:977-1008`, from wowless). The UI source does not contain the engine's per-unit combat test.

- [x] `nameplateShowAll = 0` (cvars.yaml:993): a plate shows only for the local player's target and for units in combat with the player. "Nameplates by default are only shown in combat. Check this to show all nameplates all the time." (`OPTION_TOOLTIP_UNIT_NAMEPLATES_AUTOMODE`, GlobalStrings); the NAMEPLATES binding reports "Enemy Nameplates Turned On (Combat)" while it is 0 (`Blizzard_FrameXML/Bindings_Standard.xml:1121-1133`). With 1, every unit of an enabled kind shows.
- [x] "In combat with the player" is the unit's replicated `CombatStatus` with the local player as its replicated `UnitTarget`. The client has no threat list, so a unit fighting the player while targeting someone else has no plate.
- [x] `nameplateShowEnemies = 1` (cvars.yaml:997) covers units the player can attack (`shared::faction_reaction::can_attack` and no `UNIT_FLAG_NON_ATTACKABLE_2`), neutral ones included. `nameplateShowFriendlyPlayers = 0` (cvars.yaml:1008) and `nameplateShowFriendlyNpcs = 0` (cvars.yaml:1004) hide every friendly plate, the target's included.
- [x] `nameplateMaxDistance = 60` (cvars.yaml:977): no plate beyond 60 yd from the local player. Separately, `HudOptions.nameplate_distance` fades the projected plate from half its configured camera-to-health-body distance to zero at the configured distance. `HudOptions.show_nameplates` still turns every plate off.
- [x] The local player, dead units and `UNIT_FLAG_NOT_SELECTABLE` units never have a plate.
- [x] `nameplateSelectedAlpha = 1.0` (cvars.yaml:990): the target's plate takes it in place of the camera-distance fade, so it stays opaque and shown at any camera distance within `nameplateMaxDistance`.
- [x] `nameplateOccludedAlphaMult = 0.4` (cvars.yaml:984): a plate whose unit is hidden from the camera multiplies its alpha (the selected alpha or the distance fade) by 0.4, else by 1. The test is a ray from the camera to the centre of the unit's pick box against terrain (physics layer 1) and WMO collision (layer 2). M2 doodads have no collision and do not occlude.
- [x] Native HUD `show_health_bars = false` hides the health frame and fill but retains and centers the name. Accessibility `colorblind_mode` changes only the label: player cyan `(0.45, 0.9, 1.0)`, NPC yellow `(1.0, 0.92, 0.35)`; off restores white. Reaction-based health fill remains unchanged. Independent native verification fresh-runs the owned loopback NPC fixture through the live NPC node, health visibility, NPC label restoration, unchanged fill tint, and camera-body fade separately from CVar eligibility (`/tmp/claude/verify-native-nameplate-options.md`). The pure test asserts both exact label colors; it is not live player rendering. Current rules reject player-vs-player attacks, friendly-player plates default off, and no authored control enables them, leaving live player-label proof blocked (`/tmp/claude/nameplate-player-*.log`).
- [x] Godot look and layout come from the Bevy client: reference skins, the Thick/Thin frame chosen by the nearest preset, the fill desaturated then tinted by reaction, a normally white 13px Friz name (placed as above), and the body centred 2.5 yd above the unit origin (Bevy `BAR_Y_OFFSET`). 1 UI unit is 1 viewport pixel.
- [x] Classification indicator (`NamePlateClassificationFrameMixin:GetClassificationAtlasElement`, Blizzard_NamePlateClassificationFrame.lua:90-130): plates of units that are not friendly (`NamePlateEnemyFrameOptions.showClassificationIndicator`) show `nameplates-icon-elite-gold` for elite and world-boss units, `nameplates-icon-elite-silver` for rare elites and `UI-HUD-UnitFrame-Target-PortraitOn-Boss-Rare-Star` for rares, at 20×20 with its RIGHT on the 22×22 raid-icon slot's LEFT (Blizzard_NamePlates.xml:195-214). A raid icon or a name-only plate hides it. Only the Medium size (`classificationScale` 1) exists; PvP carrier icons are not implemented. Live: `godot/tests/plateclass_live.gd` on a private server (UDP 5187) shows Hogger's gold dragon 22px left of the health bar, `data/diagnostics/plateclass-2026-10-02/`.

### Distance-alpha evidence and fixture oracle (2026-10-05)

Local search of this checkout, `~/Repos`, the [client reference catalog](../wiki/reference/open-source-wow-clients.md), and the cached Blizzard Retail Lua found **no authoritative native-engine interpolation implementation**. The referenced wow-ui-sim `src/cvars.yaml` is not installed on this host. Lua settings expose CVars, not the engine's distance curve. Do not describe the legacy HUD fade as Retail interpolation.

Retail CVar defaults in [wowless's Retail snapshot](https://github.com/ferronn-dev/wowless/blob/2c6dc52fe7dc2f359ba92a75b359dc373c9206a1/data/products/wow/cvars.yaml) (2022-10-26; not a fresh 2026 client measurement), quoted individually:

| CVar | Source entry |
| --- | --- |
| `nameplateMinAlpha` | `nameplateMinAlpha: '0.6'` |
| `nameplateMaxAlpha` | `nameplateMaxAlpha: '1.0'` |
| `nameplateMinAlphaDistance` | `nameplateMinAlphaDistance: '10'` |
| `nameplateMaxAlphaDistance` | `nameplateMaxAlphaDistance: '40'` |
| `nameplateMaxDistance` | `nameplateMaxDistance: '60'` |
| `nameplateSelectedAlpha` | `nameplateSelectedAlpha: '1.0'` |
| `nameplateOccludedAlphaMult` | `nameplateOccludedAlphaMult: '0.4'` |

[AdvancedInterfaceOptions's CVar descriptions](https://github.com/Stanzilla/AdvancedInterfaceOptions/blob/cc09dad593346e9e0d12ed2e57f3554c542161ec/cvars.lua#L142-L155) distinguish the maximum-alpha endpoint (camera distance) from `nameplateMinAlphaDistance`: "The distance from the max distance that nameplates will reach their minimum alpha." Thus 10 is an offset from the range limit, **not** a near endpoint of 10 yd. With the snapshot defaults, the described endpoints are 40 yd / alpha 1 and 50 yd / alpha 0.6. Neither source supplies a native interpolation formula, clamp behavior, or target/occlusion precedence; those remain unverified Retail details. No inferred linear Retail curve is implemented here.

The **existing project contract**, implemented in `godot/core/src/rendering/ui/nameplate_visibility_data.rs`, is separate: for camera-to-health-body distance `d`, authored HUD limit `L`, and `near = max(L/2, 1)`, unselected fade is 1 below `near`, 0 at/above `L`, otherwise the source expression `1.0 - (distance - fade_near) / (fade_far - fade_near)`. Selection replaces that fade with 1; occlusion then multiplies either result by 0.4. Target selection does not bypass `plate_shown`'s player-to-unit 60 yd eligibility limit. The addon describes the selected CVar as the selected plate's alpha and occlusion as an alpha multiplier, but this project's precise composition is not independently measured Retail proof.

At HUD limit 40, unselected clear/occluded alpha at camera distances 10/20/30/40/60 yd is 1/1/0.5/0/0 and 0.4/0.4/0.2/0/0. Selected clear/occluded alpha is 1/0.4 at every one of those distances, provided the unit is eligible. Under this formula, a fade of 0.219 corresponds to a camera-body distance of 35.62 yd; it is not the selected alpha. The flow's `40.0` argument is the HUD limit, not a measured unit distance. This arithmetic reconstructs the reported fade; it does not measure the live client's state.

`world_nameplate_options_flow.gd` selects the NPC with Tab after every Options change. Its old `expect_distance` oracle incorrectly multiplied the unselected fade by occlusion; its 20 yd check also incorrectly expected the selected plate to disappear. Both now assert the existing selected override, including beyond the HUD fade boundary. The checked-out product already applies that override in `project_plate`; no product curve changed. Rust regression `nameplate_alpha_distances_preserve_selected_and_occlusion_overrides` covers both branches and the reported 0.219 sample. The live GDScript flow is deliberately unrun for this task.

### Cast bars (Godot client)

Retail `NamePlateCastingBarMixin` over `CastingBarMixin` (`Blizzard_NamePlates/Blizzard_NamePlateCastingBar.lua`, `Blizzard_UIPanels_Game/Shared/CastingBarFrame.lua`). The replicated `CastState` is `UnitCastingInfo`/`UnitChannelInfo`; the game server's `SpellGo` is `UNIT_SPELLCAST_STOP` and `SpellFailure` (`SMSG_SPELL_FAILURE`, TrinityCore `Spell::SendInterrupted`) is `UNIT_SPELLCAST_INTERRUPTED`/`_FAILED`.

- [x] A cast fills and a channel drains under the health bar with the spell name and icon; an uninterruptible cast shows `nameplates-InterruptShield` instead of the icon and the uninterruptible fill colour.
- [x] The health and cast rows form one block: the cast track starts on the health frame's drawn left edge and ends on the plate's right edge (the frame's drawn right edge, or the level frame's end under Forever; the body's ends without the border), under the Thick and Thin presets. The frame bitmaps carry transparent end columns, so the drawn edge is inside the bitmap's rect. A cast width edited away from the health width widens or narrows the track evenly. The track sits 2px under the health body; on the Thick preset the 12px icon sits at its left end with the outlined spell name 2px right of it inside the bar (user reference 2026-10-04). The Bevy glow frame bitmaps are gone: the reference has none.
- [x] Nameplate pip matches approved `data/ui/nameplate-pip-previews/B-4x.png` (user, 2026-10-05): a warm white-gold soft radial glow centered on the fill edge, sigma approximately 0.9×2.25 UI units, cropped within the bar's height. Both Modern/Forever skins and Thin/Thick bars use the same glow; additive blending remains Retail's (`Blizzard_NamePlateCastingBar.xml:52`). Fill, text, icon and border are unchanged.
- [x] `SpellGo` fills the bar and fades it (`FadeOutAnim`, 0.2 s then 0.3 s). A replicated channel that ends fades the same way.
- [x] `SpellFailure` turns the bar and soft glow red, reads `Interrupted: <interrupter>` in the interrupter's class colour for a kick, `Interrupted` without an interrupter, `Failed` for a failure on completion, holds 1.0 s and fades 0.3 s (`HoldFadeOutAnim`).
- [x] A removed cast keeps running until its `SpellGo` or `SpellFailure`; the same cast still replicated after either does not restart the bar, and a new cast of the same spell after a replication gap does.

### Auras (Godot client)

Ported from the Bevy client's `nameplate_auras.rs`. Retail's debuff list takes harmful auras (`AuraUtil.AuraFilters.Harmful`, `Blizzard_NamePlates/Blizzard_NamePlateAuras.lua:85`) cast by the local player (`requireSourceIsLocalPlayer`, :292).

- [x] A plate of a unit the player can attack shows the first six debuffs the local player cast on it, in replicated order: 18px spell icons on a 1px black border, 22px apart from the health body's left end, their bottoms 4px above the plate (above the name on the Thin bar), each with its `SecondsToTimeAbbrev` countdown ("12 s", "2 m") in white 9px Friz on the icon's centre. Hidden with the health bars.
- [ ] Retail shows only debuffs whose spell has `nameplateShowPersonal` (Blizzard_NamePlateAuras.lua:210, unless `nameplateShowAllPersonalAuras`): the client holds no such flag (not in `AuraView` nor the spell catalog), so every debuff of the player's shows. Retail's 25px item size (`AURA_ITEM_HEIGHT`), cooldown swipe, stack count, crowd-control and buff lists are not drawn.

### Skins (Godot client)

- [x] Cast bar background, fill and interrupt shield are drawn by atlas name (`ui-castingbar-background`, `ui-castingbar-filling-standard`, `nameplates-InterruptShield`) from the active skin's `UiTextureAtlas` members: Retail `uicastingbar` under Modern, Forever's set-1 `uicastingbarc60` under Forever. A skin switch rebuilds the plates.
- [x] Forever plates carry Camelot's `NameplateLevelFrame` (`Blizzard_NamePlates/Camelot/Blizzard_NamePlateLevelFrame.xml`): 28 wide, 23 high on the Thick health bar and 16 on the Thin one, taking the right end of the health bars; `ui-hud-nameplates-levelindicator` behind the unit's level, `-skull` only for an unknown effective level (world boss or more than 10 levels above the player), `-selected` on the target. Modern plates have none.
- [x] Forever health fill is border-free, horizontally dark-to-bright. Screenshot `data/diagnostics/forever-reference/user-nameplate-reference-2026-10-04.png`: glyph-free stops at (54,35)/(440,35) are RGB (0,38,74)/(0,73,144). Use their HSV values 74/255 and 144/255 as the tintable gradient stops; existing reaction/class colour remains the tint. Both presets use the same stops. Modern retains its original bitmaps.
- [x] Forever health frame has a 1 UI-pixel edge, fill abutting its inside on all four sides at full health. Retain authored body width/height and the 28px level slot; only the edge/inset changes. Screenshot above: clean lower edge at (250,67) is RGB (63,84,104). A numeric 3x3 hollow edge patch uses that colour, never FlareUI Media art. Cast top remains the resulting health frame bottom; approved pip B is unchanged.
- [x] Level 1 Training Dummy (template 44548) viewed at level 40 stays numeric, including elite classification. A world boss or level 51 against 40 shows a skull; level 50 stays numeric. Missing level hides the slot, not a skull. FlareUI `Modules/UnitFrames.lua:357-370` distinguishes unknown `UnitLevel < 0` from elite/rare suffixes; it does not implement the engine's numeric threshold. The >10 threshold is this task's explicit Retail requirement. Resolve effective levels through shared `level_for_viewer`, as the target frame does.

#### Level sheet crop mismatch

The local Blizzard BLP `data/textures/8165538.blp` (64x64) disagrees with cached 1.60.1.69913 `UiTextureAtlasMember.csv` rows 39470-39472: the selected-border row's [1,19) x [25,43) crop contains the skull face. `apply_level` showed that selected texture on a level-1 target even while `highLevelTexture` was hidden. No missing level or elite flag caused that face.

Nameplate-local crops follow this actual sheet: icon [27,45) x [45,63), selected ring [29,47) x [25,43), skull [1,27) x [25,51). Pixel tests decode the cached BLP and prove the selected ring has a hollow centre. This scoped numeric correction is constrained to FDID 8165538; retire it when the imported atlas tables and local CASC texture are from the same layout and their decoded crop tests pass. No global atlas mapping or Modern visual changes.

- [ ] Level text difficulty colour (`GetDifficultyColor`, `C_QuestLog.GetTrivialRange`): no data source; the text is white.
- [x] Both skins tint Blizzard's white radial `OBJFX_Glow` (FDID 959719) gold/red instead of selecting opaque pip atlas crops; no FlareUI Media art.
- [ ] Focus colour of the level frame's selected border, and Camelot's hidden classification indicator (`Camelot/Blizzard_NamePlateFrameOptionsOverrides.lua:1`).

## How it works

- [Nameplate design](../wiki/design/nameplate-design.md)
- [Networking](../wiki/systems/networking.md)

## Implementation inventory

- `src/game/nameplate_style.rs` — `NameplateStyle` (sizes, colours, presets, ranges) and the Options slider model.
- `src/game/state/client_options.rs` — persists `HudOptions.nameplate_style` (`nameplateStyle`).
- `src/scenes/game_menu/options.rs` — settings draft/apply wiring, style sliders and toggles.
- `godot/ui-model/src/ui/screens/options_menu_active_sections.rs` — HUD Thin/Thick preset selectors.
- `godot/ui-model/src/ui/screens/options_menu_active_sections_nameplates.rs` — Options > Nameplates page.
- `godot/rust/src/game/faction_reaction.rs` — `FactionTemplate.csv` loader for `shared::faction_reaction`.
- `src/rendering/ui/nameplate_art.rs` — shared art cache: reference-derived health/cast frame PNGs with transparent live interiors; glyph-free health gradient crop; authored `4505182` cast fill/background. No generic pip is rendered because none appears in the reference.
- `debug/make_nameplate_skins.py` — reproducibly derives frame/fill skins from the supplied screenshot using linear unmatting; `provenance.json` records crops, source hash, and reconstruction limits.
- `debug/compare_nameplates.py` — compares aligned half-size GPU captures against the BOX-resized reference while excluding glyph regions; diagnostics only, not acceptance proof.
- `src/rendering/ui/health_bar.rs` — UI-overlay health sprites and screen sizing; health is not PBR-rendered.
- `src/rendering/ui/nameplate.rs` — projected owner names and health-label placement.
- `src/rendering/ui/nameplate_cast_bar.rs` — projected cast atlas frame/fill, labels, visibility gates, and shared distance boundary.
- `src/rendering/ui/nameplate_picking.rs` — screen-space hit testing for projected nameplate parts.
- `src/rendering/ui/target.rs` — registry-first click routing; a plate hit selects its owner before world mesh raycasting.
- `src/network_runtime/replication.rs` — cast and `UnitFlags` snapshots across worker/main worlds.
- `src/game/networking/npc.rs` — `NotSelectable` marker from `UnitFlags`; plate, pick, hover and target queries exclude it.
- `../game-server/crates/server/src/cast_presentation.rs` — validated player cast-state lifecycle; no spell effect execution or NPC cast source.

- `godot/core/src/rendering/ui/nameplate_visibility_data.rs` — engine-free CVar defaults and the Retail visibility/alpha rules, shared by both clients.
- `godot/rust/src/nameplates.rs` — Godot plates: rule inputs from snapshots, occlusion ray, CanvasLayer nodes, `nameplate_state()`/`nameplate_rules(id)` automation, `NameplateProbe`.
- `godot/rust/src/replicated.rs` — `faction_template`, `unit_flags`, `in_combat` from the host `Replica`.
- `godot/rust/src/nameplate_casts.rs` — engine-free cast bar state from `CastState`, `SpellGo` and `SpellFailure`.
- `godot/rust/src/nameplate_cast_bar.rs` — cast bar nodes: track, fill, pip, shield, icon and name row.
- `godot/rust/src/nameplate_auras.rs` — which auras a plate shows and where their icons sit.

## Tests asserting this spec

- `src/game/nameplate_style.rs` — colour per reaction/class and cast type, presets, clamping, slider keys.
- `src/game/state/client_options_tests.rs` — style persistence round trip and missing-field presets.
- `src/rendering/ui/health_bar_facing_tests.rs` — FactionTemplate 7/14/11 fills against a Human player, class colour, edited sizes, border hiding.
- `src/rendering/ui/nameplate_bar_tests.rs` — name centred 2px above the plate for both presets and hidden border; font size.
- `src/rendering/ui/nameplate_projection_tests.rs` — cast fill colours and sizes; alignment under a scaled UI camera.
- `src/scenes/game_menu/options_tests.rs`, `godot/ui-model/src/ui/screens/game_menu_component_tests.rs` — Options editing and the Nameplates page.
- `src/rendering/ui/nameplate_projection_tests.rs` — late-local-owner exclusion and shared body-distance/fade behavior.
- `src/rendering/ui/target_nameplate_tests.rs` — plate-owner selection, mesh precedence, registry UI precedence, and hidden/local exclusion.
- `src/rendering/ui/nameplate_gpu_tests.rs` — half-size visual comparison fixture.
- `godot/core/src/nameplate_visibility_data_tests.rs` — CVar defaults; target/combat/show-all rules; enemy vs friendly player/NPC switches; local, unselectable, dead and far units; occluded alpha; the target's plate opaque at any camera distance.
- `godot/network/src/wire_tests.rs` `native_bridge_receives_faction_flags_and_combat_status` — the rule inputs over loopback UDP, including a combat drop.
- `godot/rust/src/nameplates.rs` tests — plate layout around the anchor (Thick with the texts inside, borderless Thin, health fill), health text for 425,000/425,000, 42/55, 12,345/20,000 and 3.5e9/4e9, name trimming, and the rule inputs of a targeted friendly NPC (no plate) and a targeted or fighting hostile NPC (plate).
- `godot/rust/src/nameplate_auras.rs` tests — only the local player's debuffs on an enemy plate with their countdown, the six-aura cap, icon rects.
- `godot/rust/src/nameplates.rs` `cast_row_is_flush_with_the_health_row_at_both_ends` — cast track and icon on the health row's edges for every preset pair, with and without the border and the level frame.
- `godot/tests/nameplate_occlusion.gd` — the occlusion ray on real Godot physics.
- `godot/tests/world_nameplate_flow.gd` — in world on the dev server.
- `godot/rust/src/nameplate_casts_tests.rs` — fill/drain, shield, SpellGo finish, interrupt/failure texts and holds, replication races.
- `godot/rust/src/nameplate_skin_tests.rs` — cast bar crops under Modern and Forever, Forever level frame atlases and layout, Modern plate without one.
- `native_npc_visual_fixture nameplate-casts` + `godot/tests/world_nameplate_casts_flow.gd` — the fixture server replicates the targeted enemy's `CastState` and sends `SpellFailure`/`SpellGo`; the live plate shows kick, uninterruptible resolve, channel and completion failure.
- `godot/tests/world_nameplate_options_flow.gd` — owned loopback NPC fixture for authored HUD/Accessibility controls, label/fill visibility, label color/restoration, and camera fade versus CVar eligibility; no live player-label rendering.
- `not_selectable_unit_flags_hide_every_plate_part_until_cleared`, `clicking_a_not_selectable_npc_model_selects_nothing`, `tab_target_skips_not_selectable_npcs`, `unit_frame_snapshot_preserves_powers_auras_level_faction_flags_target_and_removal`.

## Known gaps (current cycle)

- [ ] Godot: combat-driven plates are proven by unit and replication tests only. The client cannot attack or cast, and the creatures near Fbworldmap are neutral, so no in-world fixture starts combat.
- [ ] Godot: no class colours, plate click-to-target, distance-based alpha (`nameplateMinAlpha` 0.6 over `nameplate{Min,Max}AlphaDistance`), selected/min scale, or overlap stacking. The authored HUD health-visibility, Accessibility label-color, and legacy camera-fade controls are covered separately; CVar Options remain fixed defaults.
- [ ] The owned fixture does not render a live player label, so player cyan label runtime proof is pending; verifier874 is pending.

- [ ] Reaction is template-only: reputation (forced ranks, at-war, Faction reputation bases) is not consulted, on client or server. Diseased Timber/Young Wolves (FactionTemplate 32) therefore read neutral although Retail shows them hostile.
- [x] Options > Nameplates "Show Health Value" (Off/On row like the page's other toggles, off by default) edits `show_health_value`; Done saves it with the client options and the visible plates switch between "77%" and "324 K  77%" without a restart. Tests: `godot/ui-model/tests/options_views.rs` `nameplates_page_offers_show_health_value_off_by_default`, `options_policy.rs` `nameplate_health_value_toggle_defaults_off_and_persists`.
- [ ] Health frame art: the skin's border ring is 4px wide where the 2026-10-04 reference shows a 1px light border with the fill reaching it; the cast fill is the tinted Retail atlas, flatter than the reference's gold.
- [ ] Fills are the reference crops desaturated to their HSV value and tinted, so the default hostile body is (195, 0, 0) where the reference shows (195, 43, 41), and the cast fill loses its white highlights. The pixel-match item above is unaffected in status (still open).

- [ ] Run the focused local-owner, distance, and plate-click tests on the committed implementation.
- [ ] Run an offline `nameplatedebug` rendered smoke/capture for plate visibility, selection, cast/channel progress, and Space pause.
- [ ] `src/rendering/ui/nameplate_gpu_tests.rs` still needs aligned half-size captures for all four thickness combinations. It must mask only glyph regions and prove the specified non-glyph geometry and colors against `data/diagnostics/nameplate-style/reference.png`.
- [ ] Prove server-to-client cast replication over a connected network session.

## Verification status

- [x] 2026-10-05, branch `npfade`, code `cc45ee22`: lock + `agent-run npfade` + local Depot targeted `nameplate` filter passed (Godot 47/47; ui-model `options_policy` 3/3 and `options_views` 1/1; cargo exit 0). Includes `nameplates::tests::nameplate_alpha_distances_preserve_selected_and_occlusion_overrides` at 10/20/30/35.62/40/60 yd. Logs: `/tmp/claude/npfade.out`, `target/depot-test.log`. This proves existing project alpha arithmetic, not Retail interpolation or live rendering. `world_nameplate_options_flow.gd` remains unrun by task instruction.

- [x] 2026-10-04, branch `nameplates`: Godot lib `nameplate` 36/36, core `nameplate` and ui-model `forever_quest_windows`/`options_*` pass (the Modern options fixture was recaptured for the Thick spellbar default). `--screen nameplatedebug` capture `data/diagnostics/nameplates-2026-10-04/debug-modern-2.webp` (4x crops beside it) inspected against the reference: texts inside the bar, cast track on the health frame's width with the icon at its left end.

- [x] 2026-09-28, branch `nameplates` (Godot): core rules 10/10, wire test 1/1, Godot lib layout 3/3 and FactionTemplate 1/1; `tests/nameplate_occlusion.gd` exits 0. `tests/world_nameplate_flow.gd` exits 0 on 127.0.0.1:5000 as Fbworldmap (`data/diagnostics/nameplates-godot-20260928/`): no plates untargeted; three enemy Tab targets (Goblin Assassin, Blackrock Worg) each showed the only plate, friendly ones none; a Blackrock Worg behind a terrain rise read alpha 0.4 with the independent ray blocked; Escape removed the plate. Captures inspected.

- [x] 2026-09-25, branch `nameplates`: targeted bin (153) and lib (76) tests pass. Headless in-world captures (`data/diagnostics/nameplate-20260925/`, `before-*` from master `07dc8db1`, `after-*` from the branch): Defias Thug yellow (was red), Deputy Willem green (was red), Mangy Wolf (FactionTemplate 38) red, names centred above the bar (were offset left); Options > Nameplates edits persisted and applied (neutral green channel 0.5, health width 200, name font 15).
- [ ] No current-cycle test execution or rendered runtime verification is claimed for local-owner hiding, shared distance, plate selection, or the offline debug screen.
- [ ] No pixel-match success claim exists yet. Frame alpha is reconstructed from a composited screenshot and therefore ambiguous; the reference-derived skins and UI-overlay calibration are implementation input, not proof. GPU comparison is pending.

## Out of scope

- Spell damage/healing execution and NPC AI: this work supplies cast presentation lifecycle, not a new combat system. NPC bars require an actual server-side cast source.
- Glow/art refinements, the separate target-first clutter state machine, bottom player casting-bar changes, and Windows work.
