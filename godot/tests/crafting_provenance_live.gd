extends "res://tests/modern_crafting_unlearned_live.gd"

## Native crafting-provenance proof; run only in the lead-granted UDP5951 window.
## craft: real trainer264578, fourteen2881 casts, trainer2153, then one Create.
## readback: fresh client on a reopened server; observes only, never casts.
## The tooltip shows the normal item; origin comes from the owner-state readback.
const PANTS := 2303
const CRAFTED_ORIGIN := {"skill_line_ability_id": 1355, "recipe_spell_id": 2153, "crafting_quality_id": null}

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_provenance_"):
		fail("Provenance proof requires owned provenance accounts on UDP5951")
		return
	role = OS.get_environment("MODERN_CRAFTING_SKIN")
	shots = OS.get_environment("BANK_SHOTS")
	var phase := OS.get_environment("PROVENANCE_PHASE")
	if phase != "craft" and phase != "readback":
		fail("Explicit PROVENANCE_PHASE craft or readback is required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	var startup_deadline := Time.get_ticks_msec() + 240000
	while client.account_state().assets_starting and Time.get_ticks_msec() < startup_deadline:
		await process_frame
	if client.account_state().assets_starting:
		fail("Asset startup exceeded240s before provenance proof")
		return
	var error = client.connect_account("127.0.0.1:5951", OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from provenance fixture")
		return
	var done := false
	if phase == "craft":
		done = await craft_once()
	else:
		done = await readback_only()
	if not done:
		return
	print("FIXTURE CRAFTING_PROVENANCE_DONE phase=", phase, " skin=", role)
	client.free()
	quit(0)

func pants_entries() -> Array:
	return client.merchant_state().bags.filter(func(entry): return entry.item_id == PANTS)

func craft_once() -> bool:
	var before := pants_entries()
	if before.size() != 1 or before[0].crafting_origin != null:
		return fail("Craft requires exactly one unknown-origin2303 control: %s" % [before])
	var control_guid: int = before[0].item_guid
	if not await prepare_classic_leatherworking():
		return false
	if not await discover_and_learn_pants():
		return false
	await capture("p1-learned-pre-craft")
	var money_before := money()
	await press("ProfessionsCreate")
	if not await wait_until(func(): return pants_entries().size() == 2 and bag_count(2318) == 10 and bag_count(2320) == 0 and text("ProfessionsSkillText").contains("16/300"), "one actual2153 craft consumes4 leather+1 thread and raises rank15 to16"):
		return false
	if money() != money_before:
		return fail("Craft changed copper")
	var crafted := pants_entries().filter(func(entry): return entry.item_guid != control_guid)
	var control := pants_entries().filter(func(entry): return entry.item_guid == control_guid)
	if crafted.size() != 1 or control.size() != 1:
		return fail("Craft must add one new GUID and keep the control: %s" % [pants_entries()])
	if crafted[0].crafting_origin != CRAFTED_ORIGIN or control[0].crafting_origin != null:
		return fail("Origin must be on the crafted GUID only: %s" % [pants_entries()])
	await capture("p2-crafted-readback")
	print("PROVENANCE CRAFTED crafted_guid=", crafted[0].item_guid, " control_guid=", control_guid, " origin=", JSON.stringify(crafted[0].crafting_origin), " money=", money())
	await tap(KEY_ESCAPE)
	return await hover_pair(crafted[0], control[0], "p3")

func readback_only() -> bool:
	var crafted_guid := int(OS.get_environment("PROVENANCE_CRAFTED_GUID"))
	var control_guid := int(OS.get_environment("PROVENANCE_CONTROL_GUID"))
	var successes: Array = client.spells_state().cast_success.duplicate(true)
	var failure_seq: int = client.spells_state().cast_failure_seq
	var crafted := pants_entries().filter(func(entry): return entry.item_guid == crafted_guid)
	var control := pants_entries().filter(func(entry): return entry.item_guid == control_guid)
	if pants_entries().size() != 2 or crafted.size() != 1 or control.size() != 1:
		return fail("Reopened owner must keep both recorded GUIDs: %s" % [pants_entries()])
	if crafted[0].crafting_origin != CRAFTED_ORIGIN or control[0].crafting_origin != null:
		return fail("Reopened origin differs: %s" % [pants_entries()])
	if money() != 940 or bag_count(2934) != 0 or bag_count(2318) != 10 or bag_count(2320) != 0:
		return fail("Reopened bags/copper differ: money=%d bags=%s" % [money(), client.merchant_state().bags])
	await tap(KEY_K)
	if not await wait_until(func(): return text("ProfessionsSkillText").contains("16/300"), "persisted rank16"):
		return false
	await tap(KEY_ESCAPE)
	if not await hover_pair(crafted[0], control[0], "r1"):
		return false
	if client.spells_state().casting != 0 or client.spells_state().cast_success != successes or client.spells_state().cast_failure_seq != failure_seq:
		return fail("Readback emitted a cast or refusal")
	print("PROVENANCE REOPENED crafted_guid=", crafted_guid, " control_guid=", control_guid, " origin=", JSON.stringify(crafted[0].crafting_origin), " money=", money())
	return true

## Hover the crafted then the control slot; each tooltip is the normal item.
func hover_pair(crafted: Dictionary, control_entry: Dictionary, prefix: String) -> bool:
	if not shown("ContainerFrame%d" % crafted.bag):
		await press("MainMenuBarBackpackButton")
	for pair in [[crafted, "crafted"], [control_entry, "control"]]:
		var entry: Dictionary = pair[0]
		var slot := "ContainerFrame%dSlot%d" % [entry.bag, entry.slot]
		if not await wait_until(func(): return shown(slot), "bag slot " + slot):
			return false
		var point := control(slot).get_global_rect().get_center()
		var motion := InputEventMouseMotion.new()
		motion.position = point
		motion.global_position = point
		root.push_input(motion, true)
		Input.warp_mouse(point)
		if not await wait_until(func(): return client.tooltip_state().visible and client.tooltip_state().title == "Handstitched Leather Pants", "normal item tooltip on " + slot):
			return false
		var state: Dictionary = pants_entries().filter(func(item): return item.item_guid == entry.item_guid)[0]
		await capture("%s-%s-tooltip" % [prefix, pair[1]])
		print("PROVENANCE HOVER ", pair[1], " slot=", slot, " guid=", state.item_guid, " origin=", JSON.stringify(state.crafting_origin), " tooltip=", JSON.stringify(client.tooltip_state().lines))
	return true
