# PLAN.md

## Goal
- [ ] Complete full Godot conversion: feature, visual/input, gameplay, performance and automation parity, tracked in `docs/specs/godot-parity-matrix.md`.

## Context
- Master uses Bevy ui-toolkit's `native_render` (nine-slice button textures, focus colors)
- Extracted master's character-creation state into a shared module; both Bevy and Godot now use it
- Restarted dev server (:5000) to match current shared-protocol
- Fixed master's `--screen login screenshot` hang in `src/app_runtime.rs`

## Completed
- **Login**: buttons match master (12px corners, hover/press skins), Enter submits, Tab/Escape/focus colors work, dev prefill/auto-focus
- **Character select**: list panel backdrop, Create New Character → creation flow, delete with 3s countdown and confirmation, Up/Down/Enter/Escape keys
- **Character create**: shared reducer in `src/game/character_creation/logic.rs`, icon masks (round portraits), error handling
- **Polish**: 0.75s fade-in, vsync disabled
- **Tests**: UI fixtures pass (login, character select, character create actions)

## Textfit: three UI defects

Scope: spellbook name fitting, character side-tab fitting, character portrait binding. Preserve approved art; no other UI changes, fixture hand edits, server changes, merge or push. Completion: concrete registry regressions pass for Modern and Forever through local build-lock + agent-run; changed Rust formatted and committed on textfit.

- [x] Fit long spell names on one line above subtext/level: measure the name against its scaled fixed bounds; retain full text and subtext/level positions.
- [x] Fit Character/Reputation captions inside side tabs: replace the Forever 55px multiline caption box with a centered 13px line, using existing native ellipsis behavior.
- [x] Bind the player portrait inside the character window ring: add the shared masked portrait host and connect it to the existing player appearance renderer.

Proof at `2baf7116` (documentation-only follow-up does not invalidate it):
- Local detached build-lock + agent-run helper, `--test -p game-engine-godot -p game-engine-ui-model --no-fail-fast textfit -- --nocapture --test-threads=1`: 3/3 pass, exit 0. RED logs reproduce missing portrait host, 221px spell name in 166.7px bounds, and Forever's 55px caption for a 13px line. Both skins; spellbook at 1920×1080 and 1280×720; both character tabs.
- Same helper, `--test -p game-engine-ui-model --test forever_character_frame --test spellbook_frame -- --nocapture --test-threads=1`: 12/12 pass, exit 0. Recorded fixture unchanged; frozen chrome comparison excludes the new live portrait host, whose geometry has its own registry regression.
- Changed-file cargo fmt completed; changed-file rustfmt check passed through build-lock + agent-run. Formatter-only changes to unrelated tests reverted. No warnings in targeted logs.
- Logs: `/tmp/claude/textfit-{red2,tab-red,green,frames,fmt,fmt-check}.out`; complete regression output in `/tmp/claude/textfit-green-test.log`.

Limits: no local Blizzard SpellBook XML/Lua found in this worktree or `~/Repos`; existing layout/template references retained. Registry and native compilation prove binding geometry/wiring, not live portrait pixels; GUI proof remains blocked by the documented host compositor failure. Pre-existing formatting drift in mirror_timer, personal_resource_display, target_cast_bar and unit_frame_atlas tests left unchanged.

## In Progress
- [x] Reconcile bounded startup CLI proof: six destinations remain proven; post-`f015f651` native fmt passes, while the `bd8282c7` terrain-object fmt failure remains historical. Loading clipping follows shared legacy layout and is not a confirmed Godot regression.
- [x] Prove selected-roster gear replacement/removal and one cached bound chest's rendered pose response.
- [x] Complete bounded native fmt/check and independent equipment verification (`9059d471`); two existing WMO warnings remain.
- [x] Reproduce and implement local/remote InWorld body/gear with stable unit identity and paused-pose continuity; grounded female probe `6acd4ec0` independently audited.
- [ ] Close repaired NPC regression audit (`ca0c9204` runtime exit 0; dummy-renderer diagnostic remains).
- [x] Prove local grounded Walk (4), Backward (13), Left (11), and Right (12) through real root-launched key/bone/decoded-UDP input: each changes bones, returns Stand, and becomes quiet (`0fcb9578`).
- [ ] Continue original jump/swim/turn animation behavior; remote player Stand matches original renderer and needs no invented wire fields.
- [ ] Resolve real-server protocol compatibility before refreshing the visible trial.

## Blockers & Next Steps
1. **Character select nav bar** (MODE/SHOP/MENU/REALMS/CAMPSITES): blocked on 3D campsite data; other session owns
2. **Game menu screen**: not yet wired
3. **Bevy tests**: `transfer.rs` error from other session prevents root test run; not a blocker to merge
4. **Visual verify Customize mode**: need rebuilt master binary

**Parity matrix updated**: `docs/specs/godot-parity-matrix.md`

## Authored selection background
- [x] Launch saved-option FPS counter/graph; current visible fixture exits 0, historical timeout remains unexplained.
- [x] Share authored Warband records, solo camera, and explicit terrain tile requests.
- [x] Render both authored textured terrain tiles with selected body; GPU/input clean exit at `df22179c`.
- [x] Preserve original first-four-layer terrain rendering; retain fifth layers in parsed data.
- [x] Share/test campsite inclusion and MDDF/MODF transforms against local fixtures.
- [x] Wire 76 primary and 42 supplemental doodads plus the nearby WMO.
- [x] Finish original sky materials/animation integration and isolated GPU proof.
- [x] Inspect combined scenery, body, sky; prove scene cleanup and world/input transition at `0266003e`.
- [x] Refresh affected trial client and obtain final independent scoped verification (535/539).
- [x] Reconcile parity matrix; full conversion/equipment/other campsite gaps remain open.

## Full conversion: selected-roster equipment
- [x] Reproduce missing equipped-item appearance with concrete roster data (`e909b136` RED).
- [x] Share original item/display resolution and clothing/geoset decisions without duplicating policies.
- [x] Apply equipment textures and geosets to selected player customization; starter render recorded at `640e9f30`.
- [x] Attach authored equipment models; HD/boar attachment transforms and one collection chest weighted-pose response verified.
- [x] Inspect starter/chest rendering and selection replacement/cleanup; independently verify bounded evidence.
- [ ] Keep uncached FDID2368173 extraction, broader slots/races and authored animation-sequence coverage explicit in matrix.
- [ ] Continue remaining feature, visual/input, gameplay, performance and automation parity scopes from the matrix.
- [ ] Refactor `godot/rust/src/char_create/scene.rs`: load_backdrop (line 189): 35 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/terrain/material.rs`: build_tile (line 91): 31 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/terrain/assets.rs`: read_tile (line 86): 37 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui_map_data.rs`: read_arts (line 374): 33 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/world_map_frame_component.rs`: border (line 270): 78 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/world_map_frame_component.rs`: breadcrumb (line 415): 31 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/world_map_frame_component.rs`: canvas (line 458): 48 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/world_map_frame_component_tests.rs`: sample_state (line 8): 34 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/wmo/global.rs`: sync (line 31): 41 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/world_map.rs`: world_map_state (line 282): 43 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/wmo/portals.rs`: new (line 49): 55 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/wmo/portals.rs`: new (line 49): nesting depth 5 (max 4) — extract into helper functions
- [ ] Refactor `src/asset/m2_format/m2_variation.rs`: read (line 24): 32 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/targeting.rs`: target_state (line 395): 34 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/rendering/skybox/sky_lightdata_data.rs`: lerp_color_sets (line 197): 36 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/spellbook_frame_component.rs`: paginate (line 204): 40 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/spellbook_frame_component.rs`: category_tab (line 387): 49 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/spellbook_frame_component.rs`: header (line 453): 35 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/spellbook_frame_component.rs`: item_texts (line 510): 31 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/spellbook_frame_component.rs`: item (line 547): 60 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/spellbook_frame_component.rs`: paging (line 651): 54 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/spellbook_frame_component.rs`: File is 783 lines (max 750). Consider splitting it. — extract into helper functions
- [ ] Refactor `godot/rust/src/spells.rs`: sync_cast_bar (line 438): 33 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/spells.rs`: float_combat_text (line 587): 54 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/spells.rs`: spells_snapshot (line 654): 67 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/spells.rs`: File is 776 lines (max 750). Consider splitting it. — extract into helper functions
- [ ] Refactor `src/ui/screens/spell_tooltip_component.rs`: spell_tooltip_screen (line 76): 50 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/spell_tooltip.rs`: strip_color_escapes (line 140): nesting depth 5 (max 4) — extract into helper functions
- [ ] Refactor `godot/rust/src/spell_tooltip.rs`: spell_tooltip_state (line 170): 46 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/core/src/spell_visual.rs`: read_kits (line 457): 60 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/core/src/spell_visual.rs`: read_conditions (line 579): 75 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/core/src/spell_visual.rs`: File is 756 lines (max 750). Consider splitting it. — extract into helper functions
- [ ] Refactor `godot/rust/src/spell_effects.rs`: sync_casts (line 201): 37 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/spell_effects.rs`: spell_go (line 275): 31 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/spell_effects.rs`: start_kits (line 372): 41 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/spell_effects.rs`: start_kits (line 372): nesting depth 6 (max 4) — extract into helper functions
- [ ] Refactor `godot/rust/src/spell_effects.rs`: advance_missiles (line 655): 39 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/spell_effects.rs`: File is 774 lines (max 750). Consider splitting it. — extract into helper functions
- [ ] Refactor `godot/rust/src/spells.rs`: action_bar_state (line 473): 31 body lines (max 30) — extract into helper functions
- [ ] Refactor `src/ui/screens/inworld_unit_frames_aura.rs`: aura_button (line 166): 45 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/auras.rs`: auras_snapshot (line 239): 43 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/auras.rs`: sync_swipe (line 334): 36 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/pet_bar.rs`: pet_bar_state (line 222): 39 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/pet_bar.rs`: pet_bar_snapshot (line 297): 56 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/spell_effects.rs`: start_kit_on (line 808): 47 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/spell_effects.rs`: launch (line 973): 42 body lines (max 30) — extract into helper functions
- [ ] Refactor `godot/rust/src/spell_effects.rs`: spawn_kit_model (line 1160): 35 body lines (max 30) — extract into helper functions
