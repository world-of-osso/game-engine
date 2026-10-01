extends RefCounted

## TEST-FIRST extraction only: preserve the fixture's existing object-only predicate.
## Terrain and unit-visual readiness must remain unfixed until MAIN observes RED.
static func is_ready(state: Dictionary) -> bool:
	return state.world_objects.pending == 0
