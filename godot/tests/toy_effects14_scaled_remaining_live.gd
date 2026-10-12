extends "res://tests/toy_effects14_scaled_live.gd"
# Separate accounts' one-hour Sealamp cooldown from the remaining source identities.
func proof_cases() -> Array:
	return [
		[238852, 1227383, 127931, "Flame's Radiance Banner", 0.75, 241494, false],
		[246227, 1240408, 131280, "Lightning-Blessed Spire", 0.5, 246171, true],
		[276229, 1299034, 145135, "Preyhunter's Trophy Stand", 1.5, 267634, false],
	]
