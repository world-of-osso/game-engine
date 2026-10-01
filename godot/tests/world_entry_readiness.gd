extends RefCounted

## All currently requested work must drain; an empty object queue alone is transient.
static func is_ready(state: Dictionary) -> bool:
	if state.terrain.pending_count != 0:
		return false
	return state.world_objects.pending == 0 and state.unit_visuals_pending == 0
