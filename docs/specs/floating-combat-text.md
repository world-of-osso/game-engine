# Floating combat text

Numbers and words that float over a unit as combat happens (Retail "world text"), plus the
screen-anchored self combat text that Retail draws from `Blizzard_CombatText`. Source lives in
`godot/rust/src/combat_text.rs` (Retail CVar defaults, start spread, size ramp, event mapping)
and `godot/rust/src/spells/floating_text.rs` (the `Label3D` nodes). How it is wired is in
[spellbook-action-bar](../../wiki/systems/spellbook-action-bar.md).

Every bullet below states its proof level:

- **verified 15890** — read out of the decompiled Wrath client (`~/Repos/wowdecomp15890`:
  `WorldText.cpp`, `Unit_C.cpp`, `PlayerName.cpp`, `UnitCombatLog_C.cpp`). Wrath is the newest
  decompiled engine locally, so this is the baseline, not proof of Retail 12.x.
- **retail Lua** — read out of `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/...`.
- **retail CVar** — the CVar exists in the cached Retail defaults
  (`/home/osso/Projects/wow/wow-ui-sim/src/cvars.yaml`) and its effect is read from its name
  unless a source above says otherwise.
- **pending retail decomp** — needs the Retail 12.x engine (relayed through the lead).
- **ours** — a current limitation of this engine, not a Retail fact.

## What it must do

### Which text exists

- [ ] A unit's damage, healing and avoidance words float over that unit ("world text"). Anchored
      at the unit's name position, not at the model origin (**verified 15890**
      `PlayerName.cpp:759-760`).
- [ ] Only text caused by the local player, or by a unit the local player owns (pet, guardian,
      player-created game object), is drawn (**verified 15890** `IsOwnedByActivePlayer`,
      `Unit_C.cpp:17681-17706`).
- [ ] A unit's own damage taken is *not* world text; it goes to the self combat text frame
      (**verified 15890** `AddWorldDamageText` returns when `guid == s_activeMover`,
      `Unit_C.cpp:17744-17746`; Retail `Blizzard_CombatText/Shared/CombatText.lua:5-6` tracks
      `unit = "player"` and draws near the player).
- [ ] Damage floats as the plain amount, crits as the same amount with a larger, ramping height
      (**verified 15890** `Unit_C.cpp:17775`; ours appends "!" — see the crit bullet below).
- [ ] Healing floats as `+N`, green (**verified 15890** `Unit_C.cpp:21221-21230`).
- [ ] Periodic damage and healing float like direct damage and healing unless
      `floatingCombatTextCombatLogPeriodicSpells_v2` is off (**verified 15890**
      `Unit_C.cpp:17705`; **retail CVar** default `1`).
- [ ] Pet melee damage floats orange, pet spell damage yellow (**verified 15890**
      `s_petMeleeDamageColor = 0xFFFF8400`, `s_petSpellDamageColor = 0xFFFFDE00`,
      `Unit_C.cpp:411-417`), each gated by `floatingCombatTextPetMeleeDamage_v2` /
      `floatingCombatTextPetSpellDamage_v2` (**verified 15890** `Unit_C.cpp:17708-17735`;
      **retail CVar** default `1`).
- [ ] Avoidance words float as text, not numbers: Miss, Dodge, Parry, Block, Resist, Immune,
      Deflect, Reflect, Evade, Absorb (**verified 15890** `s_combatTextTokens`,
      `Unit_C.cpp:406`, and `UnitCombat_C.cpp:505-545`).
- [ ] Overkill is not shown: the damage number is the damage dealt (**verified 15890**
      `PlayerNameCreateText(..., text = "%d", amount)`); overheal is not shown either (**ours**:
      `CombatLogEvent.overflow` is carried but not drawn — see Known gaps).
- [ ] Outgoing healing floats over the healed unit only for the local player's own spells
      (**verified 15890** `Unit_C.cpp:21221`, inside `if (isPlayer)`).
- [ ] Incoming healing/damage on the local player is drawn by the screen-anchored self combat
      text, not world text (**retail Lua** `CombatText.lua`, `CombatTextConstants.lua`; **ours**:
      not implemented — see Known gaps).

### Colour

- [ ] Player damage and player crits are white; player heals and heal crits are green; avoidance
      words are white; pet melee is orange; pet spell is yellow (**verified 15890**
      `s_worldTextTypes[]`, `WorldText.cpp:44-56`).
- [ ] Damage colour does not depend on spell school in 15890: the `spellColor` argument only
      selects the melee/spell pet colour, never a school colour (**verified 15890**
      `Unit_C.cpp:17737-17773`). Retail 12.x per-school colours: **pending retail decomp**. Ours
      currently splits white physical / yellow spell (unsourced; see Known gaps).
- [ ] The self combat text colours each message type from `CombatTextTypeInfo` (physical damage
      and misses red, spell damage purple `0.79, 0.3, 0.85`, heals green, energize blue, aura
      gain green / loss red, ...) (**retail Lua**
      `Blizzard_CombatText/Shared/CombatTextConstants.lua:24-73` and
      `Mainline/CombatTextConstantsOverrides.lua`).

### Size, font and ramp

- [ ] World text uses `DAMAGE_TEXT_FONT`, which is `Fonts\FRIZQT__.TTF` for the roman locale,
      at `SystemFont_World`'s height 64 with `filter` (antialiasing) and `fixedSize`, plus a
      1,-1 black shadow (**retail Lua** `Blizzard_Fonts_Shared/Shared/GameFonts.xml:22-23`,
      `Fonts.xml:1060-1113`, `GameFontStyles.xml:147-152`). No locale-independent alternative
      exists: the CASC file list has no `interface/combattext/*.blp` or other combat-text art
      (`data/community-listfile.csv`), so the text is pure font rendering.
- [ ] Crit text uses the same font as a normal hit; only the height differs (**verified 15890**
      `RecreateString` takes the height, never a second font; **retail Lua** only the height
      CVar `WorldTextCritScreenY_v2` exists). A separate crit font is **pending retail decomp**.
- [ ] Normal text height is `WorldTextScreenY_v2`, crit height `WorldTextCritScreenY_v2`, both
      scaled by `WorldTextScale_v2` (**retail CVar** `0.015`, `0.0275`, `1.0`; the same numbers
      appear as 15890's `startHeight`/`endHeight` normal `0.0183` and crit `0.0275` — **verified
      15890** `WorldText.cpp:44-56`, so the CVar is that table's field).
- [ ] Text pops larger at spawn and settles over `WorldTextRampDuration_v2`, shaped by
      `WorldTextRampPow_v2` for hits and `WorldTextRampPowCrit_v2` for crits (**retail CVar**
      `1.0`, `1.9`, `8.0`); 15890 instead uses a three-segment linear curve over the same
      duration, peaking at 2.0× the settled height at 10 % of the lifetime and back to 1.0× at
      20 % (**verified 15890** `s_critHeightCurve`, `WorldText.cpp:37-42,255-286`).
- [ ] A crit's settled height is 1.5× a normal hit in 15890 (`0.0275 / 0.0183`); the Retail CVar
      defaults give 1.83× (`0.0275 / 0.015`) (**both computed**; ours uses 1.5× — see Known gaps).
- [ ] Ours appends "!" to crit amounts (**ours**; not present in 15890
      `Unit_C.cpp:17775`; retained because `godot/tests/melee_outcomes_live.gd` and
      `docs/specs/spellbook-action-bar.md` assert it).

### Lifetime, rise and fade

- [ ] Damage, heal and avoidance text lasts 1500 ms, fades in over the first 150 ms, starts
      fading at 760 ms (damage/heal) or 1000 ms (crits, avoidance, absorb) (**verified 15890**
      `s_worldTextTypes[]`, `CalculateNewColor`, `WorldText.cpp:44-56,156-198`).
- [ ] Text rises in world Z over its lifetime: 2.0 units for damage, heals and avoidance, 0.0
      for crits (**verified 15890** `m_ascendDistance`, `CalculateAscendOffset`,
      `WorldText.cpp:44-56,118-120`).
- [ ] The text never rises above the camera frustum's top plane: it is clamped and offset down
      by the ascend distance at spawn (**verified 15890** `CreateWorldText`,
      `PlayerName.cpp:782-787`).
- [ ] `WorldTextMinAlpha_v2` keeps a floor while fading (**retail CVar** `0.5`, default
      behaviour **pending retail decomp**; 15890 fades to 0).
- [ ] `WorldTextGravity_v2` decelerates the rise (**retail CVar** `0.5`, formula **pending
      retail decomp**; 15890 rises linearly).

### Placement and stacking

- [ ] Simultaneous numbers from one unit start at different points so they do not stack
      (**verified 15890** spawn Z is offset by the type's ascend distance,
      `PlayerName.cpp:786`; **retail CVar** `WorldTextStartPosRandomness_v2` `1.0`,
      `WorldTextRandomZMin_v2` `0.8`, `WorldTextRandomZMax_v2` `1.5`,
      `WorldTextNonRandomZ_v2` `2.5`, `WorldTextRandomXY_v2` `0.0`).
- [ ] One unit holds at most 4 world texts; a new crit replaces that unit's existing crit slot,
      otherwise the first free slot is used, otherwise the new text is dropped
      (**verified 15890** `CreateWorldText`, `PlayerName.cpp:711-746`).
- [ ] Repeated periodic ticks are *not* merged into one number; they are separate texts subject
      to the 4-slot limit (**verified 15890**; Retail merge rule **pending retail decomp**).
- [ ] Ours spreads numbers with an R2 low-discrepancy sequence in the camera plane and keeps no
      per-unit limit (**ours**; see Known gaps).

### Distance

- [ ] 15890 draws world text in screen space at a fixed NDC height, so it does not shrink with
      distance (**verified 15890** `SetupOrthoProjection` + `DDCToNDCWidth(m_fontHeight)`,
      `WorldText.cpp:234-253,128-154`). Retail 12.x distance scaling — "retail world text shrinks
      with distance" is the current briefing claim — is **pending retail decomp**.
- [ ] Ours keeps a fixed on-screen size (Godot `FIXED_SIZE` draw flag) and measures the start
      spread in world units that scale with camera distance, so the spread is constant in
      screen space (**ours**, follows 15890).

### CVars

Retail defaults, from the cached Retail CVar list. A CVar whose effect is read only from its
name is marked so.

| CVar | Default | Effect |
|---|---|---|
| `WorldTextScale_v2` | `1.0` | global world-text size multiplier |
| `WorldTextScreenY_v2` | `0.015` | normal text height (fraction of screen width) |
| `WorldTextCritScreenY_v2` | `0.0275` | crit text height |
| `WorldTextRampDuration_v2` | `1.0` | seconds of the pop-and-settle ramp |
| `WorldTextRampPow_v2` | `1.9` | hit ramp exponent |
| `WorldTextRampPowCrit_v2` | `8.0` | crit ramp exponent |
| `WorldTextStartPosRandomness_v2` | `1.0` | sideways start spread |
| `WorldTextRandomXY_v2` | `0.0` | randomise the start in X and Y |
| `WorldTextRandomZMin_v2` / `Max_v2` | `0.8` / `1.5` | start height band |
| `WorldTextNonRandomZ_v2` | `2.5` | start height when randomness is off |
| `WorldTextGravity_v2` | `0.5` | rise deceleration |
| `WorldTextMinAlpha_v2` | `0.5` | alpha floor while fading |
| `WorldTextMinSize` | `0.0` | minimum text height |
| `floatingCombatTextCombatDamage_v2` | `1` | world damage text on/off |
| `floatingCombatTextCombatDamageAllAutos_v2` | `1` | draw every auto-attack, not only merged ones (merge rule **pending retail decomp**) |
| `floatingCombatTextCombatDamageDirectionalOffset_v2` / `...DirectionalScale_v2` | `1.0` / `1.0` | offset damage text toward the hit direction |
| `floatingCombatTextCombatHealing_v2` | `1` | outgoing heal text |
| `floatingCombatTextCombatLogPeriodicSpells_v2` | `1` | periodic damage/heal text |
| `floatingCombatTextPetMeleeDamage_v2` | `1` | pet melee text |
| `floatingCombatTextPetSpellDamage_v2` | `1` | pet spell text |
| `floatingCombatTextDodgeParryMiss_v2` | `0` | self-frame miss/dodge/parry words (world words are ungated in 15890) |
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

### Forever vs Modern

- [ ] Retail mainline defines the behaviour; the Forever skin re-skins the same elements. The
      Forever UI source for `Blizzard_CombatText`, `Blizzard_Fonts_Shared` `SystemFont_World` and
      `GameFonts.xml` `DAMAGE_TEXT_FONT` is byte-identical to Retail
      (`diff -r blizzard-ui/wowforever/AddOns/Blizzard_CombatText blizzard-ui/retail/AddOns/Blizzard_CombatText`
      is empty), so **no Forever/Modern difference is specified** and world text must not branch
      on `LayoutSkin`.
- [ ] A skin difference may only be added by an explicit spec entry (e.g. a Forever-only font or
      colour); none exists today.

### Performance

- [ ] Fifty simultaneous numbers cost no more than 0.1 ms/frame of CPU in the release library
      (**ours**; measured — see Tests).

## How it works

- [spellbook-action-bar](../../wiki/systems/spellbook-action-bar.md) — cast pipeline and the
  `CombatLogEvent` this feature consumes.

## Implementation inventory

- `godot/rust/src/combat_text.rs` — Retail CVar defaults, world-text type table (colours,
  durations, heights, rise), start spread, size ramp, `CombatLogEvent` → text mapping.
- `godot/rust/src/spells/floating_text.rs` — `Label3D` spawn, per-frame advance (rise, ramp,
  fade), the per-unit stack.
- `godot/rust/src/spells.rs` — the `SpellsHud` fields holding the live texts and the seen-event
  cursor.
- `godot/rust/src/spells/snapshot.rs` — `combat_text` count for automation.
- `godot/rust/src/lib.rs` — `account_state().ui_skin` (skin assertions) and `preview_*` hooks
  used by captures.

## Tests asserting this spec

- `godot/rust/src/combat_text.rs` unit tests — mapping (text, colour, style per event), CVar
  values, start spread, ramp, 50-number frame cost.
- `godot/tests/melee_outcomes_live.gd` — live crit text is 1.5× the plain size (Forever).
- `godot/tests/combat_text_capture.gd` — Forever capture fixture: 50-number frame, crit/heal/
  avoidance set, with a no-text same-frame diff.

## Known gaps (current cycle)

- [ ] Incoming damage/healing on the local player uses no self combat text frame (Retail
      `Blizzard_CombatText`); only outgoing world text exists.
- [ ] Damage colour is split white physical / yellow spell, which no source supports.
- [ ] Crits settle at 1.5× and use a `font_size` swap instead of Retail's height ramp and
      1.83× ratio.
- [ ] No per-unit 4-slot limit and no crit-replaces-crit rule; numbers spread over one global
      sequence instead.
- [ ] Overheal (`CombatLogEvent.overflow`) is not subtracted or displayed.
- [ ] No directional offset (`floatingCombatTextCombatDamageDirectionalOffset_v2`).
- [ ] No CVar store; the Retail defaults are compile-time constants.

## Out of scope

- The self combat text frame's non-combat messages (honor, reputation, XP, threat, combo points,
  auras, PvP rank icons) — they need `C_CombatText` engine events that the server does not send.
- Pet-battle floating text (`enablePetBattleFloatingCombatText_v2`), which belongs to the
  pet-battle HUD.
- World-object building damage text (15890 types 8/9), which needs destructible buildings.
