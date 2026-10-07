extends "res://tests/world_quest_flow.gd"

## Run check_kills on an accepted Northshire quest through QF_STEPS.
## Observe each real server credit before the parent combat helper's settle frames.
var previous_objective := ""
var progress_captures := 0

func _initialize() -> void:
	process_frame.connect(observe_progress)
	super._initialize()

func observe_progress() -> void:
	if not is_instance_valid(client):
		return
	var entry := log_entry(client.quest_state(), QUEST)
	if entry.is_empty():
		return
	var objective := String(entry.objectives[0])
	if objective == previous_objective:
		return
	var initial := previous_objective == ""
	previous_objective = objective
	if initial:
		return
	var count_end := objective.find(" ")
	var expected := objective.substr(count_end + 1) + ": " + objective.substr(0, count_end)
	await RenderingServer.frame_post_draw
	await capture_progress(expected)

func capture_progress(expected: String) -> void:
	var overlay := client.get_node_or_null("UIErrors")
	var found := false
	if overlay != null:
		for label in overlay.find_children("UIErrorsFrameLine*", "Label", true, false):
			if label.is_visible_in_tree() and label.text == expected:
				var color: Color = label.get_theme_color("font_color")
				found = color.is_equal_approx(Color.YELLOW)
	if not found:
		fail("Retail yellow progress message missing: " + expected)
		return
	progress_captures += 1
	await capture("progress-%d.png" % progress_captures)
	print("FIXTURE PROGRESS_MESSAGE ", expected)

func frames(count: int) -> void:
	# State predicates still gate readiness; avoid long frame-count waits at 1 FPS.
	await super.frames(mini(count, 4))

func admin(args: Array) -> bool:
	if args[0] == "revive":
		fail("Death requires normal resurrection; admin setup cannot revive")
		return false
	var output := []
	var bounded := ["20", OS.get_environment("QF_ADMIN")]
	bounded.append_array(args)
	var code := OS.execute("timeout", bounded, output, true)
	print("FIXTURE ADMIN ", args, " -> ", code, " ", output)
	if code != 0:
		fail("Admin setup failed: " + str(output))
	return code == 0

func check_kills() -> bool:
	if not await super.check_kills():
		return false
	if progress_captures == 0:
		fail("No real objective transition observed")
		return false
	return true
