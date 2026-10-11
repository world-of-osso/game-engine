## Production portrait receipt/render regression. Private server/account required.
## PORTRAIT_ACTION accepts use_spell or check (target optional); PORTRAIT_STATE holds
## real viewport pixel coverage. check exits 0 only for visible, colored models.
extends SceneTree
var client: Node
var elapsed := 0.0
var action_serial := 0
var spec_requested := false
var check_requested := false
var check_target := false
func _initialize() -> void:
	call_deferred("start")
func start() -> void:
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
func _process(delta: float) -> bool:
	elapsed += delta
	if elapsed < 1.0 or client == null:
		return false
	elapsed = 0.0
	var action_path := OS.get_environment("PORTRAIT_ACTION")
	if FileAccess.file_exists(action_path):
		var action = JSON.parse_string(FileAccess.get_file_as_string(action_path))
		if action is Dictionary and action.get("serial", 0) > action_serial:
			action_serial = action.serial
			if action.get("check", false):
				check_requested = true
				check_target = action.get("target", false)
			if action.has("use_spell"):
				print("PORTRAIT_ACTION ", client.use_spell(action.use_spell))
	if client.account_state().screen == "InWorld" and not spec_requested and OS.get_environment("LIVE_SPEC_ID").to_int() > 0:
		spec_requested = true
		print("PORTRAIT_SPEC ", client.set_specialization(OS.get_environment("LIVE_SPEC_ID").to_int()))
	var states := {}
	for slot in ["PlayerPortrait", "TargetFramePortrait", "PetPortrait"]:
		var state: Dictionary = client.unit_portrait_state(slot)
		var image: Image = state.get("image")
		state.erase("image")
		if image != null:
			var colored := 0
			var opaque := 0
			for y in range(image.get_height()):
				for x in range(image.get_width()):
					var p := image.get_pixel(x, y)
					if p.a > 0.5:
						opaque += 1
						if max(p.r, max(p.g, p.b)) > 0.03:
							colored += 1
			state["colored_pixels"] = colored
			state["opaque_pixels"] = opaque
		states[slot] = state
	var out := FileAccess.open(OS.get_environment("PORTRAIT_STATE"), FileAccess.WRITE)
	out.store_string(JSON.stringify(states))
	out.close()
	if check_requested:
		var slots := ["PlayerPortrait"]
		if check_target:
			slots.append("TargetFramePortrait")
		for slot in slots:
			var state: Dictionary = states[slot]
			if not state.get("model_shown", false) or state.get("pending", true) or state.get("colored_pixels", 0) < 32:
				push_error("PORTRAIT_BLACK_FAIL %s %s" % [slot, state])
				quit(1)
				return false
		print("PORTRAIT_BLACK_PASS ", JSON.stringify(states))
		quit(0)
	return false
