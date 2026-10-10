# Talents UI (trait tree window)

Historical Bevy implementation, retired2026-10-02. Current native layout, art and behavior: [Talents](talents.md) and its [contract](../../specs/talents.md). Sizes, source paths and gaps below describe the retired client, not current acceptance.

Retail-style talent window `PlayerSpellsFrame`, driven by the server trait config messages. Wide window class of the [in-game UI plan](../../plans/2026-09-23-ingame-ui.md): centered, 1000×680, top y=104. The server model and validation rules are in game-server `docs/wiki/systems/talents.md`. The client is display-only: the server validates every commit.

## Code

| Piece | Path |
|---|---|
| Tree data (CSV load, model) | `src/game/talent_tree/{mod,load}.rs` |
| Mirrored rules (granted, validate, spent) | `src/game/talent_tree/rules.rs` |
| Local edits of a snapshot config | `src/game/talent_tree/session.rs` |
| Network state (snapshot, pending, outgoing; spec from `ActiveSpecialization`) | `src/talent.rs` (`TalentState`, `TalentPlugin`) |
| View model + layout + tooltips | `src/ui/screens/talent_frame_view.rs` |
| `rsx!` window | `src/ui/screens/talent_frame_component.rs` |
| Scene: build/sync, clicks, edge rotation | `src/scenes/talent_frame/` |

## Data

`TalentTreePlugin` loads on `AsyncComputeTaskPool` at `Startup` (`TalentTrees.state`: `Loading` → `Ready`/`Failed`). The result is cached in `data/cache/talent_trees-12.1.0.69933.bin` (bincode, keyed by format, build, and the size and mtime of every source CSV) through `src/game/db2_cache.rs`, the helper the [[spell-catalog]] also uses.

- Trees: the ten character-creation classes, class skill line → `SkillLineXTraitTree` (same table as the server, Paladin 800 → 790).
- Per node: `PosX`/`PosY`, `Type`, `TraitSubTreeID`, entries in `_Index` order (`TraitNodeEntry` → `TraitDefinition` spell, override name/icon, `MaxRanks`), groups, parents (`TraitEdge` types 2/3; any other type fails the load, as on the server), and the conditions and costs linked to the node, its groups and its entries.
- `SpecSetMember` resolves condition spec sets. `TraitTreeXTraitCurrency` gives the currencies and their flags (0x4 class, 0x8 spec). `TraitSubTree` gives the hero tree names. `ChrSpecialization` gives the class specs.
- Entry `passive` = `SpellMisc.Attributes_0 & 0x40` at `DifficultyID` 0. This picks the node shape.
- Art: every `UiTextureAtlasMember` named `talents-*` (from `data/UiTextureAtlas*.csv`), cropped from its `UiTextureAtlas` texture.

Not loaded: `TraitCurrencySource`. Owned points come from the snapshot instead: `unspent` plus what the snapshot config spends.

## Art (authored Retail atlas members)

| Use | Atlas member | Texture FDID |
|---|---|---|
| Node border | `talents-node-{square,circle,choice}-{yellow,green,gray}` | 4556093 (`interface/talentframe/talents.blp`, atlas 1970, 2048×1024) |
| Edge | `talents-arrow-line-{yellow,gray}` | 4636998 (`talentsarrowline.blp`, atlas 2071) |
| Background | `talents-background-<class>-<spec>`, e.g. `talents-background-paladin-retribution` (member 15969) | 4631340 (`talentsclassbackgroundpaladin2.blp`, atlas 2048) |
| Buttons | `defaultbutton-nineslice-*` (toolkit atlas) | — |
| Unknown icon | `INV_Misc_QuestionMark` | 134400 |

Node icons are the entry's `OverrideIcon`, else the `SpellCatalog` icon. Shapes: square = active spell, circle = passive, choice (octagon) = Selection or SubTreeSelection. A choice node shows the left half of its first option and the right half of its second, and each half is clickable. Border colour: yellow = maxed, green = purchasable or partly ranked, gray = not purchasable now.

Not used yet: icon masks (`talents-node-circle-mask`), gates (`talents-gate`), glows and animations. Circle icons are inset so their square corners stay inside the ring.

## Layout

Class tree on the left, hero tree in the middle, spec tree on the right. All three use one scale that fits the 960×(628−86) tree area. Class and spec rows share the same top row. The hero column shows the visible SubTreeSelection node at the top, and under it the selected hero sub-tree normalized to its own bounds. Before a pick it shows only the selection node, which is visible from level 71.

Section by node: SubTreeSelection → hero selection. `TraitSubTreeID` ≠ 0 → hero. Otherwise, a cost in a class-flag currency → class, else spec. Nodes are shown when their Visible conditions pass for the spec and level.

Edges run from center to center, shortened by one node size. `rsx!` has no rotation attribute, so after each sync the scene sets `TextureData.rotation = −angle` on every `TalentEdge_<from>_<to>`. Toolkit rotation is counterclockwise, and the angle is a y-down screen angle.

## Registry names

`PlayerSpellsFrame`, `PlayerSpellsFrameTitle`, `PlayerSpellsFrameBackground`, `TalentNode_<nodeId>` (children `…Icon`, `…Border`, `…Rank`, choice halves `TalentNode_<nodeId>Choice<entryId>`), `TalentEdge_<from>_<to>`, `TalentClassPointsText` / `TalentHeroPointsText` / `TalentSpecPointsText` ("Paladin  31"), `TalentSpecButton<i>`, `TalentApplyButton`, `TalentResetButton`, `TalentTreeStatePanel` (loading/error).

## Behaviour

- **States:** Loading while tree data, the first `TraitConfigSnapshot`, or the local `UnitLevel` is missing. Error when the tree load failed or the snapshot names an unknown tree.
- **Left click** buys a rank of the node's entry. For tiered (apex) nodes it buys the first entry that is not maxed. For a choice half it buys that entry, replacing a bought choice of the same node. **Right click** refunds one bought rank (the last entry with ranks); granted ranks cannot be refunded. An edit is kept only if the whole resulting config passes the mirrored `validate`, so a refund that would strand a child is refused.
- **Apply** (enabled with pending edits) opens a `PopupStack` confirmation "Apply talent changes?" (Accept/Cancel, key `TALENT_APPLY_CHANGES`, plan rule 11). Only Accept queues `CommitTraitConfig { spec_id, entries }` on `TalentChannel`. Ranks are totals (granted + bought), as the snapshot sends them. Pending edits stay until the next snapshot replaces them. `TraitCommitResult { ok: false }` adds its reason to `UiErrors` (UIErrorsFrame).
- **Reset** drops the pending edits.
- **Spec buttons:** the class specs except the Initial one (`OrderIndex` 4). They are disabled below level 10 (server `SPEC_UNLOCK_LEVEL`) and for the active spec. A click sends `SetSpecialization`. The client does not receive `SpecializationChanged` itself: `player_spells` owns that receiver and writes `ActiveSpecialization`, and `TalentPlugin` reacts to changes of that resource. A change to another spec drops the snapshot, so the window shows Loading until the new spec's snapshot arrives.
- **Tooltip:** the shared `TooltipFrame` looks up the hovered frame (or its nearest ancestor) in `TalentTooltips`. It shows the name (override name or spell name), "Rank x/y", and the `SpellCatalog` rendered description wrapped at 46 characters.

## Mirrored rules

A port of game-server `trait_config` (TrinityCore `TraitMgr`): granted entries, Visible/Available/RanksAllowed condition lists (IsSufficient, spec sets, level), gates (spend in earlier rows by gate bucket), rank conditions, parents (Sufficient/Required, Selection filled by any maxed entry), choice nodes with exactly one entry, one hero tree and bought ranks only in the selected one, and the budget. The only difference from the server: owned points come from the snapshot instead of `TraitCurrencySource`.

## Load time

Measured on 2026-09-23 in the dev test profile (debug crate, deps at opt-level 2), with a machine load average of about 120:

| | |
|---|---|
| Cold (CSV build + cache write) | 1.49 s |
| Warm (cache read) | 7.4 ms |
| Cache file | 151 KB |

The cold load includes the 417k-row `SpellMisc.csv` scan for the passive flags. Command: `cargo test --lib talent_tree -- --nocapture`.

## Known gaps

- No gate art or "spend N more points" text. No search, loadouts, or import/export.
- Edge rotation direction and node art are unverified in a native capture.
- The window does not block world clicks under it.

## Sources

- game-server `docs/wiki/systems/talents.md`, `crates/server/src/trait_config.rs`, `trait_config_tests.rs` (node IDs used in the client tests)
- `data/UiTextureAtlasMember.csv` rows `talents-*`

## See Also

- [[ui-system]]
- [[spell-catalog]]
