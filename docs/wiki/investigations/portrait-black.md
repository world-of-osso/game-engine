# Black unit-frame portraits

Verified: 2026-10-10. Scope: Human/Orc warlocks, summoned Imp display4449, Human paladin control; Modern and Forever art skins. Receipt publication and final live proof are recorded below when complete.

## Root cause

TwoWarlock is Human (race1), class9: `data/diagnostics/spell-live2-2026-10-10/roster.txt`. The original process logged missing authenticated receipts for starter cloth texture3946815 and Imp model1098889. The renderer never attached those visuals; the correctly masked transparent portrait becomes a black circle. Existing unqualified files are not valid substitutes for product/build-qualified assets.

Private UDP54133 reproduction, canonical prebuilt server, fresh `fb_portraitblack` account: Human and Orc warlock portraits have zero colored pixels and `model_shown=false`; Human paladin has848 colored pixels. Human warlock's targeted summoned Imp also has zero pixels and `model_shown=false`. Errors name the missing receipts, before camera framing. No camera/attachment or render-target workaround is appropriate.

`godot/tests/portrait_black.gd` boots production startup and inspects real portrait viewport images. A check requires an attached, non-pending model and at least32 colored pixels in the player/optionally target slot. RED: `blackwarlock-modern-fixture-red-client.log`, `fixture-red-state.json` under `data/diagnostics/portrait-black-2026-10-10/`.

## Fix

`scripts/stage_portrait_assets.py` stages the authored purple/yellow starter-cloth replacement texture sets and Imp's complete embedded dependency chain from authenticated frozen Retail resolution plus local archives. The existing [product-isolated importer](../../specs/product-isolated-model-assets.md) verifies magic, content-key MD5, SHA256 and source identity; publishes scoped bytes and additive `model-asset-index-portrait-black.json`. No legacy fallback, fabricated assets, shader/camera change or `world.db` write.

Run under `scripts/agent/agent-run portrait-black python3 scripts/stage_portrait_assets.py` with `--data`, `--source-manifest` (existing authenticated Retail69933 chain receipt), `--resolution` (matching frozen snapshot), `--archives` (local CASC `Data/data`), `--extractor` (Depot-built `target/debug/examples/model_asset_frozen_extract`), `--keys`, and `--output` (persistent diagnostics). The native helper builds the extractor with `--fixture model_asset_frozen_extract`.

Retail source: local `blizzard-ui/retail/AddOns/Blizzard_UnitFrame/Mainline/UnitFrame.lua:175-199` calls `SetPortraitTexture(self.portrait, self.unit, self.disablePortraitMask)` and refreshes on `UNIT_PORTRAIT_UPDATE`. Engine [M2 camera parser](../../../godot/core/src/asset/m2_format/m2_camera.rs) selects authored portrait camera type0; framing is unchanged. Forever remains an art skin, not another model source.

## Evidence

PNG directory: `/syncthing/AgentShared/2026-10-10/portrait-black/`; logs/receipts/states: `data/diagnostics/portrait-black-2026-10-10/`. Baseline Human/Orc warlock and Human paladin PNGs inspected; black player and black Imp reproduced. Post-publication proof pending.
