extends RefCounted

# Original target.rs uses target-only for distance > 5, including corpse loot.
const PROBE_NAME := "Fixture Reach Probe"
const WAIT_MS := 15000
const QUIET_MS := 2250
const STABLE_FRAMES := 8
const CASES := [
	{"stage": "far", "distance": 5.1},
	{"stage": "inside", "distance": 4.9},
	{"stage": "edge", "distance": 5.0},
	{"stage": "living", "distance": 5.0},
]

func run(flow, client: Node) -> bool:
	var previous_id = null
	for case in CASES:
		var anchor_value = await stable_player_position(flow, client)
		if anchor_value == null:
			return false
		var anchor: Vector3 = anchor_value
		var stage: String = case.stage
		var desired := anchor + Vector3(0.0, 0.0, float(case.distance))
		print("FIXTURE LOOT_REACH_SPAWN %s %.9f %.9f %.9f" % [stage, desired.x, desired.y, desired.z])
		var probe := await wait_probe(flow, client, desired, previous_id)
		if probe.is_empty():
			return false
		previous_id = probe.id
		print("FIXTURE LOOT_REACH_READY")
		if stage != "living" and not await wait_loot_cursor(flow, client, probe):
			return false
		if not await exercise_click(flow, client, stage, anchor, desired):
			return false
		if not await flow.wait_inventory(client, 3, 1250):
			return false
	print("FIXTURE LOOT_REACH_DONE")
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await flow.process_frame
		if client.get_node_or_null("WorldUnits/" + PROBE_NAME) == null and client.account_state().unit_count == 3:
			return true
	flow.fail("Reach probes did not return to the original three-unit fixture")
	return false

func stable_player_position(flow, client: Node):
	var previous = null
	var stable := 0
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await flow.process_frame
		var position = client.account_state().local_player_position
		if not position is Vector3:
			continue
		stable = stable + 1 if position == previous else 0
		previous = position
		if stable >= STABLE_FRAMES:
			return position
	flow.fail("Local player did not settle before arranging reach input")
	return null

func wait_probe(flow, client: Node, desired: Vector3, previous_id) -> Dictionary:
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await flow.process_frame
		var unit := client.get_node_or_null("WorldUnits/" + PROBE_NAME) as Node3D
		if unit == null or unit.global_position != desired:
			continue
		var area := unit.find_child("UnitPick", true, false) as Area3D
		var model := unit.get_node_or_null("NpcVisualRoot/NpcModel")
		if area == null or model == null or area.get_meta("unit_server_id") == previous_id:
			continue
		# The fresh dead model must finish moving before choosing its click triangle.
		# Living probes keep their animated pose and are re-picked before the click.
		var animation := model.get_node_or_null("M2Animation")
		if animation != null and animation.current_animation_id() == 1:
			if not await wait_held_pose(flow, model):
				return {}
		return await flow.find_corpse(client, PROBE_NAME)
	flow.fail("Fresh replicated reach probe did not arrive exactly at requested position: " + str(desired))
	return {}

func wait_held_pose(flow, model: Node) -> bool:
	var skeleton := model.get_node_or_null("Skeleton3D") as Skeleton3D
	if skeleton == null:
		flow.fail("Reach corpse has no authored skeleton")
		return false
	var previous: Array[Transform3D] = []
	var stable := 0
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await flow.process_frame
		var pose: Array[Transform3D] = []
		for bone in range(skeleton.get_bone_count()):
			pose.append(skeleton.get_bone_global_pose(bone))
		stable = stable + 1 if pose == previous else 0
		previous = pose
		if stable >= STABLE_FRAMES:
			return true
	flow.fail("Reach corpse pose did not hold before exact mesh picking")
	return false

func wait_loot_cursor(flow, client: Node, probe: Dictionary) -> bool:
	var motion := InputEventMouseMotion.new()
	motion.position = probe.point
	flow.root.push_input(motion, true)
	var deadline := Time.get_ticks_msec() + WAIT_MS
	while Time.get_ticks_msec() < deadline:
		await flow.process_frame
		var unit := client.get_node_or_null("WorldUnits/" + PROBE_NAME)
		var sparkle := unit.get_node_or_null("LootSparkle") if unit != null else null
		if sparkle != null and sparkle.is_visible_in_tree() and client.merchant_state().cursor == "Loot":
			return true
	flow.fail("Per-looter reach probe was not marked lootable after model readiness")
	return false

func exercise_click(flow, client: Node, stage: String, anchor: Vector3, desired: Vector3) -> bool:
	var probe: Dictionary = await flow.find_corpse(client, PROBE_NAME)
	if probe.is_empty():
		return false
	var unit := client.get_node_or_null("WorldUnits/" + PROBE_NAME) as Node3D
	var actual_player: Vector3 = client.account_state().local_player_position
	if actual_player != anchor or unit == null or unit.global_position != desired:
		flow.fail("Reach arrangement changed before actual mesh click: " + stage)
		return false
	var distance := unit.global_position.distance_to(anchor)
	var valid_geometry := distance > 5.0 if stage == "far" else distance < 5.0 if stage == "inside" else distance == 5.0
	if not valid_geometry:
		flow.fail("Reach case does not exercise its independent boundary: %s distance=%s" % [stage, distance])
		return false
	print("LOOT REACH GEOMETRY stage=%s distance=%s player=%s probe=%s" % [stage, distance, anchor, desired])
	await flow.corpse_click(probe.point, false)
	print("FIXTURE LOOT_REACH_CLICKED")
	if stage in ["inside", "edge"]:
		if not await flow.wait_rows(client, ["Melted Candle", "1 Gold\n5 Silver\n2 Copper"]):
			return false
		var close := flow.loot_host(client).find_child("LootFrameCloseButton", true, false) as Control
		if close == null:
			flow.fail("Reach request did not open the authored close action")
			return false
		await flow.click(close)
		return await flow.wait_hidden(client)
	return await assert_target_only(flow, client, probe.id, stage)

func assert_target_only(flow, client: Node, id, stage: String) -> bool:
	var deadline := Time.get_ticks_msec() + QUIET_MS
	while Time.get_ticks_msec() < deadline:
		await flow.process_frame
		var frame := flow.loot_host(client) as Control
		if client.target_state().target != id or (frame != null and frame.is_visible_in_tree()):
			flow.fail("Reach %s click did not retain target without a loot window" % stage)
			return false
	if stage == "far" and not far_error_visible(client):
		flow.fail("Out-of-range corpse click did not display ERR_LOOT_TOO_FAR")
		return false
	return true

func far_error_visible(client: Node) -> bool:
	var errors := client.get_node_or_null("UIErrors")
	if errors == null:
		return false
	for node in errors.find_children("*", "Label", true, false):
		if node.is_visible_in_tree() and node.text == "You are too far away to loot that corpse.":
			return true
	return false
