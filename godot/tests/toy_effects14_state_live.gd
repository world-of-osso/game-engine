extends "res://tests/toy_effects14_scaled_remaining_live.gd"
# Source CDI state kits stay held until their real stationary actor expires.
func proof_cases() -> Array:
	return [[246227, 1240408, 131280, "Lightning-Blessed Spire", 0.5, 246171, true, 236744, 1952826, 0.25]]

func prove_toy(sample: Array) -> void:
	await super.prove_toy(sample)
	if failed: return
	var kit_id := int(sample[7])
	var model_id := int(sample[8])
	if not await wait_until(func(): return state_model(sample) != null, "source CDI state kit%d/model%d" % [kit_id, model_id], 30000): return
	var effect := state_model(sample)
	if abs(effect.scale.x - float(sample[9])) > 0.00001: fail("state kit attachment scale lost"); return
	var unit := scenery(str(sample[3]))
	if effect.global_position.distance_to(unit.global_position) > 0.01: fail("GROUND19 did not use actual unit origin"); return
	await create_timer(2.0).timeout
	var starts := 0
	for kit in client.spell_visuals_state().started:
		if int(kit.kit) == kit_id: starts += 1
	if starts != 1: fail("state kit restarted across native frames"); return
	receipts.append({"phase": "source-state-kit", "item": sample[0], "kit": kit_id, "model": model_id, "starts": starts, "attachment_scale": effect.scale.x})
	await capture("toy%d-state-kit%d" % [sample[0], kit_id])
	# Source duration180000ms: observe natural server expiry, not the calendar clock.
	if not await wait_until(func(): return scenery(str(sample[3])) == null and state_model(sample) == null, "source actor and held state kit expiry", 200000): return
	receipts.append({"phase": "source-state-kit-expired", "kit": kit_id, "model": model_id})
	write_receipts()

func state_model(sample: Array) -> Node3D:
	var unit := scenery(str(sample[3]))
	if unit == null: return null
	var effects := unit.find_children("SpellEffect%d" % int(sample[8]), "Node3D", true, false)
	if effects.is_empty(): return null
	return effects[0]
