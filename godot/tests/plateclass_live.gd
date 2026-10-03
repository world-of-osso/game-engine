extends "res://tests/unitrank_live.gd"

## Nameplate classification indicator against a private server: left-click the creature to
## target it (a target always has a plate), then wait for its plate to carry the
## `GetClassificationAtlasElement` atlas left of the raid icon slot and capture it.
## Environment:
##   GODOT_TEST_SERVER       private server address (never 127.0.0.1:5000)
##   PLATECLASS_ACCOUNT / PLATECLASS_CHARACTER  account (password fbtest) and its first
##                           character, in sight of the creature
##   PLATECLASS_CREATURE     creature name (default Hogger, world.db 448, rank 1 elite)
##   PLATECLASS_ATLAS        expected atlas (default nameplates-icon-elite-gold)
##   PLATECLASS_SHOTS        screenshot directory

var atlas := "nameplates-icon-elite-gold"

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("PLATECLASS_ACCOUNT")
	character = OS.get_environment("PLATECLASS_CHARACTER")
	shots = OS.get_environment("PLATECLASS_SHOTS")
	creature = "Hogger"
	for pair in [["PLATECLASS_CREATURE", "creature"], ["PLATECLASS_ATLAS", "atlas"]]:
		if OS.get_environment(pair[0]) != "":
			set(pair[1], OS.get_environment(pair[0]))
	if server == "" or server == "127.0.0.1:5000" or account == "" or character == "" or shots == "":
		fail("Needs a private GODOT_TEST_SERVER, PLATECLASS_ACCOUNT/CHARACTER and PLATECLASS_SHOTS")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Live connection: " + error)
		return
	if not await enter_world():
		return
	if (await find_unit()).is_empty():
		return
	if not await track_until(func(): return client.target_state().target_name == creature, true, "%s targeted" % creature):
		return
	var target: int = client.target_state().target
	if not await approach(target):
		return
	if not await wait_until(func(): return plate(target).get("classification") == atlas, 5000, "%s on %s (%s): plate %s plates %s rules %s" % [atlas, creature, target, plate(target), client.nameplate_state().keys(), client.nameplate_rules(target)]):
		return
	await wait_frames(5)
	var entry := plate(target)
	print("FIXTURE CLASSIFICATION_PLATE ", target, " ", entry)
	var icon: Rect2 = entry.classification_rect
	var frame: Rect2 = entry.frame_rect
	print("FIXTURE CLASSIFICATION_GAP icon_right=%s frame_left=%s icon_center_y=%s frame_center_y=%s" % [icon.end.x, frame.position.x, icon.get_center().y, frame.get_center().y])
	await capture("01-%s-classification-nameplate.png" % creature.to_lower())
	print("FIXTURE PLATECLASS_LIVE_DONE")
	client.free()
	quit(0)

## Walk at the wandering creature until it is inside `nameplateMaxDistance` (60 yd):
## turn while its screen point is off centre, run forward while centred.
func approach(target: int) -> bool:
	var deadline := Time.get_ticks_msec() + 45000
	var forward := false
	while Time.get_ticks_msec() < deadline:
		var distance: float = client.nameplate_rules(target).get("distance", INF)
		if distance < 40.0:
			if forward:
				push_key(KEY_W, false)
			await wait_frames(30)
			return true
		var turn = null
		var node := unit_node()
		if node == null or not camera().is_position_in_frustum(node.global_position):
			turn = KEY_RIGHT
		else:
			var x := camera().unproject_position(node.global_position).x - root.size.x / 2.0
			if absf(x) > 80.0:
				turn = KEY_RIGHT if x > 0.0 else KEY_LEFT
		if turn != null:
			if forward:
				push_key(KEY_W, false)
				forward = false
			push_key(turn, true)
			await wait_frames(2)
			push_key(turn, false)
		elif not forward:
			push_key(KEY_W, true)
			forward = true
		await wait_frames(2)
	push_key(KEY_W, false)
	fail("Could not reach %s: rules %s" % [creature, client.nameplate_rules(target)])
	return false

func unit_node() -> Node3D:
	var units = client.get_node_or_null("WorldUnits")
	if units == null:
		return null
	for unit in units.get_children():
		if str(unit.name) == creature:
			return unit as Node3D
	return null

func plate(id: int) -> Dictionary:
	return client.nameplate_state().get(id, {})
