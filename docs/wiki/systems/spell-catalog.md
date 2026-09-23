# Spell catalog

Client-side lookup of static spell data for tooltips, spellbook and action bars. Code: `src/game/spell_catalog/`. Phase 1 input of the [in-game UI plan](../../plans/2026-09-23-ingame-ui.md).

## Source and load

- CSVs: `data/db2/12.1.0.69933/` — SpellName, Spell, SpellMisc, SpellEffect, SpellPower, SpellCastTimes, SpellRange, SpellDuration, SpellRadius, SpellCooldowns, SpellCategories, SpellCategory, SpellAuraOptions.
- Per-difficulty tables contribute only `DifficultyID = 0` rows. 3,382 SpellName ids have no SpellMisc row and keep zeroed misc fields.
- `SpellCatalogPlugin` spawns the load on `AsyncComputeTaskPool` at `Startup`; `SpellCatalog.state` goes `Loading` → `Ready` / `Failed`.
- Cache: `data/cache/spell_catalog-12.1.0.69933.bin`, bincode (`CacheKey` then `Vec<CatalogSpell>`). Key = format version + build + (size, mtime ns) of every source CSV; any mismatch or decode error rebuilds and rewrites it.

## Measured (dev test profile, 2026-09-23)

| | |
|---|---|
| Spells | 414,027 |
| Cold (CSV build + cache write) | 10.7 s |
| Warm (cache read) | 0.89 s |
| Cache file | 53 MB |
| Heap estimate (structs + text + effects, excl. allocator overhead) | 114 MB |

The crate itself is unoptimized in dev builds, so cold times there are pessimistic. Command: `cargo test --lib spell_catalog_load_probe -- --ignored --nocapture`.

## Description tokens

`render_description` / `render_aura_description` substitute from DB rows only:

- `$sN`/`$wN` (abs base points), `$mN` (signed), `$tN` (period s), `$aN` (radius yd), `$oN` (points × floor(duration/period)), `$xN` (chain targets).
- `$d` (humanized duration: `sec`/`min`/`hour(s)`/`day(s)`), `$u` (max stacks), `$n` (proc charges), `$h` (proc chance).
- Any of these can be prefixed by a spell id (`$57724d`). Tokens are case-insensitive.
- Verbatim: `${expr}`, `$?cond`, `$<var>`, `$@name`, `$/n;sN`, `$l`/`$g` plurals, and any token whose data is missing or ≤ 0. Branch text inside `$?cond[...]` is still substituted.

## Limits

- There are no caster stats. Spell-power-scaled effects render their stored `EffectBasePointsF`, which is often 0 (e.g. Fireball: "causes 0 Fire damage").
- About 3.5% of effects have fractional base points. They render with up to two decimals.
