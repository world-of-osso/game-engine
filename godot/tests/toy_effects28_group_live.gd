extends "res://tests/toy_effects14_signs_framed_live.gd"
# Actual source owner activation/meshes/natural expiry. Group-audience proof is
# separate real-UDP peer evidence; this fixture never substitutes privacy UI.
func proof_cases() -> Array:
	return [
		[130214, 195753, 25017, "Worn Doll", 1.0, 99535, false, 60000],
		[163745, 280084, 37806, "Witherbark Gong", 1.0, 143703, false, 30000],
	]

func scenery(title: String) -> Node3D:
	var actor_title := "Lonely Worn Doll" if title == "Worn Doll" else title
	return super.scenery(actor_title)
