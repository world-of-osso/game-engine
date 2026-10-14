extends "res://tests/toy_effects14_signs_framed_live.gd"
# Actual source owner activation/meshes/natural expiry. Group-audience proof is
# separate real-UDP peer evidence; this fixture never substitutes privacy UI.
var active_case: Array = []

func prove_toy(sample: Array) -> void:
	active_case = sample
	await super.prove_toy(sample)

func fail(message: String) -> void:
	if message == "scenery expiry outside source duration observation window" and not active_case.is_empty():
		receipts.append({"phase": "source-expiry-window-failure", "item": active_case[0], "source_duration_ms": active_case[7], "elapsed_ms": Time.get_ticks_msec() - activation_ticks(int(active_case[0]))})
		write_receipts()
	super.fail(message)

func proof_cases() -> Array:
	return [
		[130214, 195753, 25017, "Worn Doll", 1.0, 99535, false, 60000],
		[163745, 280084, 37806, "Witherbark Gong", 1.0, 143703, false, 30000],
	]

func scenery(title: String) -> Node3D:
	var actor_title := "Lonely Worn Doll" if title == "Worn Doll" else title
	return super.scenery(actor_title)
