extends "res://tests/toy_effects14_signs_framed_live.gd"
# Real cursor placement and source CDI state models. Server fixture proves
# 180s simulation lifetime; native wall timing is observed, not reinterpreted.
const STATE_KIT := 234717
const STATE_MODELS := [[2339771, 0.2], [3023279, 0.15]]

func proof_cases() -> Array:
	return [[245942, 1240082, 129958, "Sea-Blessed Shrine", 1.0, 246037, true, 180000]]

func capture(label: String) -> void:
	if label == "native":
		await super.capture(label)
		return
	for effect in STATE_MODELS:
		var fdid := int(effect[0])
		if not await wait_until(func(): return state_model(fdid) != null, "source shrine state model%d" % fdid, 30000): return
		var model := state_model(fdid)
		if abs(model.scale.x - float(effect[1])) > 0.00001: fail("shrine source state scale lost"); return
		var unit := scenery("Sea-Blessed Shrine")
		if model.global_position.distance_to(unit.global_position) > 0.01: fail("shrine ground attachment moved from source origin"); return
		receipts.append({"phase": "shrine-state-model", "model": fdid, "scale": model.scale.x, "origin": str(model.global_position)})
	var starts := 0
	for kit in client.spell_visuals_state().started:
		if int(kit.kit) == STATE_KIT and int(kit.spell) == 0: starts += 1
	if starts != 1: fail("shrine display state kit did not start exactly once"); return
	await super.capture(label)

func prove_toy(sample: Array) -> void:
	await super.prove_toy(sample)
	if failed: return
	for effect in STATE_MODELS:
		if not client.find_children("SpellEffect%d" % int(effect[0]), "Node3D", true, false).is_empty(): fail("shrine state model survived actor expiry"); return
	receipts.append({"phase": "shrine-state-removed", "kit": STATE_KIT})
	write_receipts()

func state_model(fdid: int) -> Node3D:
	var unit := scenery("Sea-Blessed Shrine")
	if unit == null: return null
	var models := unit.find_children("SpellEffect%d" % fdid, "Node3D", true, false)
	return null if models.is_empty() else models[0]
