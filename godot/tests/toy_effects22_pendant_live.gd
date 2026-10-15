extends "res://tests/toy_effects21_area_live.gd"
# Actual source115503 learning/use. Same-map source Ashran7332 is outside group4031.
# Travel edits position only; no cast, aura, cooldown or clock injection.
const PENDANT := 115503
const MOLTEN_PATH := 171352
const MAGMA_MODELS := [937030]
# TDB395482/80769 friendly Karabor Peacekeeper; ADT843283 Area7078→6719.
const ALLOWED := Vector3(1867.41, 78.231, -260.26)
const OUTSIDE := Vector3(3594.38, 19.8726, 3928.24)

func proof_cases() -> Array:
	return [[PENDANT, MOLTEN_PATH, MAGMA_MODELS, "Blazing Diamond Pendant"]]

func prove_toy(sample: Array) -> void:
	await learn_toy(PENDANT)
	if failed or not learned_toy(PENDANT): return
	if int(client.account_state().world_map_id) != 1116: fail("source Draenor login required"); return
	await move_on_source_map(OUTSIDE)
	if failed: return
	if not await wait_until(func(): return ready_at_source_site(OUTSIDE, 7332), "Ashran actual area and native HUD ready", 180000): return
	await open_source_toy(str(sample[3]))
	if failed: return
	await click("ToySpellButton1")
	if not await wait_until(func(): return client.toybox_state().error == "Requires area group4031", "Pendant outside-region refusal"): return
	if has_aura(MOLTEN_PATH) or remaining_cooldown(PENDANT) > 0.0: fail("refusal spent Pendant recovery"); return
	await capture("pendant22-outside-refused")
	await click("CollectionsMicroButton")
	await move_on_source_map(ALLOWED)
	if failed: return
	if not await wait_until(func(): return ready_at_source_site(ALLOWED, 7078), "Draenor actual parent area and native HUD ready", 180000): return
	await open_source_toy(str(sample[3]))
	if failed: return
	var errors_before: int = client.spells_state().errors.size()
	await click("ToySpellButton1")
	if not await wait_until(func(): return int(client.spells_state().casting) == MOLTEN_PATH, "natural Pendant cast bar"): return
	if has_aura(MOLTEN_PATH) or kit_model(MOLTEN_PATH, MAGMA_MODELS) != 0: fail("aura/kit started before cast completion"); return
	await move_on_source_map(OUTSIDE)
	if failed: return
	if not await wait_until(func(): return int(client.spells_state().casting) != MOLTEN_PATH and client.spells_state().errors.size() > errors_before, "completion area refusal", 15000): return
	var errors = client.spells_state().errors
	if str(errors[errors.size() - 1]) != "Requires area group4031": fail("wrong completion failure"); return
	if has_aura(MOLTEN_PATH) or kit_model(MOLTEN_PATH, MAGMA_MODELS) != 0 or remaining_cooldown(PENDANT) > 0.0: fail("completion rejection spent recovery or applied aura"); return
	receipts.append({"phase": "completion-refused", "position": observed_position(), "spells": client.spells_state(), "toys": client.toybox_state()})
	await capture("pendant22-completion-refused")
	await move_on_source_map(ALLOWED)
	if failed: return
	var started := Time.get_ticks_msec()
	await click("ToySpellButton1")
	if not await wait_until(func(): return int(client.spells_state().casting) == MOLTEN_PATH, "second natural five-second cast"): return
	await create_timer(2.0).timeout
	if has_aura(MOLTEN_PATH) or kit_model(MOLTEN_PATH, MAGMA_MODELS) != 0: fail("Molten Path applied early"); return
	if not await wait_until(func(): return has_aura(MOLTEN_PATH) and kit_model(MOLTEN_PATH, MAGMA_MODELS) == 937030, "source ground magma", 30000): return
	var elapsed := Time.get_ticks_msec() - started
	if elapsed < 4750: fail("source five-second cast shortened"); return
	if not native_display(-1): fail("magma replaced native player display"); return
	var active := Time.get_ticks_msec()
	var before := remaining_cooldown(PENDANT)
	receipts.append({"phase": "pendant-active", "position": observed_position(), "cast_elapsed_ms": elapsed, "source_cast_ms": 5000, "source_aura_ms": 120000, "model": kit_model(MOLTEN_PATH, MAGMA_MODELS), "cooldown": before})
	await click("CollectionsMicroButton")
	client.set_camera_orbit(PI / 2.0, -0.4, 7.0)
	await capture("pendant22-source-magma")
	if OS.get_environment("TOYFX_PENDANT_END") == "natural":
		await prove_natural_expiry(active)
	else:
		await prove_area_exit(before)
	if failed: return
	print("PASS toy-effects14 actual Pendant source cast/completion/model/lifecycle ", skin, " ", OS.get_environment("TOYFX_PENDANT_END"))

func prove_area_exit(before: float) -> void:
	await move_on_source_map(OUTSIDE)
	if failed: return
	if not await wait_until(func(): return not has_aura(MOLTEN_PATH) and kit_model(MOLTEN_PATH, MAGMA_MODELS) == 0, "Ashran interrupted source magma"): return
	var after := remaining_cooldown(PENDANT)
	if after <= 0.0 or after > before: fail("area exit reset source cooldown"); return
	if not native_display(-1): fail("area exit lost native model"); return
	receipts.append({"phase": "pendant-area-exit", "position": observed_position(), "cooldown_before": before, "cooldown_after": after})
	await capture("pendant22-ashran-magma-removed")

func prove_natural_expiry(active: int) -> void:
	await create_timer(110.0).timeout
	if not has_aura(MOLTEN_PATH) or kit_model(MOLTEN_PATH, MAGMA_MODELS) != 937030: fail("source magma expired before110s"); return
	if not await wait_until(func(): return not has_aura(MOLTEN_PATH) and kit_model(MOLTEN_PATH, MAGMA_MODELS) == 0, "natural120s aura/kit expiry", 180000): return
	var elapsed := Time.get_ticks_msec() - active
	if elapsed < 115000 or elapsed > 300000: fail("unexpected natural magma lifetime"); return
	if remaining_cooldown(PENDANT) <= 0.0 or not native_display(-1): fail("natural expiry lost cooldown/native model"); return
	receipts.append({"phase": "pendant-natural-expiry", "elapsed_from_model_ms": elapsed, "position": observed_position(), "cooldown": remaining_cooldown(PENDANT)})
	await capture("pendant22-natural-magma-expired")

func move_on_source_map(point: Vector3) -> void:
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, str(point.x), str(-point.z), str(point.y)]): return
	if not await wait_until(func(): return at_source_site(point), "actual same-map source movement", 30000): return

func at_source_site(point: Vector3) -> bool:
	var local_id = client.account_state().local_player_id
	if local_id == null: return false
	var transform = client.unit_transform(int(local_id))
	if transform == null: return false
	var delta: Vector3 = transform.origin - point
	return Vector2(delta.x, delta.z).length() < 1.0

func ready_at_source_site(point: Vector3, area: int) -> bool:
	if not at_source_site(point): return false
	var state = client.account_state()
	if state.area_id == null: return false
	if state.screen != "InWorld" or int(state.area_id) != area: return false
	return control("CollectionsMicroButton") != null and native_display(-1)

func fail(message: String) -> void:
	if client != null:
		var controls := []
		for node in client.find_children("*", "Control", true, false):
			if node.is_visible_in_tree(): controls.append(str(node.name))
		receipts.append({"phase": "failure", "message": message, "account": client.account_state(), "spells": client.spells_state(), "toys": client.toybox_state(), "auras": client.aura_state(), "visuals": client.spell_visuals_state(), "visible_controls": controls})
		write_receipts()
	super.fail(message)

func observed_position() -> Array:
	var point: Vector3 = client.unit_transform(int(client.account_state().local_player_id)).origin
	return [point.x, point.y, point.z]
