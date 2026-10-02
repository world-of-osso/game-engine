# Spell description tokens

`Spell.Description_lang` / `AuraDescription_lang` (DB2 12.1.0.69933) are templates. The
client renders them in `src/game/spell_catalog/render.rs` + `render_eval.rs`.

## Sources

- No warcraft.wiki.gg page documents the grammar (`Spell_description_tokens` returns 404,
  checked 2026-09-25). wowdev.wiki `Spells` (https://wowdev.wiki/Spells) has an identifier
  table and says "Below list is missing a lot, still"; it is behind Cloudflare, so only its
  table is summarized below.
- `vels-spell-string-tags.txt` (this directory): "Vels Spell String Tag Documentation" from
  https://github.com/stoneharry/WoW-Spell-Editor (3.3.5-era client), fetched 2026-09-25.
- Token frequencies from the local `Spell.csv` (e.g. `$s` 118504, `$@spelldesc` 41987,
  `$?` 10174, `${` 7102, `$<` 1059 uses).

wowdev.wiki identifier table (case-insensitive unless noted): `$a` effect radius, `$b`
EffectPointsPerResource, `$c` specialization conditional (1 = first spec), `$d`
SpellDuration, `$e` EffectAmplitude, `$g` gender form, `$h` proc chance, `$i` max targets,
`$m` EffectBasePointsF, `$n` proc charges, `$p` EffectMiscValue, `$r` range, `$s` effect
points, `$t` aura period, `$u` max stacks, `$v` max target level, `$x` chain targets,
`$z` hearthstone location.

## Resolved

| Token | Value |
| --- | --- |
| `$s1` `$S1` `$w1` `$W1` | \|EffectBasePointsF\| of effect 1 (index defaults to 1) |
| `$m1` `$M1` | signed EffectBasePointsF |
| `$t1` | aura period, seconds |
| `$a1` | effect radius, yards |
| `$o1` | points x (duration / period) |
| `$x1` | chain targets |
| `$d` | SpellDuration text: `15 sec`, `1.5 min`, `1 hour`, `2 days` |
| `$u` `$n` `$h` | max stacks, proc charges, proc chance |
| `$r` | max range (hostile, else friendly) |
| `$12345s1` | any token taken from spell 12345 |
| `$?cond[a]?cond[b][c]` | else-if chain; whitespace allowed before `[` |
| cond `s123` / `a123` / `c2` | spell known / aura on the player / active spec OrderIndex + 1 |
| cond `$w3>0`, `$s1=5`, `!`, `&`, `\|`, `( )` | comparisons (`< > = <= >= !=`) and logic |
| `${expr}.N` | `+ - * /`, parens, `$gte $gt $lt $lte $abs $floor $ceil $max $min $cond`; N decimals, default 0 |
| `$/1000;s1` `$*2;s1` | token divided / multiplied |
| `$lpoint:points;` | singular after the number 1, else plural |
| `$@spelldesc123` `$@spelltooltip123` `$spelldesc123` | rendered description of 123 (depth 4) |
| `$@spellaura123` `$@auradesc123` | rendered aura description of 123 |
| `$@spellname123` | spell name |
| `$s1` etc. of a spell/attack power scaled effect | base points + trunc(`EffectBonusCoefficient` × spell power) + trunc(`BonusCoefficientFromAP` × attack power), from the owner-only `DerivedStats` (TrinityCore `Unit::SpellDamageBonusDone`); no crit, versatility or aura percentages |
| `$SP` `$sp` `$AP` `$ap` | the player's spell / attack power |

## Unresolved

Rendered as `{?<token>}` and logged once per spell and token (`warn!`):

- Effect points that scale with level (`ScalingClass` with a `Coefficient`; the client
  has no ExpectedStat data), and spell/attack power scaled points (`EffectBonusCoefficient`
  or `BonusCoefficientFromAP` non-zero on a SCHOOL_DAMAGE / HEAL effect or PERIODIC_DAMAGE /
  PERIODIC_HEAL aura, or with 0 stored points) before `DerivedStats` arrives.
- Other caster stats (`$MHP`, `$pri`, `$PL`, `$SPH`, `$RAP`, ...), `$<var>` (no
  SpellDescriptionVariables table locally), `$g` gender forms, `$@spellicon`,
  `$@versadmg`, `$@switch`, garrison / loot-spec references, `$j`, `$e`, `$i`, `$p`, `$q`.
- Conditions other than `s`/`a`/`c`/comparisons (`diff`, `pc`, `j1g`, ...).
- `$d` of a spell without a positive duration.
