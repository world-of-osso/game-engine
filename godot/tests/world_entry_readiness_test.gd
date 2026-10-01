extends SceneTree

const Readiness = preload("res://tests/world_entry_readiness.gd")

func _initialize() -> void:
	var cases := [
		{
			"name": "terrain job still in flight",
			"state": {
				"terrain": {"map": "Azeroth", "pending_count": 1, "parsed_tiles": []},
				"world_objects": {"pending": 0}, "unit_visuals_pending": 0,
			},
			"expected": false,
		},
		{
			"name": "unit visual still in flight",
			"state": {
				"terrain": {"map": "Azeroth", "pending_count": 0, "parsed_tiles": []},
				"world_objects": {"pending": 0}, "unit_visuals_pending": 1,
			},
			"expected": false,
		},
		{
			"name": "all current requested jobs drained",
			"state": {
				"terrain": {"map": "Azeroth", "pending_count": 0,
					"parsed_tiles": [{"tile_y": 32, "tile_x": 48}]},
				"world_objects": {"pending": 0}, "unit_visuals_pending": 0,
			},
			"expected": true,
		},
		{
			"name": "object jobs still queued",
			"state": {
				"terrain": {"map": "Azeroth", "pending_count": 0,
					"parsed_tiles": [{"tile_y": 32, "tile_x": 48}]},
				"world_objects": {"pending": 2}, "unit_visuals_pending": 0,
			},
			"expected": false,
		},
	]
	var failures := 0
	for test_case in cases:
		var observed: bool = Readiness.is_ready(test_case.state)
		var passed: bool = observed == test_case.expected
		print("READINESS %s %s expected=%s observed=%s state=%s" % [
			"PASS" if passed else "FAIL", test_case.name,
			test_case.expected, observed, JSON.stringify(test_case.state),
		])
		if not passed:
			failures += 1
	print("READINESS_RESULT cases=%d failures=%d" % [cases.size(), failures])
	quit(1 if failures > 0 else 0)
