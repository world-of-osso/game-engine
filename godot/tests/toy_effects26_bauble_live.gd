extends "res://tests/toy_effects21_area_live.gd"
# Actual source140309 learning/use. No aura/cooldown/clock injection or reset.
const BAUBLE_ITEM := 140309
const BAUBLE_SPELL := 223143
const BAUBLE_MODELS := [1417024]

func proof_cases() -> Array:
	return [[BAUBLE_ITEM, BAUBLE_SPELL, BAUBLE_MODELS, "Prismatic Bauble"]]

func prove_toy(sample: Array) -> void:
	await learn_toy(BAUBLE_ITEM)
	if failed or not learned_toy(BAUBLE_ITEM): return
	await open_source_toy(str(sample[3]))
	if failed: return
	var state = client.account_state()
	var health: float = state.local_player_health
	var position: Vector3 = client.unit_transform(int(state.local_player_id)).origin
	var clicked := Time.get_ticks_msec()
	await click("ToySpellButton1")
	if not await wait_until(func(): return has_aura(BAUBLE_SPELL) and kit_model(BAUBLE_SPELL, BAUBLE_MODELS) == 1417024 and remaining_cooldown(BAUBLE_ITEM) > 0.0, "source Bauble attachment and independent recovery receipt", 15000): return
	if not native_display(-1): fail("Bauble replaced native player model"); return
	var active := Time.get_ticks_msec()
	var cooldown := remaining_cooldown(BAUBLE_ITEM)
	if cooldown <= 0.0: fail("source item recovery absent"); return
	receipts.append({"phase": "bauble-active", "from_click_ms": active - clicked, "model": 1417024, "source_model_scale": 1.0, "source_duration_ms": 20000, "cooldown": cooldown})
	await click("CollectionsMicroButton")
	client.set_camera_orbit(PI / 2.0, -0.4, 7.0)
	await capture("bauble26-source-model")
	if not await wait_until(func(): return not has_aura(BAUBLE_SPELL) and kit_model(BAUBLE_SPELL, BAUBLE_MODELS) == 0, "natural Bauble aura/kit expiry", 60000): return
	var after := remaining_cooldown(BAUBLE_ITEM)
	if after <= 0.0 or after > cooldown: fail("natural expiry reset recovery"); return
	if not native_display(-1): fail("natural expiry lost player model"); return
	if float(client.account_state().local_player_health) != health: fail("Bauble changed player health"); return
	if client.unit_transform(int(client.account_state().local_player_id)).origin.distance_to(position) > 0.1: fail("Bauble moved player"); return
	receipts.append({"phase": "bauble-natural-expiry", "from_model_ms": Time.get_ticks_msec() - active, "cooldown": after})
	await capture("bauble26-natural-expiry")
	print("PASS toy-effects14 actual Bauble source model/natural expiry/recovery ", skin)
