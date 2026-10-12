extends "res://tests/toy_effects14_visual_live.gd"
# Actual learned UseToy; source friendly-area aura and native recipient attachment.
func proof_cases() -> Array:
	return [[141301, 225828, 1384408, "Unstable Powder Box"]]

func capture(label: String) -> void:
	await super.capture("friendly-area-" + label)
