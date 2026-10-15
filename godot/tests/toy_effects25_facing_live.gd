extends "res://tests/toy_effects21_area_live.gd"
# Source-facing acceptance through rendered input and private5911 only.
# Travel changes position; no cast, target, facing, aura or cooldown injection.
var selected_unit := 0

func proof_cases() -> Array:
	return [[141300, 225826, [], "Magi Focusing Crystal"], [204818, 406870, [], "Mallard Mortar"]]

func prove_toy(sample: Array) -> void:
	var item := int(sample[0])
	var spell := int(sample[1])
	await learn_toy(item)
	if failed or not learned_toy(item): return
	if not await select_authored_hostile(): return
	await open_source_toy(str(sample[3]))
	if failed: return
	if remaining_cooldown(item) > 0.0: fail("source recovery already active before facing proof"); return
	await position_relative_to_target(10.0)
	if failed: return
	await record_facing("before-rear-start", item, spell)
	await click("ToySpellButton1")
	if not await wait_until(func(): return client.toybox_state().error == "NotInFront", "rear start refusal"): return
	if int(client.spells_state().casting) == spell or remaining_cooldown(item) > 0.0: fail("rear refusal started cast or spent recovery"); return
	await record_facing("rear-start-refused", item, spell)
	await capture("facing25-toy%d-start-refused" % item)
	await position_relative_to_target(-10.0)
	if failed: return
	var errors_before: int = client.spells_state().errors.size()
	await click("ToySpellButton1")
	if not await wait_until(func(): return int(client.spells_state().casting) == spell, "natural source cast"): return
	await position_relative_to_target(10.0)
	if failed: return
	if not await wait_until(func(): return int(client.spells_state().casting) != spell and client.spells_state().errors.size() > errors_before, "completion facing refusal", 15000): return
	var errors = client.spells_state().errors
	if str(errors[errors.size() - 1]) != "Target needs to be in front of you.": fail("wrong completion refusal: " + str(errors)); return
	if remaining_cooldown(item) > 0.0: fail("completion refusal spent item recovery"); return
	await record_facing("completion-refused", item, spell)
	await capture("facing25-toy%d-completion-refused" % item)
	# Crystal has no explicit-target minimum. Mortar Range503 has min8/max30.
	await position_relative_to_target(1.0 if item == 141300 else -10.0)
	if failed: return
	await click("ToySpellButton1")
	if not await wait_until(func(): return has_source_go(spell) and remaining_cooldown(item) > 0.0, "accepted source use after refusals", 20000): return
	await record_facing("near-rear-exempt" if item == 141300 else "front-permitted", item, spell)
	await capture("facing25-toy%d-permitted" % item)
	await click("CollectionsMicroButton")
	print("FACING25 root proof", spell, " ", skin)

func select_authored_hostile() -> bool:
	# TDB1210 source20279834/42937;20yd north, inside normal interest range.
	if not await move_private(Vector3(-8919.74, 73.4971, 335.934)): return false
	if not await wait_until(func(): return not client.find_children("Blackrock Invader", "Node3D", true, false).is_empty(), "source42937 interest readiness", 30000): return false
	var source: Node3D
	for node in client.find_children("Blackrock Invader", "Node3D", true, false):
		if node.get_node_or_null("NpcVisualRoot") != null:
			source = node
			break
	if source == null: fail("source Blackrock Invader42937 missing"); return false
	var player_transform = client.unit_transform(int(client.account_state().local_player_id))
	var point: Vector3 = source.position - player_transform.basis.z.normalized() * 20.0
	point.y = source.position.y
	if not await move_private(point): return false
	for attempt in range(40):
		await tap(KEY_TAB)
		var target = client.target_state().target
		if target == null: continue
		var rules: Dictionary = client.nameplate_rules(int(target))
		if str(rules.get("reaction", "")) == "Hostile" and bool(rules.get("alive", false)):
			selected_unit = int(target)
			return await wait_until(func(): return client.target_state().server_target == selected_unit, "authoritative selected unit")
	fail("source hostile selection failed")
	return false

func position_relative_to_target(distance: float) -> void:
	var target = client.unit_transform(selected_unit)
	var player = client.unit_transform(int(client.account_state().local_player_id))
	if target == null or player == null: fail("selected transform absent"); return
	var forward: Vector3 = player.basis.z.normalized()
	forward.y = 0.0
	var point: Vector3 = target.origin + forward.normalized() * distance
	point.y = target.origin.y
	await move_private(point)

func move_private(point: Vector3) -> bool:
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(point.x), str(-point.z), str(point.y)]): return false
	return await wait_until(func():
		var observed = client.account_state().local_server_position
		return observed != null and Vector2(observed.x - point.x, observed.z - point.z).length() < 0.5,
		"authoritative private position", 10000)

func has_source_go(spell: int) -> bool:
	for cast in client.spell_visuals_state().casts:
		if int(cast.spell) == spell and bool(cast.go): return true
	return false

func record_facing(label: String, item: int, spell: int) -> void:
	var player = client.unit_transform(int(client.account_state().local_player_id))
	var target = client.unit_transform(selected_unit)
	var offset: Vector3 = target.origin - player.origin
	receipts.append({"phase": label, "item": item, "spell": spell, "target": client.target_state(),
		"distance3d": offset.length(), "forward_dot": player.basis.z.normalized().dot(offset.normalized()),
		"cooldown": remaining_cooldown(item), "spells": client.spells_state(), "toys": client.toybox_state()})
	write_receipts()

func fail(message: String) -> void:
	if client != null and selected_unit != 0:
		await record_facing("failure", 0, 0)
	super.fail(message)
