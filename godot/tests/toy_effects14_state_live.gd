extends "res://tests/toy_effects14_scaled_remaining_live.gd"
# CDI131280 holds StateSpellVisualKit236744/model1952826 until this source actor expires.
func proof_cases() -> Array:
	return [[246227, 1240408, 131280, "Lightning-Blessed Spire", 0.5, 246171, true]]

func prove_toy(sample: Array) -> void:
	await super.prove_toy(sample)
	if failed: return
	if not await wait_until(func(): return state_model() != null, "source CDI state kit236744/model1952826", 30000): return
	var effect := state_model()
	if abs(effect.scale.x - 0.25) > 0.00001: fail("state kit attachment scale lost"); return
	var unit := scenery(str(sample[3]))
	if effect.global_position.distance_to(unit.global_position) > 0.01: fail("GROUND19 did not use actual unit origin"); return
	await create_timer(2.0).timeout
	var starts := 0
	for kit in client.spell_visuals_state().started:
		if int(kit.kit) == 236744: starts += 1
	if starts != 1: fail("state kit restarted across native frames"); return
	receipts.append({"phase": "source-state-kit", "kit": 236744, "model": 1952826, "starts": starts, "attachment_scale": effect.scale.x})
	await capture("toy246227-state-kit236744")
	if not command(OS.get_environment("TOYFX_ADMIN"), ["clock-advance", "181"]): return
	if not await wait_until(func(): return scenery(str(sample[3])) == null and state_model() == null, "source actor and held state kit expiry", 30000): return
	receipts.append({"phase": "source-state-kit-expired", "kit": 236744, "model": 1952826})
	write_receipts()

func state_model() -> Node3D:
	var unit := scenery("Lightning-Blessed Spire")
	if unit == null: return null
	var effects := unit.find_children("SpellEffect1952826", "Node3D", true, false)
	if effects.is_empty(): return null
	return effects[0]
