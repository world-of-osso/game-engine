# Spell catalog

Client-side lookup of static spell data for tooltips, spellbook and action bars. Code: `src/game/spell_catalog/`. Phase 1 input of the [in-game UI plan](../../plans/2026-09-23-ingame-ui.md).

## Source and load

- CSVs: `data/db2/12.1.0.69933/` — SpellName, Spell, SpellMisc, SpellEffect, SpellPower, SpellCastTimes, SpellRange, SpellDuration, SpellRadius, SpellCooldowns, SpellCategories, SpellCategory, SpellAuraOptions.
- Per-difficulty tables contribute only `DifficultyID = 0` rows. 3,382 SpellName ids have no SpellMisc row and keep zeroed misc fields.
- `SpellCatalogPlugin` spawns the load on `AsyncComputeTaskPool` at `Startup`; `SpellCatalog.state` goes `Loading` → `Ready` / `Failed`.
- Cache: `data/cache/spell_catalog-12.1.0.69933.bin`, bincode (`CacheKey` then `Vec<CatalogSpell>`). Key = format version + build + (size, mtime ns) of every source CSV; any mismatch or decode error rebuilds and rewrites it. Storage is the shared `src/game/db2_cache.rs` (also used by [[talents-ui]]).

## Measured (dev test profile, 2026-09-23)

| | |
|---|---|
| Spells | 414,027 |
| Cold (CSV build + cache write) | 10.7 s |
| Warm (cache read) | 0.89 s |
| Cache file | 53 MB |
| Heap estimate (structs + text + effects, excl. allocator overhead) | 114 MB |

The crate itself is unoptimized in dev builds, so cold times there are pessimistic. Command: `cargo test --lib spell_catalog_load_probe -- --ignored --nocapture`.

## Passive flag and spellbook tabs

- `CatalogSpell.passive` = `SpellMisc.Attributes_0 & 0x40` (cache format 2).
- `SpellCatalogData.tabs` (`tabs.rs`) is rebuilt from ChrClasses, ChrSpecialization, SpecializationSpells, SkillLine and SkillLineAbility on every load; it is not cached.
- Rule: a spell in the active non-Initial spec's `SpecializationSpells` or mastery goes on the spec tab. A spell of a class skill line (`SkillLine` category 7 whose name equals a `ChrClasses` name, e.g. 800 Paladin) goes on the class tab. Everything else is General. Initial-spec (OrderIndex 4) spells go on the class tab.
- Tab titles: class from the spec's `ClassID`, else from the first known class spell; spec from `ChrSpecialization.Name_lang`.
- `CatalogSpell.spellbook` (`SpellbookListing`, cache format 9), client-side rules from TrinityCore a352b1fa SharedDefines.h: `Never` for `Attributes_0 & 0x80` (SPELL_ATTR0_DO_NOT_DISPLAY, :443: Warrior 137047, Block 123829, Initial Warrior 325446, Plate Specialization 86101) or `Attributes_4 & 0x8000` (SPELL_ATTR4_NOT_IN_SPELLBOOK, :599: Dodge 81, Block 107, Summon Friend 45927, Boost, Honk the Horn!, Return from Home); `UntilLearned` for `Attributes_7 & 0x10000` (:711, e.g. Judgment "Rank 2" 327977, Victory Rush 319158: listed as a future spell, hidden once known); `OnceLearned` for `Attributes_8 & 0x2000` (:745, e.g. mage teleports: hidden until known).
- The server teaches every GENERIC (SkillLine 183) AcquireMethod 2 row, as TrinityCore `Player::LearnSkillRewardedSpells` does (Player.cpp:25407-25455, no further filter), so the General tab still lists about 33 content-gated abilities (Garrison/Covenant/Venthyr/Delves/Pact abilities, Lorewalking, Pocopoc, two "Unraveling Sands" 436524 and 1232808). No documented attribute hides them; Retail's server gates them with conditions TrinityCore does not have.
- `CatalogSpell.auto_attack` (`SpellAutoAttack`, cache format 5): `OnCast` for `Attributes_1 & 0x200` (SPELL_ATTR1_INITIATES_COMBAT_ENABLES_AUTO_ATTACK: Slam, Mortal Strike, Attack 88163), else `PostCast` for `Attributes_2 & 0x100000` (SPELL_ATTR2_INITIATE_COMBAT_POST_CAST_ENABLES_AUTO_ATTACK: Smite), else `None` (Frostbolt). The client sends `AttackSwing` for them ([[spellbook-action-bar]]).
- Future spells (`SpellbookTabIndex::future_spells`): the class line's auto-learned rows (AcquireMethod 1, 2, 4, class and race masks) and the active spec's spells with `SpellLevels.SpellLevel` above the player level, by level. `build_spellbook_tabs(.., Some(SpellbookPlayer))` appends them with `available_at`.

## Godot

The catalog data, build, render and tabs files are engine-free (`data.rs`; the Bevy `SpellCatalogPlugin` stays in `mod.rs`) and compiled into `godot/core` as `spell_catalog` with `spellbook_data`. The Godot client loads it on a worker thread when it enters the world, caching at `user://spell_catalog-12.1.0.69933.bin`. The spellbook, action bar and casting that use it: [[spellbook-action-bar]].

## Description tokens

`render_description` / `render_aura_description` substitute from DB rows only:

- `$sN`/`$wN` (abs base points), `$mN` (signed), `$tN` (period s), `$aN` (radius yd), `$oN` (points × floor(duration/period)), `$xN` (chain targets).
- `$d` (humanized duration: `sec`/`min`/`hour(s)`/`day(s)`), `$u` (max stacks), `$n` (proc charges), `$h` (proc chance).
- Any of these can be prefixed by a spell id (`$57724d`). Tokens are case-insensitive.
- Verbatim: `${expr}`, `$?cond`, `$<var>`, `$@name`, `$/n;sN`, `$l`/`$g` plurals, and any token whose data is missing or ≤ 0. Branch text inside `$?cond[...]` is still substituted.

## Limits

- There are no caster stats. Spell-power-scaled effects render their stored `EffectBasePointsF`, which is often 0 (e.g. Fireball: "causes 0 Fire damage").
- About 3.5% of effects have fractional base points. They render with up to two decimals.
