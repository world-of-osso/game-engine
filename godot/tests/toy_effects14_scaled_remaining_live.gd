extends "res://tests/toy_effects14_scaled_live.gd"
# Keep the one-hour Sealamp cooldown separate from the remaining source identities.
func proof_cases() -> Array:
	return [
		[238852, 1227383, 127931, "Flame's Radiance Banner", 0.75, 241494, false],
		[246227, 1240408, 131280, "Lightning-Blessed Spire", 0.5, 246171, true],
		[276229, 1299034, 145135, "Preyhunter's Trophy Stand", 1.5, 267634, false],
	]

func prove_toy(sample: Array) -> void:
	# The preceding enemy-target proof can leave the private fixture character dead.
	# Use the real admin resurrection contract, never change toy alive/cast gates.
	if client.account_state().local_player_health <= 0.0:
		if not command(OS.get_environment("TOYFX_ADMIN"), ["revive", character]): return
		if not await wait_until(func(): return client.account_state().local_player_health > 0.0, "private fixture resurrection"): return
	await super.prove_toy(sample)
