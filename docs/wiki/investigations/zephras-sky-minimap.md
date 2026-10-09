# Zephras sky and minimap defects

Map2991 private native reproduction, 2026-10-09: orange/black sky panel and empty Forever minimap. Both are code defects, not established missing assets or TACT keys. Changes are scoped to map2991; other maps retain their current rendering paths. [Forever source context](../systems/forever-data.md).

## Minimap

`data/db2/1.60.1.70205/Map.csv:62` names map2991, directory `2991`, WDT FDID7198644. Its WDT MAID names minimap7199025 for tile(28,24); no corresponding `world/minimaps/2991/map28_24.blp` listfile entry exists. Native `load_tile` previously returned `None` at the listfile lookup, before looking at this usable FDID. The scoped correction reads this map's existing WDT MAID and decodes its authored minimap FDIDs. No tile names or substitute images are invented.

Local CASC7199025 content key `5f81482b149c2bb7d666247c33b3e594` matches freshly extracted bytes. Its decoded tile has real terrain pixels. Tile7199013 at(28,22) is genuinely authored black (every mip0 BC1 block `00000000aaaaaaaa`); this does not explain the missing tiles at the actual spawn.

Behavioral RED: `zephras_authored_minimap_tile_decodes_without_a_listfile_name` failed with missing tile, exit101. GREEN and private after capture pending at initial documentation checkpoint.

## Sky

Source chain: `Light.csv:528` row15617 → LightParams7588 (`LightParams.csv:671`) → LightSkybox683 (`LightSkybox.csv:15`, flags3) → M2 FDID7345733. Real skin7345740 has six batches, all shader0x8012, texture triplets8164017–8164034 and texture-weight indices0/1/2. `godot/core/src/m2_material.rs` maps0x8012 to pixel26, Mod_Dual_Crossfade. The equivalent native ordinary-M2 shader already computes `mix(mix(first,second,weight1),third,weight2)`.

The old sky shader instead computed `first * mix(second,third,third.a)`; `SkyModel` sampled only overall opacity, not the second/third texture weights. This multiplies different day-phase skies and creates wrong dark/orange regions. The scoped sky route samples the original weight tracks at the held M2 day fraction and enables the proper combiner only for map2991/sky7345733.

Actual-texture GPU test `godot/tests/zephras_sky_crossfade.gd`: original shader endpoint0 RGB error0.129412 (RED); corrected shader all three endpoints maximum RGB error1/255 (GREEN). This is shader proof, not yet acceptance of the rebuilt native consumer or whole-day visual parity.

## Asset provenance and proof boundary

Fresh extraction used only `/syncthing/World of Warcraft/Data`, product `wow_classic_beta`, actual local resolution cache build `e8dd824cf6c3d96cd01f804ca2ea5a63`. All23 extracted M2/texture bytes match that cache's CASC content-key MD5; existing runtime sky M2, skin0 and all18 textures match too. Sky M2 MD5 `5c5db4fb21c86cdca3acaf5841ca02fd`, encoding key `e13972d1e8beb86d7fbe36bf03513dca`. WDT SHA256 `f3710569f77e4d9bc96667e8acf2602321f1633d03d8febf62db5e56ab4aa29c`. No unknown key, asset replacement, world DB rewrite or Retail substitution was used.

Evidence: `data/diagnostics/skypatch2-2026-10-09/`: `before.png`, personally inspected downscale `before-inspect.png`, `before-snapshot.json`, `before-client.log`, `extract.log`, `red.log`, `sky-red.log`, `sky-green.log`. Before capture used a fresh private redb/account `fb_skypatch2`, UDP5480; server/client stopped immediately afterward. Rebuilt private after remains pending. The initial startup attempt hit a stale shared5480 token; its original bytes were backed up and restored, never touching5000.

## Sources

- [Minimap contract](../../specs/minimap.md), [native minimap](../systems/minimap.md).
- `godot/core/src/asset/wdt.rs` — MAID minimap slots and coordinate order.
- `godot/rust/src/{minimap,sky_model}.rs`, `godot/rust/src/lighting/mod.rs`, `godot/shaders/sky_m2.gdshader` — native consumers.
- `godot/core/src/m2_material.rs`, `godot/shaders/m2.gdshader` — maintained WebWowViewerCpp1a8cccb shader-table/Mod_Dual_Crossfade reference port.

## See Also

- [[forever-data]] — source isolation and unrelated NPC acquisition gaps.
- [[skybox]] — sky depth/ordering, separate from this combiner defect.
