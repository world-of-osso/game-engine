# Floating combat text

Numbers and words that float over a unit as combat happens (Retail "world text"), plus the
screen-anchored self combat text that Retail draws from `Blizzard_CombatText`. Source lives in
`godot/rust/src/combat_text.rs` (Retail CVar defaults, style table, distance law, start spread,
size ramp, event mapping) and `godot/rust/src/spells/floating_text.rs` (the `Label3D` nodes).
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
- [x] Incoming healing and damage on the local player belong to the screen-anchored self
      combat text, not world text (**retail Lua** `CombatText.lua`, `CombatTextConstants.lua`),
      and that frame is off by default: `enableFloatingCombatText` is `0` and is what loads
      `Blizzard_CombatText` (**retail Lua** `Combat/Overrides` `CreateFloatingCombatTextSetting`,
      `Combat.lua:43`; **retail CVar** `0`). The engine has no self frame **and** no CVar store,
      so it matches Retail's default state; the frame becomes necessary only together with that
      switch (Known gaps).

### Colour

- [x] Player damage, player crits and avoidance words are white; player heals and heal crits are
      green; pet melee orange; pet spell yellow (**verified 12.1.5**: style indices 0-9, 15, 18
      white `0xFFFFFFFF`, 13/14/16 green `0xFF00FF00`, 17 orange `0xFFFF9900`, 10 XP
      `0x8094008B`, 11/12 gold).
- [x] Damage colour does not depend on spell school (**verified 12.1.5**: no per-school colour;
      **verified 15890** the `spellColor` argument only selects the pet colour,
      `Unit_C.cpp:17737-17773`). The engine's former white-physical / yellow-spell split was
      unsourced and is gone.
- [ ] The self combat text colours each message type from `CombatTextTypeInfo` (physical damage
      and misses red, spell damage purple `0.79, 0.3, 0.85`, heals green, energize blue, aura
      gain green / loss red, ...) (**retail Lua**
      `Blizzard_CombatText/Shared/CombatTextConstants.lua:24-73` and
      `Mainline/CombatTextConstantsOverrides.lua`). Not implemented.

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
- [x] Settled crit height is 1.5× a normal hit, then the pop multiplies it (**verified 15890**
      `0.0275 / 0.0183`; **retail CVar** `WorldTextCritScreenY_v2` / `WorldTextScreenY_v2` would
      give 1.83×, interpretation unproven).
- [x] The engine appends "!" to crit amounts (**ours**; not present in 15890
      `Unit_C.cpp:17775`; kept because `godot/tests/melee_outcomes_live.gd` and
      `docs/specs/spellbook-action-bar.md` assert it).

### Distance

- [x] Text scales inversely with camera distance (**verified 12.1.5**): nearer targets draw
      larger, farther smaller, with a far floor (**verified 12.1.5** a constant 7 px draw
      clamp). The scale constant's two call operands (`0x141e67100`, `0x1401f45a0`) are not
      decoded, so the anchor is **calibrated**: a target 10 yards away keeps the size this
      engine drew before the law existed, and the far floor is 20 % of it
      (`combat_text.rs` `DISTANCE_ANCHOR_YARDS`, `FAR_SIZE_FLOOR`). Retirement: replace both
      with the decoded operands.
- [x] `WorldTextMinSize` is a floor that only applies beyond 1 yard (**verified 12.1.5**, exact
      formula unproven); the default `0.0` means no extra floor, so the engine's far floor
      stands alone.

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
- [x] `WorldTextMinAlpha_v2` floors the alpha while fading and `WorldTextGravity_v2`
      decelerates the rise (**retail CVar** `0.5` / `0.5`); their formulas are **not decoded**,
      so the engine keeps 15890's linear rise and fade to zero.

### Placement and stacking

- [x] Simultaneous numbers from one unit start at different points (**verified 15890** spawn Z
      is offset by the type's rise, `PlayerName.cpp:786`; **retail CVar**
      `WorldTextStartPosRandomness_v2` `1.0`, `WorldTextRandomZMin_v2` `0.8`,
      `WorldTextRandomZMax_v2` `1.5`, `WorldTextNonRandomZ_v2` `2.5`, `WorldTextRandomXY_v2`
      `0.0`).
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
| `WorldTextScreenY_v2` | `0.015` | normal height; source of the 1.5× crit ratio comparison only |
| `WorldTextCritScreenY_v2` | `0.0275` | crit height |
| `WorldTextRampDuration_v2` / `RampPow_v2` / `RampPowCrit_v2` | `1.0` / `1.9` / `8.0` | **not observed** driving the 12.1.5 pop, which is the fixed three-segment curve; unused |
| `WorldTextStartPosRandomness_v2` | `1.0` | sideways start spread |
| `WorldTextRandomXY_v2` | `0.0` | randomise the start in X and Y |
| `WorldTextRandomZMin_v2` / `Max_v2` | `0.8` / `1.5` | start height band |
| `WorldTextNonRandomZ_v2` | `2.5` | start height when randomness is off |
| `WorldTextGravity_v2` | `0.5` | rise deceleration; formula not decoded |
| `WorldTextMinAlpha_v2` | `0.5` | alpha floor while fading; formula not decoded |
| `WorldTextMinSize` | `0.0` | height floor beyond 1 yard; formula not decoded |
| `floatingCombatTextCombatDamage_v2` | `1` | world damage text on/off |
| `floatingCombatTextCombatDamageAllAutos_v2` | `1` | every auto-attack vs a merged swing |
| `floatingCombatTextCombatDamageDirectionalOffset_v2` / `...DirectionalScale_v2` | `1.0` / `1.0` | hit-direction offset |
| `floatingCombatTextCombatHealing_v2` | `1` | outgoing heal text |
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

The engine has no CVar store; the defaults are compile-time constants.

### Forever vs Modern

- [x] Retail mainline defines the behaviour; the Forever skin re-skins the same elements. The
      Forever UI source for `Blizzard_CombatText`, `SystemFont_World` and `DAMAGE_TEXT_FONT` is
      byte-identical to Retail
      (`diff -r blizzard-ui/wowforever/AddOns/Blizzard_CombatText blizzard-ui/retail/AddOns/Blizzard_CombatText`
      is empty), so no Forever/Modern difference is specified and world text does not branch on
      `LayoutSkin`.
- [ ] A skin difference may only be added by an explicit spec entry (a Forever-only font or
      colour); none exists.

### Performance

- [x] Fifty simultaneous numbers cost no more than 0.1 ms/frame in the release library:
      18.8-19.0 µs per frame over 13 units, three 2000-iteration samples
      (`preview_combat_text_cost`, release `libgame_engine_godot.so`, 2026-11-06).

## How it works

- [spellbook-action-bar](../../wiki/systems/spellbook-action-bar.md) — cast pipeline and the
  `CombatLogEvent` this feature consumes.

## Implementation inventory

- `godot/rust/src/combat_text.rs` — Retail style table (colours, lifetimes, rises, scale),
  distance law, start spread, crit pop, fade, `CombatLogEvent` → text mapping.
- `godot/rust/src/spells/floating_text.rs` — `Label3D` spawn, per-frame advance (rise, pop,
  distance, fade), the per-unit four-slot stack, the offline preview hooks.
- `godot/rust/src/spells.rs` — the `SpellsHud` fields holding the live texts and the seen-event
  cursor.
- `godot/rust/src/spells/snapshot.rs` — `combat_text` count for automation.
- `godot/rust/src/lib.rs` — `account_state().ui_skin` (skin assertions),
  `preview_combat_text` / `preview_combat_text_cost`.

## Tests asserting this spec

- `godot/rust/src/combat_text.rs` unit tests — per-school-invariance, melee vs spell styles,
  crit pop curve, avoidance word scale, colours, zero amounts, periodic, fade, distance law,
  start spread.
- `godot/rust/src/spells.rs` — settled sizes (64 / 96 / 48).
- `godot/tests/combat_text_capture.gd` — Forever capture fixture, offline in Stormwind:
  `set` (nine texts over three units, every style), `limit` (four slots, crit replacement,
  fifth-text eviction), `spread` (fifty over thirteen units) and `perf` (the frame cost). Each
  capture saves a 960×540 JPEG plus the same frame without text. Evidence:
  `/syncthing/AgentShared/2026-11-06/combat-text/{set,limit,spread}.jpg` with
  `-notext.jpg` partners; 2026-11-06 the `set` frame drew 12 872 white, 12 979 green,
  436 orange and 370 yellow text pixels, `limit` 2 012 / 926 / 1 169 / 39, `spread`
  12 118 / 4 551 / 22 313 / 90.
- `godot/tests/melee_outcomes_live.gd` — live crit text is 1.5× the plain size (Forever).

## Known gaps (current cycle)

- [ ] The self combat text frame (`Blizzard_CombatText`) and its `enableFloatingCombatText`
      switch: incoming damage/healing on the player, auras, honor, reputation, XP, threat,
      combo points and PvP rank icons. Retail ships it off, so this waits for the CVar store
      rather than for the frame alone.
- [ ] `floatingCombatTextCombatDamageDirectionalOffset_v2` hit-direction offset.
- [ ] `WorldTextGravity_v2`, `WorldTextMinAlpha_v2` and `WorldTextMinSize` formulas, and the
      decoded operands of the distance constant.
- [ ] Guardian damage colour (unknown; pet colours apply).
- [ ] CVar gating: the defaults are compiled in, with no runtime switch.

## Out of scope

- Pet-battle floating text (`enablePetBattleFloatingCombatText_v2`), which belongs to the
  pet-battle HUD.
- World-object building damage text (15890 types 8/9 style slots), which needs destructible
  buildings.
- The self frame's non-combat messages (honor, reputation, XP, threat, combo points, auras, PvP
  rank icons) until `C_CombatText` engine events exist.
