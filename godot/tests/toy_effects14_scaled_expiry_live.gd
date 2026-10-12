extends "res://tests/toy_effects14_scaled_remaining_live.gd"
# Actual journal activation and natural source expiry; no server-clock manipulation.
func proof_cases() -> Array:
	return [
		[238852, 1227383, 127931, "Flame's Radiance Banner", 0.75, 241494, false, 60000],
		[276229, 1299034, 145135, "Preyhunter's Trophy Stand", 1.5, 267634, false, 120000],
	]

func prove_toy(sample: Array) -> void:
	await super.prove_toy(sample)
	if failed: return
	var activated := activation_ticks(int(sample[0]))
	var duration := int(sample[7])
	var halfway := activated + duration / 2
	while Time.get_ticks_msec() < halfway:
		if scenery(str(sample[3])) == null: fail("source scenery expired before half duration"); return
		await create_timer(0.5).timeout
	if not await wait_until(func(): return scenery(str(sample[3])) == null, "natural source scenery expiry", duration + 20000): return
	var elapsed := Time.get_ticks_msec() - activated
	if elapsed < duration - 10000 or elapsed > duration + 20000:
		fail("scenery expiry outside source duration observation window"); return
	receipts.append({"phase": "scaled-natural-expiry", "item": sample[0], "source_duration_ms": duration, "elapsed_ms": elapsed})
	write_receipts()

func activation_ticks(item: int) -> int:
	for receipt in receipts:
		if receipt.get("phase") == "scaled-activation" and int(receipt.item) == item:
			return int(receipt.ticks_msec)
	fail("missing observed activation time")
	return 0
