extends "res://tests/toy_effects14_scaled_expiry_live.gd"

func frame_scenery_camera() -> void:
	# Observe from the open side, not through the authored building behind the player.
	client.set_camera_orbit(-PI / 2.0, -0.45, 8.0)

# Three complete source-bound stationary signs; ordinary destination47, natural60s.
func proof_cases() -> Array:
	return [
		[104331, 148627, 51525, "Warning Sign", 1.0, 73781, false, 60000],
		[119039, 176310, 51525, "Lilian's Warning Sign", 1.0, 88280, false, 60000],
		[170196, 304505, 92597, "Shirakess Warning Sign", 0.6, 156052, false, 60000],
	]
