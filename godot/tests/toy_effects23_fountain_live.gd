extends "res://tests/toy_effects21_area_live.gd"
# Actual source64361 learning/use. No aura/cooldown/clock injection or reset.
const FOUNTAIN_ITEM := 64361
const FOUNTAIN_SPELL := 91743
const FOUNTAIN_MODELS := [253570]

func proof_cases() -> Array:
	return [[FOUNTAIN_ITEM, FOUNTAIN_SPELL, FOUNTAIN_MODELS, "Druid and Priest Statue Set"]]

func prove_toy(sample: Array) -> void:
	await learn_toy(FOUNTAIN_ITEM)
	if failed or not learned_toy(FOUNTAIN_ITEM): return
	await open_source_toy(str(sample[3]))
	if failed: return
	var state = client.account_state()
	var health: float = state.local_player_health
	var position: Vector3 = client.unit_transform(int(state.local_player_id)).origin
	var clicked := Time.get_ticks_msec()
	await click("ToySpellButton1")
	if not await wait_until(func(): return has_aura(FOUNTAIN_SPELL) and kit_model(FOUNTAIN_SPELL, FOUNTAIN_MODELS) == 253570, "source Fountain ground model", 15000): return
	if not native_display(-1): fail("Fountain replaced native player model"); return
	var active := Time.get_ticks_msec()
	var cooldown := remaining_cooldown(FOUNTAIN_ITEM)
	if cooldown <= 0.0: fail("source item recovery absent"); return
	receipts.append({"phase": "fountain-active", "from_click_ms": active - clicked, "model": 253570, "source_model_scale": 0.5, "source_duration_ms": 10000, "cooldown": cooldown})
	await click("CollectionsMicroButton")
	client.set_camera_orbit(PI / 2.0, -0.4, 7.0)
	await capture("fountain23-source-model")
	if not await wait_until(func(): return not has_aura(FOUNTAIN_SPELL) and kit_model(FOUNTAIN_SPELL, FOUNTAIN_MODELS) == 0, "natural Fountain aura/kit expiry", 60000): return
	var after := remaining_cooldown(FOUNTAIN_ITEM)
	if after <= 0.0 or after > cooldown: fail("natural expiry reset recovery"); return
	if not native_display(-1): fail("natural expiry lost player model"); return
	if float(client.account_state().local_player_health) != health: fail("Fountain changed player health"); return
	if client.unit_transform(int(client.account_state().local_player_id)).origin.distance_to(position) > 0.1: fail("Fountain moved player"); return
	receipts.append({"phase": "fountain-natural-expiry", "from_model_ms": Time.get_ticks_msec() - active, "cooldown": after})
	await capture("fountain23-natural-expiry")
	print("PASS toy-effects14 actual Fountain source model/natural expiry/recovery ", skin)
