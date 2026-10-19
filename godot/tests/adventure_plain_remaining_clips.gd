extends "res://tests/adventure_plain_actor_animation.gd"
# All three remain in the default suite. Explicit fixture-only case isolation lets
# each real process finish and report while other source cases retain failures.
const REMAINING_CASES := [
	[4311,74282,"Atrigan",70,93,74,39,27,395,875,1867],
	[5204,96665,"Remornia",357,1061,363,182,213,499,1190,2424],
	[5932,128123,"Arcane Echo",1071,2672,1055,432,26,514,1302,2687],
]

func animated_cases() -> Array:
	var selected := OS.get_environment("JOURNAL_PLAIN_CLIP_CREATURE")
	if selected.is_empty():
		return REMAINING_CASES
	for row in REMAINING_CASES:
		if str(row[0]) == selected:
			return [row]
	return []
