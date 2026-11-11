# Halls of Infusion interior opening

A flooded Halls of Infusion channel exposes the background beyond its WMO geometry. At the recorded camera, the opening does not have a portal owned by the camera's group. The correct Retail background behind that opening remains unknown; this investigation does not establish that sky, clear color, or fog is correct.

## Recorded view

Engine master `4aa30b7b9b049c15d74a8d08cda9063706a8ce43` rendered player `(-75.0, -39.766, 250.0)` and camera `(-65.403, -34.463, 250.0)`, yaw `1.570796` and pitch `-0.35`. The fixture asserted `ui_skin == "Forever"` before the capture. The source report is `/syncthing/AgentShared/2026-10-30/user/halls-of-infusion-missing-walls.png`; the channel is a representative reproduction, not the exact reported position. Before image and portal overlays are in `/syncthing/AgentShared/2026-11-02/dungeon-walls/`.

The WMO is root FDID `4325339`. All 28 groups were present as runtime mesh groups. Groups 22 and 23 had 62 and 58 visible batches. Independently forcing only group 15 or 18 mesh batches visible added geometry over part of the opening; no doodad visibility was changed. The local root and group 15, 18, 22 and 23 files matched the installed CASC content by SHA-256.

With the actual WMO placement, the camera is file-local `(65.403, 250.0, -34.463)`. Of the 28 MOGI bounds only 22 and 23 contain it. A MOBN/MOBR downward-face query found group 22's floor at approximately `-44.087`; group 23 had no face at the point. The same result holds at the player. This is a reconstruction of the reference query, not an execution of Retail or WebWowViewerCpp.

Group 22 owns portal references 1→2, 11→20 and 13→14; group 23 owns none. None of the three portals intersects the recorded camera frustum. Groups 15 and 18 have their own portals beyond the opening, but no portal owned by 22 or 23 covers it. The root MOGI flags are `0x80002000` for group 12 and `0x00002000` for groups 14, 15, 18, 22 and 23. None satisfies the verified `0x20010008` independent-admission mask. For this captured camera and a single seed, WebWowViewerCpp's source traversal also predicts only 22 and 23; it was not run at this pose.

## Remaining evidence

Retail 70688's camera setup can seed multiple groups from two WMO-instance query slots. The spatial selection and near-plane rules are not established. The final background color behind this unportaled opening is also not established. Do not treat the all-group pass as admission for arbitrary interior groups or use this view alone to specify a sky fix. A Retail capture at the same location and view is still needed.

The dungeon census and group-level evidence are local diagnostics at `/home/osso/.worktrees/dungeon-walls-run/`. Sampled outdoor or unclassified poses do not prove interior coverage. `census-second.log` records runtime group mesh visibility; `camera-classification.json` records a BSP-based candidate analysis. These are not GPU draw-call records.

## Sources

- Retail 70688 static analysis: `/home/osso/.local/state/wow-ui-sim/handoff/native-evidence-70688/R-wmo-interior-visibility.md`, SHA-256 `2437205dc92c7e18b400525d9832dc0ff9b6855a50fbc63d9cf6597b1f7e636c`. Its scope is partial and does not validate this asset or run Retail.
- WebWowViewerCpp `wowViewerLib/src/engine/objects/wmo/wmoObject.cpp:933-1000,1343-1407` and `wmoGroupObject.cpp:317-445,561-650` — reference traversal and group query.
- `godot/rust/src/wmo/portals.rs` — engine group admission and portal traversal.

## See Also

- [[wmo-format]] — group, portal and collision records.
- [[stockade-entrance]] — camera classification and culled WMO geometry.
