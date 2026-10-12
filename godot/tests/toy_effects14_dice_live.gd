extends "res://tests/toy_effects14_signs_dice_live.gd"
# Dice's10s cooldown is independent of the completed one-hour sign capture.
func proof_cases() -> Array:
	return [[36863, 47770, 100, "Decahedral Dwarven Dice", 1], [36862, 47776, 6, "Worn Troll Dice", 2]]
