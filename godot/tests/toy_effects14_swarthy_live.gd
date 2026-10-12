extends "res://tests/toy_effects14_scaled_remaining_live.gd"
# Source Swarthy CDI117639 state kit195897, both authored root-attached effects.
const SOURCE_KIT := 195897
const EFFECTS := [[5581612, 0.5, 3.3499999], [929379, 0.525, 1.0]]

func proof_cases() -> Array:
	return [[170197, 438546, 117639, "Swarthy Warning Sign", 0.6, 218978, false]]

func frame_scenery_camera() -> void:
	client.set_camera_orbit(-PI / 2.0, -0.45, 8.0)

func prove_toy(sample: Array) -> void:
	await super.prove_toy(sample)
	if failed: return
	var unit := scenery(str(sample[3]))
	for effect in EFFECTS:
		var fdid := int(effect[0])
		if not await wait_until(func(): return state_model(sample, fdid) != null, "source Swarthy state model%d" % fdid, 30000): return
		var model := state_model(sample, fdid)
		if abs(model.scale.x - float(effect[1])) > 0.00001: fail("Swarthy authored effective scale lost"); return
		var expected := unit.global_transform * Vector3(0.0, float(effect[2]), 0.0)
		if model.global_position.distance_to(expected) > 0.01: fail("Swarthy root attachment offset lost"); return
		receipts.append({"phase": "swarthy-state-model", "model": fdid, "local_scale": model.scale.x, "source_offset_z": effect[2], "observed": str(model.global_position), "expected": str(expected)})
	await create_timer(2.0).timeout
	var starts := 0
	for kit in client.spell_visuals_state().started:
		if int(kit.kit) == SOURCE_KIT and int(kit.spell) == 0: starts += 1
	if starts != 1: fail("Swarthy source display kit did not start exactly once"); return
	receipts.append({"phase": "swarthy-state-start", "kit": SOURCE_KIT, "starts": starts})
	await capture("toy170197-state-kit195897")
	if failed: return
	if not await wait_until(func(): return scenery(str(sample[3])) == null, "natural Swarthy60s expiry", 80000): return
	for effect in EFFECTS:
		var name := "SpellEffect%d" % int(effect[0])
		if not client.find_children(name, "Node3D", true, false).is_empty(): fail("Swarthy state model survived its actor"); return
	receipts.append({"phase": "swarthy-state-expired", "kit": SOURCE_KIT})
	write_receipts()

func state_model(sample: Array, fdid: int) -> Node3D:
	var unit := scenery(str(sample[3]))
	if unit == null: return null
	var effects := unit.find_children("SpellEffect%d" % fdid, "Node3D", true, false)
	if effects.is_empty(): return null
	return effects[0]
