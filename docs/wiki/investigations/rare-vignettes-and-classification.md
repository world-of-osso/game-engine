# Rare vignettes and target classification

Verified 2026-10-09 against imported server world.db, local CASC and Retail Mainline UI. A rare classification does not create a minimap vignette. Forever's target frame also bypassed the existing classification elements; its target overlay now includes them without changing other frame shapes.

## Classification and vignette are independent

The server reads `content_creature_template.rank` and replicates `CreatureClassification` (`game-server/crates/server/src/world_data.rs`, `zones.rs:228`). Among spawned primary-template IDs, rank2 has91 entries/172 spawns and rank4 has224/285; dungeon maps contribute alongside maps0/1/530/571. All315 have `tdb_creature_template.VignetteID=0`. TDB Classification differs from content rank for64 IDs; this investigation reports those differences without changing either source.

Creature vignettes require the server-side TDB `VignetteID` joined to a real Vignette row. Engine `godot/rust/src/vignettes.rs` requires replicated `UnitVignette` and Position; minimap.rs:439-440 draws only those sightings with `on_minimap()`. Ordinary target/tracking blips are separate, not rare skulls. Retail's generated `VignetteInfoDocumentation.lua` defines `C_VignetteInfo.GetVignettes`, `GetVignetteInfo` and `onMinimap`; world-map `VignetteDataProvider.lua:114-144` consumes actual vignette GUIDs rather than inferring them from classification. Minimap markers are native client behavior, not a classification loop in Mainline Minimap.lua.

A name join would assign the wrong creatures: Ruul2602 has no vignette, while Ruul142683 has3216; Singer2600 has none, while Singer142690 has3217. The other shared names likewise belong to distinct newer IDs. Never copy these assignments to older IDs.

Local CASC Creature FDID841631/layout6E14C900 has23,031 readable IDs but none of these315;48 encrypted records were skipped, so no DB2 rank agreement was asserted. Local Vignette FDID892861/layout1A5A691B decodes7,226 IDs including copies, exactly matching all imported IDs and Flags. Neither table defines a creature-to-vignette foreign key. TrinityCore a352b1fa's DB2Structure.h contains only VignetteEntry itself, not another creature mapping. No missing vignette import was reproduced. Wowhead and wago page fetches:0.

## Forever target classification gap

Native run1 selected real Ruul2602 on grounded terrain but found no classification controls: `target_frame` returned the Forever frame before calling `classification_art`. The target already carries its full UnitFrameState into `FlareUnit.aura_state`, so its existing overlay can draw the shared classification elements without adding state plumbing or changing non-target frames.

Mainline `TargetFrame.lua:437-445` shows a silver dragon only for rareelite and gold for elite; `:459-462` shows a star for rare/rareelite. Forever1.60.1.69913 UiTextureAtlas tables contain set1 gold/silver dragon members on FDID8244541 (1x) and8244544 (2x). Atlas names are `UI-HUD-UnitFrame-Target-PortraitOn-Boss-Gold` and `ui-hud-unitframe-target-portraiton-boss-rare-silver`. `UI-HUD-UnitFrame-Target-PortraitOn-Boss-Rare-Star` has no set1 member and uses Retail set0 sheets4703659/4703662 through the existing skin resolver. No generated or recolored art was added. The shared classification helper retains Retail target-relative anchors and authored atlas sizes.

## Proof and limits

Three Forever registry tests reproduced missing star/dragon controls (REDf314d1b52, exit101). GREEN and related unit-frame verification pending. Native run2 before the fix captured Modern Ruul with silver dragon/star, one ordinary target blip and zero vignettes. Both requested PNG paths contain the same native frame. Run3 Brack520 could not be selected via native input, despite grounded staging; rank4 capture remains unproved. Three-run budget exhausted: no post-fix Forever native proof.

## Sources

- Local Retail UI: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_UnitFrame/Mainline/TargetFrame.lua`, `Blizzard_APIDocumentationGenerated/VignetteInfoDocumentation.lua`, `Blizzard_SharedMapDataProviders/VignetteDataProvider.lua`.
- Local CASC extracted DB2 and exact atlas/member rows: `data/diagnostics/rares-2026-10-09/` (untracked evidence).
- `game-server/crates/server/src/{vignettes,world_data,zones}.rs`; imported `game-server/data/world.db`.
- [HUD preset contract](../../specs/hud-edit-mode.md); `godot/ui-model/src/ui/screens/inworld_unit_frames_{component,flare,art}.rs`.
- Primary schemas cached in diagnostics: WoWDBDefs Creature/Vignette definitions and TrinityCore a352b1fa DB2Structure.h/DBCEnums.h.

## See Also

- [[minimap]] — ordinary blips and vignette rendering.
- [[native-hud-edit-mode]] — selected preset and skin ownership.
