extends "res://tests/toy_effects23_fountain_live.gd"
# Actual learning/use and stationary natural source300s expiry; no aura/CD/clock edits.
const TOME_ITEM := 127670
const TOME_SPELL := 187146
const TOME_MODELS := [1097945]

func proof_cases() -> Array:
	return [[TOME_ITEM, TOME_SPELL, TOME_MODELS, "Accursed Tome of the Sargerei"]]

func prove_toy(sample: Array) -> void:
	await learn_toy(TOME_ITEM)
	if failed or not learned_toy(TOME_ITEM): return
	await open_source_toy(str(sample[3]))
	if failed: return
	var state = client.account_state()
	var health: float = state.local_player_health
	var position: Vector3 = client.unit_transform(int(state.local_player_id)).origin
	await click("ToySpellButton1")
	if not await wait_until(func(): return has_aura(TOME_SPELL) and kit_model(TOME_SPELL, TOME_MODELS) == 1097945, "source Tome attachment", 30000): return
	if not native_display(-1): fail("Tome replaced player model"); return
	var active := Time.get_ticks_msec()
	var cooldown := remaining_cooldown(TOME_ITEM)
	if cooldown <= 0.0: fail("source recovery absent"); return
	receipts.append({"phase": "tome-active", "model": 1097945, "source_duration_ms": 300000, "cooldown": cooldown})
	await click("CollectionsMicroButton")
	client.set_camera_orbit(PI / 2.0, -0.3, 4.0)
	await capture("tome24-source-attachment")
	await create_timer(280.0).timeout
	if not has_aura(TOME_SPELL) or kit_model(TOME_SPELL, TOME_MODELS) != 1097945: fail("Tome ended before source lifetime boundary"); return
	if not await wait_until(func(): return not has_aura(TOME_SPELL) and kit_model(TOME_SPELL, TOME_MODELS) == 0, "natural Tome aura/kit expiry", 180000): return
	var after := remaining_cooldown(TOME_ITEM)
	if after <= 0.0 or after > cooldown: fail("natural removal reset recovery"); return
	if not native_display(-1): fail("natural removal lost original model"); return
	if float(client.account_state().local_player_health) != health: fail("Tome changed player health"); return
	if client.unit_transform(int(client.account_state().local_player_id)).origin.distance_to(position) > 0.1: fail("Tome moved player"); return
	receipts.append({"phase": "tome-natural-expiry", "from_model_ms": Time.get_ticks_msec() - active, "cooldown": after})
	await capture("tome24-natural-expiry")
	print("PASS toy-effects14 actual Tome source attachment/natural expiry/recovery ", skin)

func fail(message: String) -> void:
	if client != null:
		var controls := []
		for node in client.find_children("*", "Control", true, false):
			if node.is_visible_in_tree(): controls.append(str(node.name))
		receipts.append({"phase": "failure", "message": message, "account": client.account_state(), "spells": client.spells_state(), "toys": client.toybox_state(), "bags": client.merchant_state().bags, "auras": client.aura_state(), "visuals": client.spell_visuals_state(), "visible_controls": controls})
		write_receipts()
	super.fail(message)
