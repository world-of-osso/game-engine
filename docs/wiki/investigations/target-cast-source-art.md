# Modern target cast source art

The Modern target bar used flat background/fill/pip frames despite already carrying the Retail target track geometry. The approved Forever skin used those flat fills intentionally; the player feedback path already selected source atlases. [HUD contract](../../specs/hud-edit-mode.md#target-cast-bar-and-replicated-unit-relationships).

## Correction

`retail_target_background` supplies `ui-castingbar-background` and `ui-castingbar-textbox`; the textbox extends12 below the150×10 track. `cast_fill` shares the existing atlas decision with the player path, cropping the selected standard/channel/uninterruptible/interrupted member to progress rather than compressing a full member. The6×16 source pip centres on the fill edge, with the red pip for interrupted samples. Existing target anchors, icon/shield/border, labels/timer, alpha and reducer behavior remain unchanged. Forever uses its previous solid fills; all four rendered Forever controls are byte-identical before/after.

## Bounded proof

The source-render-input test failed on missing `TargetCastingBarBackgroundArt`, then passed alongside target visibility/anchors/interruption and player spark controls:9 tests. Four states at0/25/100% verify source selection, cropping and pip/textbox geometry. Native fixture `target_cast_art.gd` loads the actual textures, checks textbox/pip bounds and captures cast/channel/locked/interrupted samples in both skins at1920×1080. All sixteen before/after target regions and both comparison sheets were inspected; the native after run passes8 samples. Build/check/format and manual changed-Rust audit pass.

Captured implementation: `bbed18488`; before fixture-only revision: `cd217b3b3`. PNGs: `AgentShared/2026-10-14/ui-questlog-target-cast/{before-source-art,after-source-art}/`. Exact commands, failures and source/binary/image hashes: `data/diagnostics/ui-questlog-target-cast-2026-10-14/target-cast-proof-ledger.json`. The initial unqualified exact test filter ran0 tests and is invalid evidence; the corrected qualified filter produced the genuine RED.

These are supplied presentation states, not a server cast/interrupt or lifecycle proof. No new completion/glow/empowered/crafting effects, spell-target-name text, nameplate or player styling is claimed.

## Sources

- Retail `Blizzard_UIPanels_Game/Mainline/CastingBarFrame.xml:399–454` — SmallCastingBarFrameTemplate regions and anchors.
- Retail `Blizzard_UnitFrame/Mainline/TargetFrame.xml:529–531` — target track height override.
- Retail `Blizzard_UIPanels_Game/Shared/CastingBarFrame.lua:45–80,198–200,650–653,687–696` — state atlases, crop-edge pip placement and red interrupted pip.
- [HUD contract](../../specs/hud-edit-mode.md#target-cast-bar-and-replicated-unit-relationships) — unchanged target anchoring, timer extension and approved Forever art.
- [Player feedback](../systems/player-cast-feedback.md) — separate player feedback clock and animation scope.
