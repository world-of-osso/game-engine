# Native gathering

Verified: 2026-10-11, gathering-nodes branch; not merged or live-accepted. The [native contract](../../specs/gathering.md) owns UI requirements; [server gathering](../../../../game-server/docs/wiki/systems/gathering.md) owns metadata, loot, skills and lifecycle.

## Native paths

The replica codec decodes `GatherableResource.parent_skill`. GO50 passes the existing M2 rendering/picking path. Right-click, soft interaction and IPC quest-interact send existing `UseGameObject` for resources; normal cast bars and loot UI show server outcomes.

Minimap tracking adds Find Herbs/Find Minerals only when profession snapshots know2383/2580. Selected available filters consume replicated parent182/186 resources and clip with the existing map mask. Skinning corpses do not enter herb/ore tracking. Art comes from Retail atlas647 members17635/17636; both skins use the same semantic filters.

## Evidence and limits

Eleven minimap tests passed at UI code5eacf1e0e, including two gathering tests for filter eligibility, clipping, both-skin registry/menu and authored art. Matching extension and CLI built successfully; later fixture-only changes do not invalidate that build. Copper model219514, skin509861 and texture219513 were extracted locally into canonical data, but file presence is not verified-receipt/render proof.

`gathering_live.gd` uses real private realm5720/fb_gathering_nodes, authentic resource/creature positions, native input, cast state and viewport PNG capture. It is not accepted proof: attempt1 reached Modern InWorld but an obsolete micro-menu readiness check timed out; corrected attempt2 timed out at auth/roster after restart. No requested PNGs were produced or inspected. Independent artifact gate was unavailable (Claude OAuth expired). Further acceptance needs a fresh live run or explicit retry-cap override, not a fabricated capture.

## Sources

- [Tracking model](../../../godot/ui-model/src/minimap_tracking.rs) and [tests](../../../godot/ui-model/tests/minimap_gathering.rs).
- [Native tracking](../../../godot/rust/src/minimap.rs), [interaction](../../../godot/rust/src/merchant.rs) and [GO rendering](../../../godot/rust/src/game_objects.rs).
- [Live fixture](../../../godot/tests/gathering_live.gd) — input and PNG automation; diagnostics retained in canonical data/diagnostics/gathering-nodes-2026-10-11.
- Local Retail `UiTextureAtlasMember.csv` and `Blizzard_Minimap/Mainline/Minimap.lua` — authored art and learned spell tracking.

## See Also

- [[minimap]] — shared cluster/mask/placement.
- [[professions-ui]] — owner snapshots.
- [[offline-asset-closure]] — verified payload/receipt boundary.
