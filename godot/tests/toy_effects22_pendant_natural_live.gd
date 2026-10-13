extends "res://tests/toy_effects22_pendant_live.gd"
# Dedicated expiry proof. Completion/area-exit proof remains in the parent fixture.
# No position changes, cancellation, relogging or cooldown edits during the aura.

func prove_toy(sample: Array) -> void:
	if not learned_toy(PENDANT):
		fail("expiry account must already have physically learned Pendant")
		return
	if int(client.account_state().world_map_id) != 1116:
		fail("source Draenor login required")
		return
	if remaining_cooldown(PENDANT) > 0.0:
		fail("source cooldown has not naturally elapsed")
		return
	await move_on_source_map(ALLOWED)
	if failed: return
	if not await wait_until(func(): return ready_at_source_site(ALLOWED, 7078), "Lunarfall actual area and native HUD ready", 180000): return
	await open_source_toy(str(sample[3]))
	if failed: return
	var started := Time.get_ticks_msec()
	await click("ToySpellButton1")
	if not await wait_until(func(): return int(client.spells_state().casting) == MOLTEN_PATH, "natural five-second Pendant cast"): return
	await create_timer(2.0).timeout
	if has_aura(MOLTEN_PATH) or kit_model(MOLTEN_PATH, MAGMA_MODELS) != 0:
		fail("Molten Path applied before cast completion")
		return
	if not await wait_until(func(): return has_aura(MOLTEN_PATH) and kit_model(MOLTEN_PATH, MAGMA_MODELS) == 937030, "source ground magma", 30000): return
	var active := Time.get_ticks_msec()
	if active - started < 4750:
		fail("source five-second cast shortened")
		return
	receipts.append({"phase": "pendant-active", "cast_elapsed_ms": active - started, "position": observed_position(), "source_aura_ms": 120000, "model": kit_model(MOLTEN_PATH, MAGMA_MODELS), "cooldown": remaining_cooldown(PENDANT)})
	await click("CollectionsMicroButton")
	client.set_camera_orbit(PI / 2.0, -0.4, 7.0)
	await capture("pendant22-source-magma")
	await prove_natural_expiry(active)
	if failed: return
	print("PASS toy-effects14 actual Pendant source cast/model/natural lifecycle ", skin)
