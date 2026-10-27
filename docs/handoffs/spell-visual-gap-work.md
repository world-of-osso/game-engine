# Spell visual census gaps

## Type 1: procedural effect semantics unresolved

The 70448 census identifies type 1 in 220 distinct supplied spell IDs (476 spec memberships). WoWDBDefs `SpellVisualKitEffect.dbd` identifies `Effect` as a `SpellProceduralEffect` ID, but its `SpellProceduralEffect.dbd` identifies only `Type` and four `Value` fields, not their behavior. For example, Rend 772 / kit 279229 references procedural row 376875: type 27, values 42, 0, -2.14748365e+09, 0. Cleave 845 / kit 279319 references row 376888: type 30, values 2, 0, 0, 0. The local reference clients searched under `~/Repos` have no `SpellProceduralEffect` implementation. [Mount Special](../specs/mount-special.md) also records this unresolved boundary.

Sources: [SpellVisualKitEffect definition](https://github.com/wowdev/WoWDBDefs/blob/master/definitions/SpellVisualKitEffect.dbd), [SpellProceduralEffect definition](https://github.com/wowdev/WoWDBDefs/blob/master/definitions/SpellProceduralEffect.dbd). The input CSV is `data/db2/active/SpellProceduralEffect.csv`; the full census is in `data/diagnostics/spell-visual-census-2026-10-27/ledger.json`.

**Named gap: procedural subtype and parameter semantics.** The table identity is known, but the rendering behavior is not. No implementation, failing behavior assertion, or after-capture is claimed. Continue to refraction rather than guessing.
