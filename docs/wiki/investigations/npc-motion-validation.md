# Northshire NPC Motion and Terrain Validation

Bounded validation at engine revision `9404235d`, verified 2026-09-09. This records observed fixes and their evidence; it does not claim complete cold-ring streaming, NPC locomotion-state generation, WMO vertical floors, or a performance improvement.

## Observed Native Behavior

- Native tree comparison found 422 changed NPC bone transforms, supporting actual authored NPC idle playback.
- A local player walk covered 20.33 units. Endpoint height deltas against the terrain sampler were 0.0016 and 0.0413 units, supporting grounded endpoint agreement.
- Initial native streaming reported `load_radius: 1`, nine loaded tiles, nine heightmap tiles, zero pending tiles, and zero failed tiles. `streaming-fixed.webp` shows the observed surrounding terrain without the prior nearby tile-edge void.
- Cold-edge capture extracted two root ADTs and their declared companions from local CASC; one tile fully spawned before the 30-second cap. No attempt for the third tile was observed before the cap, which is not an extraction failure.

## Verified Fixes

- Authored NPC facing retains server orientation and applies the client M2 basis offset; actual-model idle playback changed 422 sampled NPC bones.
- `JumpLandRun` no longer restarts jump/free-fall; 20 focused movement and animation tests pass.
- M2 effect fog compiles and renders in headless GPU fog-off and fog-on variants without the reported binding/argument error.
- World picking uses `Camera3d`; two production raycast tests cover an NPC child mesh with the UI camera and preserve intentional `--no-ui` health-bar suppression.
- WMO local conversion removes the extra 180° basis error; all seven authored WMO/MODF bounds comparisons pass.
- Terrain keeps the radius-one 3×3 neighborhood and resolves official ADT roots plus declared companions through the local CASC cache.

## Final wolf/equipment follow-up

`data/diagnostics/wolf-nameplate-equipment-20260909/final-proof.md` records a later bounded integration at engine `9d22c7fb`. Native UI capture shows a named Diseased Timber Wolf through the overlay path and Theron's shirt, pants, and boots. Theron's five physical items persisted across a server restart: GUIDs 10–14 for sword, shirt, pants, boots, and shield. The rear camera cannot independently distinguish the sword or shield; their attachment proof is the real Loading→InWorld regression, which parents their model roots to HumanHD bones 201 and 206. The prior cause was an absolute-path guard that skipped existing SKA1 skeleton attachments, not missing HumanHD assets.

The native capture does not establish long-run wolf variation frequencies. Exact authored weighted selection and terminal recovery remain test evidence. Both earlier and final captures contain the same HUD rectangles; no toolkit regression or HUD-health causal claim follows from this evidence.

## Boundaries

NPC walking/running remains unproven because no replicated NPC `MovementState` producer exists. WMO collision still has no vertical floor support. The cold capture does not prove every tile in a cold ring completes within the cap. The deferred `resolve_*` naming note reflects cache-writing I/O and is maintainability-only, not a functional failure. Later shared-checkout shader-clock changes are outside this revision and this evidence.

## Sources

- `../../../data/diagnostics/npc-motion-20260909/final-verification/report.md` — original six-issue proof and native observations
- `../../../data/diagnostics/npc-motion-20260909/followup-verification/report.md` — revision-pinned resolver, retention, picking, and GPU verification
- `../../../data/diagnostics/npc-motion-20260909/native-motion-proof.json` — bone-motion and grounded-walk samples
- `../../../data/diagnostics/npc-motion-20260909/streaming-fixed-terrain.txt` — initial nine-tile streaming state
- `../../../data/diagnostics/npc-motion-20260909/cold-edge-run.txt` — bounded cold local-CASC capture
- `../../../data/diagnostics/wolf-nameplate-equipment-20260909/final-proof.md` — later bounded nameplate, equipment, and weighted-animation evidence

## See Also

- [[animation]] — NPC attachment and landing behavior
- [[terrain]] — streaming neighborhood and ADT companion loading
- [[ui-system]] — world-camera selection and `--no-ui` policy
- [[wmo-format]] — WMO placement basis
