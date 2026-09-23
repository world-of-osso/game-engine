# Spell system baseline (phase 1 input)

**Status:** Code-read evidence, 2026-09-23. Input to phase 1 of the [in-game UI plan](2026-09-23-ingame-ui.md). Not runtime-verified.

## Summary

- Retail spell data is loaded into world SQLite.
- The server resolves no gameplay:
  - Casts only produce a replicated `CastState` bar (`game-server cast_presentation.rs:1`, "Expiry does not resolve spell effects").
  - Instant spells are rejected (`:51-53`).
  - No cost is spent, no cooldown or GCD starts, and no aura is applied.
  - There is no heal path.
  - Melee is a flat 10 damage every 2 s within 5 yd (`combat.rs:14-18`).
- The protocol crate is `shared-protocol` (patched by path). `game-server/crates/shared` is a stale, unused copy.

## Data on hand

`game-server/data/world.db` is imported by `import-azerothcore.sh` from `game-engine/data/*.csv`. The column names match WoWDBDefs 12.0.0.63534+. The exact build was not recorded.

| Table | Rows |
|---|---|
| spell_name / spell | 399,649 |
| spell_misc | 403,223 |
| spell_effect | 607,032 (577,341 at DifficultyID 0; 251,211 APPLY_AURA over 520 aura types) |
| spell_power | 5,677 |
| spell_cooldowns | 35,547 |
| spell_duration / cast_times / range | 428 / 268 / 222 |

- Complete Trait* CSVs are present in `game-engine/data/`: TraitTree 190, TraitNode 11,710, TraitDefinition 16,631, plus edges, conds, costs, currencies, subtrees, loadouts and SkillLineXTraitTree 132. No code reads them.
- These tables are absent, and local CASC cannot supply them (missing TACT keys `0x583C5B29BF208655` and `0x1DACCE2B44C78902`, or not in local archives):
  - SpellAuraOptions, SpellCategories, SpellLevels, SkillLineAbility, SpecializationSpells
  - SpellClassOptions and SpellProcsPerMinute (both do extract as WDC5)
  - ChrSpecialization, SpellShapeshift, SpellInterrupts, SpellTargetRestrictions, SpellEquippedItems (not tested)
- Plan: import build-pinned wago.tools CSV exports into `data/`, the same way character creation already does for 12.1.0.69875.

## Defects in the current loader (`game-server spell_db.rs`)

- **Effect IDs:** interrupt is mapped to 32 and dispel to 19. Retail uses 68 for interrupt (Kick 1766, Pummel 6552, Counterspell 2139) and 38 for dispel (Purify 527).
- **Power types:** 5 maps to RunicPower, but DK spells use 6. Types 7, 8, 11, 12, 13, 16, 17, 18 and 19 are unmapped (Soul Shards, Astral, Maelstrom, Chi, Insanity, Arcane Charges, Fury, Pain, Essence).
- **Costs:** percent costs (`PowerCostPct`) are dropped, so Fireball 133 has no cost. Rage/Runic Power costs look stored ×10 (inference) and are used raw.
- **Effects and targets:** only the first 3 effects and effect 0's implicit target are used. Multi-school spells collapse to their highest bit. Durations are never read.
- **Resources:** `ResourceType` has 7 variants. `class_spec.rs:91` gives Demon Hunter Energy.

## Unused scaffolding

- `shared-protocol`:
  - `aura.rs`: 13 AuraEffect variants, stacking/absorb/periodic helpers, and ProcDef/`check_procs`.
  - `casting.rs`: GlobalCooldown, SchoolLockouts, SpellCooldowns, SpellCharges, SpellProjectiles.
  - `spell_catalog.rs`: 5 hand-made Warrior spells.
  - `CastFailReason` never reaches the client.
- Server:
  - `Resources`, `Auras`, `SpellCooldowns` and `GlobalCooldown` are never inserted.
  - Talents are 28 hardcoded Paladin talents for every class.
  - `BaseStats` is only inserted in tests.
  - New characters get 100/100 health.
- Client:
  - `spell_list_data.rs`, `cooldown_data.rs` and `action_bar_data.rs` are unused.
  - `buff_data.rs` has no network producer.
  - `casting_data.rs` is filled only by gathering.
  - The spellbook is hardcoded Paladin.

## Classes

Character creation offers Warrior, Paladin, Hunter, Rogue, Priest, Death Knight, Shaman, Mage, Warlock and Druid (`game-engine src/scenes/char_create/data.rs:213-274`). It has no Monk, Demon Hunter or Evoker. The server does not validate class.
