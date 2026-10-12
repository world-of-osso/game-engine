extends "res://tests/toy_effects14_live.gd"
# Actual1973/16739, Human native male -> source10137; cancellation restores native body.
func proof_cases() -> Array:
	return [[1973, 16739, 10137, "Orb of Deception"]]

func capture(label: String) -> void:
	await super.capture("orb-" + label)
