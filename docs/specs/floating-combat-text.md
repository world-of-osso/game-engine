# Floating combat text

Numbers and words that float over a unit as combat happens (Retail "world text"), plus the
screen-anchored self combat text that Retail draws from `Blizzard_CombatText`. Source lives in
`godot/rust/src/combat_text.rs` (world-text styles and mapping),
`godot/rust/src/combat_text/incoming.rs` (self-frame message/layout math),
`godot/rust/src/spells/floating_text.rs` (world and screen labels), and
`godot/core/src/combat_text_options.rs` (persisted Retail CVar defaults). The [source-line parity
audit](../wiki/investigations/floating-combat-text-parity.md) records coverage and native source gaps.
How it is wired is in [spellbook-action-bar](../../wiki/systems/spellbook-action-bar.md).

Every bullet states its proof level:

- **verified 12.1.5** — read out of the decompiled Retail 12.1.5.70688 engine (style array at
  `0x144868b78`, 19 records of 40 bytes: `+0x00` delay ms, `+0x08` lifetime ms,
  `+0x0c`/`+0x10` start/end height, `+0x14` ARGB colour, `+0x18` flags, `+0x1c` scale), relayed
  by the lead on 2026-11-06.
- **verified 15890** — read out of the decompiled Wrath client (`~/Repos/wowdecomp15890`:
  `WorldText.cpp`, `Unit_C.cpp`, `PlayerName.cpp`, `UnitCombatLog_C.cpp`) where Retail 12.1.5
  was not decoded. Wrath is the older baseline, never Retail proof.
- **retail Lua** — read out of `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/...`.
- **retail CVar** — the CVar exists in the cached Retail defaults
  (`/home/osso/Projects/wow/wow-ui-sim/src/cvars.yaml`); its effect is read from its name unless
  a source above says otherwise.
- **calibrated** — a verified shape with an engine constant that no local source fixes.
- **ours** — a current limitation of this engine, not a Retail fact.

## What it must do

### Which text exists

- [x] A unit's damage, healing and avoidance words float over that unit ("world text"), anchored
      at the unit's name position, not at the model origin (**verified 15890**
      `PlayerName.cpp:759-760`).
- [x] Only text caused by the local player, or by a unit the local player owns (pet, guardian,
      player-created game object), is drawn (**verified 15890** `IsOwnedByActivePlayer`,
      `Unit_C.cpp:17681-17706`).
- [x] A unit's own damage taken is not world text; it belongs to the self combat text frame
      (**verified 15890** `AddWorldDamageText` returns when `guid == s_activeMover`,
      `Unit_C.cpp:17744-17746`; **retail Lua** `CombatText.lua` tracks `unit = "player"` and
      draws near the player).
- [x] Damage floats as the plain amount; a crit floats the same amount, taller, with the crit
      pop below (**verified 12.1.5**).
- [x] Healing floats as `+N` in green (**verified 12.1.5** style 13 green; **verified 15890**
      `Unit_C.cpp:21221-21230` for the `+N` text).
- [x] Periodic damage and healing float like direct damage and healing unless
      `floatingCombatTextCombatLogPeriodicSpells_v2` is off (**verified 15890**
      `Unit_C.cpp:17705`; **retail CVar** default `1`). Retail 12.1.5 does not merge repeated
      ticks inside the world-text container (coalescing upstream unproven).
- [x] Pet melee damage floats orange, pet spell damage yellow (**verified 12.1.5** the colours
      survive, gated by `floatingCombatTextPetMeleeDamage_v2` /
      `floatingCombatTextPetSpellDamage_v2`, defaults `1`; **verified 15890**
      `0xFFFF8400` / `0xFFFFDE00`, `Unit_C.cpp:411-417`). Guardian colour is **unknown**; it
      uses the pet colour today.
- [x] Avoidance words float as text, not numbers: Miss, Dodge, Parry, Block, Resist, Immune,
      Deflect, Reflect, Evade, Absorb (**verified 12.1.5** style 9 is shared by every avoidance
      word; **verified 15890** `s_combatTextTokens`, `Unit_C.cpp:406`).
- [x] Overkill and overheal are not shown: the number is the amount dealt or healed
      (**verified 15890** `PlayerNameCreateText(..., "%d", amount)`). `CombatLogEvent.overflow`
      is carried but unused.
- [x] Only the local player's heals float over a healed unit (**verified 15890**
      `Unit_C.cpp:21221`, inside `if (isPlayer)`).
- [x] Incoming damage, healing and avoidance on the local player use a screen-anchored self
      frame, separate from world text (**retail Lua** `CombatText.xml:3-12`, `CombatText.lua:31-40`),
      and default off with `enableFloatingCombatText=0` (**retail CVar** `cvars.yaml:621`). Mapped
      values load from `hud.combatText` in the options file; there is no live CVar registry or
      settings panel.

### Colour

- [x] Player damage, player crits and avoidance words are white; player heals and heal crits are
      green; pet melee orange; pet spell yellow (**verified 12.1.5**: style indices 0-9, 15, 18
      white `0xFFFFFFFF`, 13/14/16 green `0xFF00FF00`, 17 orange `0xFFFF9900`, 10 XP
      `0x8094008B`, 11/12 gold).
- [x] Damage colour does not depend on spell school (**verified 12.1.5**: no per-school colour;
      **verified 15890** the `spellColor` argument only selects the pet colour,
      `Unit_C.cpp:17737-17773`). The engine's former white-physical / yellow-spell split was
      unsourced and is gone.
- [x] The self combat text uses `CombatTextTypeInfo` colors for carried Damage/Heal/Miss kinds:
      physical damage red, spell damage purple, heals green, plus the matching avoidance and
      reduction colors (**retail Lua** `CombatTextConstants.lua:24-73`,
      `Mainline/CombatTextConstantsOverrides.lua:1-7`). Other C_CombatText notifications lack
      protocol event kinds.

### Size, font and ramp

- [x] World text uses `DAMAGE_TEXT_FONT`, `Fonts\FRIZQT__.TTF` for the roman locale, at
      `SystemFont_World`'s height 64 with `filter` (antialiasing) and `fixedSize`, plus a 1,-1
      black shadow (**retail Lua** `Blizzard_Fonts_Shared/Shared/GameFonts.xml:22-23`,
      `Fonts.xml:1060-1113`, `GameFontStyles.xml:147-152`). No combat-text art exists in CASC
      (`data/community-listfile.csv` has no `interface/combattext/*.blp`), so the text is pure
      font rendering.
- [x] Avoidance words are 60 % of a normal hit's height (**verified 12.1.5** style 9
      `+0x1c scale 0.6`).
- [x] A crit changes height only, never the font or outline (**verified 12.1.5** no font or
      outline field; absence-based).
- [x] A crit (and an absorbed amount) plays a three-segment pop: 10 % of its height at spawn,
      200 % at 10 % of the lifetime, 100 % at 20 %, settled after (**verified 12.1.5** styles 2,
      8, 14 and 6; **verified 15890** `s_critHeightCurve`, `WorldText.cpp:37-42,255-286`).
      Every non-crit style keeps its height for its whole lifetime.
- [x] Settled crit height is `WorldTextCritScreenY_v2 / WorldTextScreenY_v2` = 1.833× a normal
      hit (**retail CVar** `0.0275 / 0.015`, the pair Retail ships; 15890's own table gives
      `0.0275 / 0.0183` = 1.5×, the older ratio), then the pop multiplies it.
- [x] Crit amounts have no appended punctuation and use the same amount formatter as hits
      (**verified 15890** `Unit_C.cpp:17775`; the stale `!` live assertion was removed).
- [x] Amounts are whole numbers, never a decimal fraction (`%d`, **verified 15890**
      `Unit_C.cpp:17775`). The "+900.0" seen in the first review was two labels overlapping,
      not a formatter: `text_for` formats `i32` and its test asserts `"+900"`.
- [x] Absolute size comes from Retail's constants, not from this engine's old 174 px em:
      `WorldTextScreenY_v2` (`0.015`) is the normal text height, and 15890 stores the same
      quantity as a style height (`0.018333`, `WorldText.cpp:44-56`) in device-dependent
      coordinates whose `0.6` height scale is one half-viewport (`Coordinate.cpp:19-22`).
      The em is therefore `0.015 / 0.6 = 0.025` of the viewport height: 18 px at 720p (12.6 px
      of digit ink, FRIZQT__ cap height 0.701 em) and 27 px at 1080p. `WorldTextCritScreenY_v2`
      (`0.0275`) gives the crit the same conversion (0.0458 of the viewport, the 1.833x ratio)
      and `WorldTextScale_v2` (`1.0`) multiplies both. A fixed world em makes the screen height
      follow Retail's 1/d law with no per-frame distance maths.
      **Calibrated operand:** Retail 12.1.5 builds that fraction through two calls
      (`0x141e67100`, `0x1401f45a0`) that are still undecoded, so `REFERENCE_YARDS` (10) stands in
      for them; retirement is to replace it with whatever distance those operands resolve to.
      Consequences: the engine no longer draws master's 174 px em and has no far-size floor.
      Measured on the Forever captures (1280x720): a hit draws an 18 px em and a crit 33 px at
      10 yards, and the 1/d law takes them to 6 px and 11 px at 30 yards.

### Distance

- [x] Text scales inversely with camera distance (**verified 12.1.5**): the text is a fixed
      world size, so the projected height is exactly `CVar fraction x viewport height x
      reference / distance`. There is no combat-text size floor: the 12.1.5 draw path does not
      clamp world text. Measured on the Forever `single10`/`single30` captures (1280x720): an
      18 px em at 10 yards draws 6 px at 30 yards, one third, with no lift back up.
- [x] `WorldTextMinSize` is **not** a combat-text setting: Retail registers it as "Minimum
      Character Name Size" (0-64, default `0`) for names, never for world text (**retail Lua**
      `Blizzard_SettingsDefinitions_Frame/Accessibility.lua:12-35`). The former 7 px combat-text
      floor was this setting misapplied and is gone.

### Lifetime, rise and fade

- [x] Damage, heal and avoidance text lives 1500 ms, fades in over the first 150 ms and starts
      fading at 760 ms (damage, heal) or 1000 ms (crits, avoidance) (**verified 15890**
      `s_worldTextTypes[]`, `CalculateNewColor`, `WorldText.cpp:44-56,156-198`; **verified
      12.1.5** lifetime and delay live in each style record).
- [x] Text rises 2.0 world units over its lifetime for damage, heals and avoidance, and does not
      rise for crits (**verified 15890** `m_ascendDistance`, `CalculateAscendOffset`,
      `WorldText.cpp:44-56,118-120`).
- [x] Text never rises above the camera frustum's top plane: it is clamped and offset down by
      its own rise distance at spawn (**verified 15890** `CreateWorldText`,
      `PlayerName.cpp:782-787`). **Ours**: the rise stays inside the frustum because the spawn
      anchor is 3.6 yards above the unit, not because of the clamp.
- [ ] `WorldTextMinAlpha_v2` and `WorldTextGravity_v2` have defaults `0.5` / `0.5`, but their
      native formulas are **not decoded**; the engine still uses its linear rise and fades to zero.

### Placement and stacking

- [x] Engine placement uses a deterministic camera-plane spread and a per-unit vertical ladder;
      the 2026-11-08 Forever fixture measured no overlap among one unit's four slots (0 of 6
      pairs). This proves the engine layout only, not Retail's native random distribution.
- [ ] Retail packs world-text rectangles in screen space (`SmartScreenRectPlace`, BFS
      up/left/right/down, `SmartScreenRect.cpp:432`; crits skip it, `WorldText.cpp:440-441`). The
      engine does not do this; separate units can still overlap (the 2026-11-08 set capture had
      1 overlap among 36 pairs).
- [x] World text carries a black shadow. Label3D outlines do not render for this font (raising
      `outline_size` 8 -> 24 changed the dark-edge pixel count by 1), so the shadow is a black
      copy of the label one tenth of an em down-right, moved and faded with the text
      (**verified 15890** `GxuFontAddShadow` with a 1,-1 shadow; `s_shadowOffset` is 0.002
      against the 0.0183 em, WorldText.cpp:19-29,59,75).
- [x] One unit holds four world texts (search bound 4, array declared as 5): a spell crit
      replaces that unit's existing spell crit, otherwise the first free slot is used, otherwise
      the third slot is evicted, otherwise the text is dropped — in practice the newest text
      always lands (**verified 12.1.5**; **verified 15890** replaced only a crit and dropped the
      rest, `PlayerName.cpp:711-746`).
- [x] Repeated periodic ticks are separate texts, not merged (**verified 12.1.5** no coalescing
      in the world-text container).
- [ ] `floatingCombatTextCombatDamageAllAutos_v2` controls whether every auto-attack draws or
      only a merged swing (**retail CVar** `1`); the merge rule is **not decoded**, so every
      event draws today.
- [ ] `floatingCombatTextCombatDamageDirectionalOffset_v2` / `...DirectionalScale_v2` offset
      damage text toward the hit direction (**retail CVar** `1.0` / `1.0`); not implemented.

### CVars

Retail defaults. A CVar whose effect is read only from its name is marked so.

| CVar | Default | State |
|---|---|---|
| `WorldTextScale_v2` | `1.0` | pure size multiplier; folded into the calibrated anchor |
| `WorldTextScreenY_v2` | `0.015` | normal height: `/ 0.6` DDC gives a 0.025 viewport em |
| `WorldTextCritScreenY_v2` | `0.0275` | crit height |
| `WorldTextRampDuration_v2` / `RampPow_v2` / `RampPowCrit_v2` | `1.0` / `1.9` / `8.0` | **not observed** driving the 12.1.5 pop, which is the fixed three-segment curve; unused |
| `WorldTextStartPosRandomness_v2` | `1.0` | sideways start spread |
| `WorldTextRandomXY_v2` | `0.0` | randomise the start in X and Y |
| `WorldTextRandomZMin_v2` / `Max_v2` | `0.8` / `1.5` | start height band |
| `WorldTextNonRandomZ_v2` | `2.5` | start height when randomness is off |
| `WorldTextGravity_v2` | `0.5` | rise deceleration; formula not decoded |
| `WorldTextMinAlpha_v2` | `0.5` | alpha floor while fading; formula not decoded |
| `WorldTextMinSize` | `0.0` | minimum **character name** size, not combat text (Accessibility.lua:12) |
| `floatingCombatTextCombatDamage_v2` | `1` | world damage text on/off |
| `floatingCombatTextCombatDamageAllAutos_v2` | `1` | every auto-attack vs a merged swing |
| `floatingCombatTextCombatDamageDirectionalOffset_v2` / `...DirectionalScale_v2` | `1.0` / `1.0` | hit-direction offset |
| `floatingCombatTextCombatHealing_v2` | `1` | outgoing heal text |
| `floatingCombatTextCombatHealingAbsorbSelf_v2` / `...Target_v2` | `1` / `1` | self absorb is handled; target path is not mapped |
| `floatingCombatTextCombatLogPeriodicSpells_v2` | `1` | periodic damage/heal text |
| `floatingCombatTextPetMeleeDamage_v2` | `1` | pet melee text |
| `floatingCombatTextPetSpellDamage_v2` | `1` | pet spell text |
| `floatingCombatTextDodgeParryMiss_v2` | `0` | self-frame miss/dodge/parry words (world words are ungated) |
| `floatingCombatTextDamageReduction_v2` | `0` | self-frame block/absorb/resist words |
| `floatingCombatTextFriendlyHealers_v2` | `0` | self-frame heals from other players, `+N [name]` |
| `floatingCombatTextAuras_v2` / `AuraFade_v2` | `0` / `0` | self-frame aura gain/loss |
| `floatingCombatTextCombatState_v2` | `0` | self-frame "Entering/Leaving combat" |
| `floatingCombatTextLowManaHealth_v2` | `1` | self-frame low-health/low-mana |
| `floatingCombatTextReactives_v2` | `1` | self-frame proc-ready `<Spell>` |
| `floatingCombatTextEnergyGains_v2` / `PeriodicEnergyGains_v2` | `0` / `0` | self-frame power gains |
| `floatingCombatTextComboPoints_v2` / `HonorGains_v2` / `RepChanges_v2` | `0` | self-frame combo points / honor / reputation |
| `floatingCombatTextFloatMode_v2` | `1` | self-frame scroll direction (`1` up, `2` down, else fountain) |
| `enableFloatingCombatText` | `0` | loads `Blizzard_CombatText` at all — the **self** frame, not world text (`Settings/CombatOverrides` `CreateFloatingCombatTextSetting`, `Combat.lua:43`) |

Mapped values load from `hud.combatText` in the existing options file; defaults match Retail. This is not a live CVar registry or settings panel, so edits take effect on the next client start.

### Forever vs Modern

- [x] Retail mainline defines behavior; Forever re-skins the same elements. Cached Forever and
      Retail `Blizzard_CombatText` files differ only by `CombatText_UpdateDisplayedMessages`
      (Forever `Shared/CombatText.lua:469-474`); the fonts are identical. World and self text
      share the same skin-neutral paths and do not branch on `LayoutSkin`.
- [ ] A skin difference may only be added by an explicit spec entry (a Forever-only font or
      colour); none exists.

### Performance

- [x] Fifty simultaneous numbers cost no more than 0.1 ms/frame: three runs on the dev library
      over 13 units with their shadows report 24.8, 26.4 and 42.4 µs per frame (three
      2000-iteration samples each, medians, `preview_combat_text_cost`, headless Forever capture,
      2026-11-08); the same fixture cost 38.1 µs before this change. The per-frame distance
      lookup and 7 px clamp are gone. Budget `FRAME_BUDGET_US` = 100.

## Retail source coverage (updated 2026-11-10)

The 2026-11-08 table is superseded by the [source-line coverage matrix](../wiki/investigations/floating-combat-text-parity.md). The current implementation covers outgoing world text and the supported incoming self-frame Damage/Heal/Miss subset. The self frame follows the Retail default-off switch; CVar-named values persist under `hud.combatText` and load on client restart. The world crit punctuation and cross-unit slot eviction errors are fixed. Native world gravity, minimum-alpha, direction, random-Z and upstream auto/multistrike merge formulas remain unproven; world screen-rectangle packing also remains missing.

## How it works

- [spellbook-action-bar](../../wiki/systems/spellbook-action-bar.md) — cast pipeline and the
  `CombatLogEvent` this feature consumes.

## Implementation inventory

- `godot/rust/src/combat_text.rs` — world styles/gates and `CombatLogEvent` mapping.
- `godot/rust/src/combat_text/incoming.rs` — self-frame colors, formatting, scroll, row placement, crit scale and fade math.
- `godot/core/src/combat_text_options.rs` — persisted CVar-named options and Retail defaults.
- `godot/rust/src/spells/floating_text.rs` — world `Label3D`s, self `Label`s, per-frame animation and per-unit slots.
- `godot/rust/src/spells.rs` — the `SpellsHud` fields holding the live texts and the seen-event
  cursor.
- `godot/rust/src/spells/snapshot.rs` — `combat_text` count for automation.
- `godot/rust/src/lib.rs` — `account_state().ui_skin` (skin assertions),
  `preview_combat_text` / `preview_combat_text_cost`.

## Tests asserting this spec

- `godot/rust/src/combat_text.rs` unit tests — per-school-invariance, melee vs spell styles,
  crit pop curve, avoidance word scale, colours, zero amounts, periodic, fade, distance law,
  start spread.
- `godot/rust/src/spells.rs` — settled font sizes (64 / 117 / 48: hit, crit, glancing).
- `godot/rust/src/combat_text/parity_tests.rs` — self message colors/formatting, CVar independence, scroll/fountain, row stack math, crit curve and fade.
- `godot/tests/combat_text_capture.gd` — offline both-skin captures for outgoing styles, per-unit eviction and 20 simultaneous self labels; every scene saves a 960×540 JPEG and a no-text frame. Evidence: `/syncthing/AgentShared/2026-11-06/combat-text/`, 2026-11-06:

  | scene | boxes intersecting, master | ours | text pixels (white/green/orange/yellow), ours |
  |---|---|---|---|
  | `set`, 9 texts over 3 units | 36 of 36 | 0 of 36 | 397 / 278 / 44 / 46 |
  | `limit`, 4 slots on 1 unit | 15 of 15 | 0 of 6 | 65 / 0 / 43 / 4 |
  | `spread`, 50 over 13 units | 640 of 1225 | 0 of 1225 | 950 / 647 / 5 / 0 |
  | `single10`, 3 texts at 10 yd | 0 of 3 (fixed-size boxes modelled with the old factor) | 0 of 3 | 319 / 62 / 7 / 2 |
  | `single30`, 3 texts at 30 yd | 0 of 3 | 0 of 3 | 11 / 6 / 21 / 2 |

  Master drew 13 800 yellow and 899 white pixels in `set` (school split, no heals or words) and
  36 928 yellow in `spread`, because its labels kept the old size.

  2026-11-08 re-run after the size/distance change (headless cage, dev library, same fixture,
  same scenes; files `sizebase-*` and `sizeafter-*` in
  `data/diagnostics/combat-text-parity-2026-11-08/`):

  | scene | before (master's 10.8 px em) | after (Retail 18 px em) |
  |---|---|---|
  | `spread`, 50 over 13 units | 0 of 1225 boxes intersect; 5344 text pixels; 38.1 µs/frame | 0 of 1225; 12 571 text pixels; 24.8-42.4 µs/frame (three runs) |
  | `set`, 9 over 3 units | 0 of 36 (2026-11-06 capture) | 1 of 36 (two different units, 1.4 yd apart; screen-space packing remains open) |
  | `single10` | hit em 10.7 px | hit em 17.9 px, crit 33.3 px |
  | `single30` | hit clamped to the old 7 px floor | hit em 6.0 px, crit 11.0 px, exactly one third of 10 yd |
- `godot/tests/melee_outcomes_live.gd` — live crit text is 1.833× the plain size (Forever).

## Known gaps (current cycle)

- [ ] Self-frame event kinds not present in the protocol: energize/power, aura gain/loss,
      combat state, low health/mana, honor, reputation, XP, threat, combo points, runes and
      reactive spells. The frame switch and carried Damage/Heal/Miss messages are implemented.
- [ ] Screen-space packing across world units, Retail's native random-Z distribution, and
      `floatingCombatTextCombatDamageDirectionalOffset_v2` hit-direction offset.
- [ ] `WorldTextGravity_v2`, `WorldTextMinAlpha_v2` and the decoded operands of the distance
      constant.
- [ ] Guardian damage colour (unknown; pet colours apply).
- [ ] The persisted combat-text options have no settings panel/live CVar callback; file edits load on client restart.

## Out of scope

- Pet-battle floating text (`enablePetBattleFloatingCombatText_v2`), which belongs to the
  pet-battle HUD.
- World-object building damage text (15890 types 8/9 style slots), which needs destructible
  buildings.
- Native formulas that are not specified by the cached Lua/CVar sources; see the source-blocked
  list above.
