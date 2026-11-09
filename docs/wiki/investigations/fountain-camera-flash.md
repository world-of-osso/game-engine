# Fountain camera flash

The Stormwind Trade District fountain (model 366804, player pose -8855,645,97) changes between blue and blue-white water while the camera moves. The [contract](../../specs/m2-transparent-order.md) covers authored batch priority, not a new water shader.

## Evidence and cause — 2026-11-09

VERIFIED: the user's second Windows recording contains 788 frames in 13.131556 seconds. Analysis excludes the desktop: source game rectangle (760,154,1920,1080), sampled at 960×540, with every frame measured in a 32×18 grid. A tracked lower-basin mask brightens 27.723/255 at frame120 (PTS1.999900s), RGB delta +(23.385,28.624,31.575), for a three-frame approximately50ms threshold run. Frame436 (PTS7.265811s) darkens28.724/255 for two frames, approximately33ms. The earlier stationary evidence did not cover this condition.

VERIFIED: all eight native fountain materials have priority0 although raw skin priorities, in resolved batch order, are [-24,0,0,0,0,3,24,25]. Their custom AABBs are identical, so Godot receives equal depth keys for the overlapping sheets. Their water shaders use UV coordinates, not environment coordinates or edge fade; the main water combiner has no specular contribution.

VERIFIED: a headless Linux in-world yaw/pitch sweep on engine3fc8727cb with private serveradcb9dd81/protocol7fa253f, port5977, reproduces basin discontinuities. A/B camera transforms match exactly; the material clock and client simulation are frozen. At the unobstructed pitch-sweep event209→210, baseline basin luma changes+4.656/255; priority-only changes+0.041/255 and removes the visible sheet flip. Whole-path maxima are not water-only proof: some angles put intervening roofs in the measured region.

VERIFIED: an isolated production fountain with frozen materials, no AA/history, and128 zero-alpha transparent quads reproduces large flashes under the same240-pose camera path. The quads contribute no colour; they only perturb the transparent sort list. Baseline maximum basin luma step is29.168/255 (95th percentile17.431). Changing only authored material priority reduces these to0.290/255 (95th percentile0.212); disabling AABB-center sorting alone does not fix it (maximum28.471). Both changes equal priority-only; reverting priority reproduces baseline exactly. The isolated fountain without the surrounding transparent draws is stable (maximum0.400/255), which explains why a simple stationary or isolated-model test missed the failure.

INFERRED from these controlled raster experiments: missing priority binding leaves the overlapping sheets tied; camera-dependent ordering of the surrounding transparent draw list changes their composition. This is not a UV clock, specular, LOD, occlusion or temporal-history correction.

## Decided rules

Bind the resolved signed `priority_plane` directly to the Godot material's `render_priority` during construction. Preserve existing material sharing/copy behavior, shader code, geometry, AABBs and all other render state. Do not add per-frame sorting, fountain IDs, offsets, alternate paths or new configuration.

WebWowViewerCpp1a8cccbeffc46231c6497e6b3f5bfbf3507d8071's `SortMeshes` compares transparent priority planes ascending before distance; `M2Object::createM2Mesh` passes the skin priority into the mesh. Godot's signed priority field has the same ascending sense; existing native ribbons already bind it directly. Object-level and equal-plane distance sorting remain outside this bounded correction.

## Acceptance and proof status

The committed `m2_transparent_priority.gd` test loads real366804 geometry/textures and freezes material time at700ms. It compares rendered basin pixels at120 yaw/pitch poses with an independent raw-skin priority reference, including128 colourless transparent draws. It also requires visible blue water, so an empty render cannot pass. RED/GREEN and final gates are pending; no product fix is included in the initial acceptance commit.

## Sources

- Evidence/scripts/JSON: `/home/osso/.worktrees/game-engine-s2/data/diagnostics/fountain-flash-2-2026-11-09/` (`water-events.json`, `pts.json`, `near/`, `stress/`, `stress-analysis.json`). The initial eased-camera and distant-roof sweeps are exploratory only, not clean A/B proof.
- User-facing crop sheets: `/syncthing/AgentShared/2026-11-09/fountain-flash/`.
- Local WebWowViewerCpp1a8cccb: `wowViewerLib/src/gapi/interface/sortLambda.h`, `engine/objects/m2/m2Object.cpp`, `m2Helpers/M2MeshBufferUpdater.cpp`.
- Native material binding: `godot/rust/src/assets/material.rs`; model bounds: `godot/rust/src/assets/mod.rs`.

## See Also

- [[m2-format]] — authored blend and shader rules.
- [M2 transparent order contract](../../specs/m2-transparent-order.md).
