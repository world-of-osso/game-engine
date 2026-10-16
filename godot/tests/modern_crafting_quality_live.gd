extends "res://tests/bank_live.gd"

## Native physical input + authoritative allocation responses on private UDP5951.
## Uses the explicit private modern_crafting_fixture grants, not acquisition evidence.
func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if endpoint != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Modern crafting proof requires owned accounts on UDP5951")
		return
	role = OS.get_environment("MODERN_CRAFTING_SKIN")
	shots = OS.get_environment("BANK_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(endpoint, OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs: " + str(client.account_state().ui_skin))
		return
	await tap(KEY_K)
	if not await wait_until(func(): return shown("ProfessionRecipe375188"), "authored Alchemist's Hat recipe"):
		return
	await press("ProfessionRecipe375188")
	if not await wait_until(func(): return text("ProfessionsOutputName") == "Alchemist's Hat", "quality schematic"):
		return
	if shown("ProfessionsOutputSlot") or not control("ProfessionsCreate").disabled:
		fail("Missing operation must not assign a crafted item or enable Create")
		return
	await capture("1-quality-recipe-unassigned")
	await press("ProfessionReagent1Slot")
	if not await wait_until(func(): return shown("ProfessionsQualityDialog"), "source reagent-quality dialog"):
		return
	if not await typed_quantity_commit():
		return
	if not await choose_leather():
		return
	await capture("2-mixed-tier-dialog")
	await press("ProfessionsQualityCancel")
	await press("ProfessionReagent1Slot")
	if not await wait_until(func(): return quantity(193208) == "0" and quantity(193210) == "0", "Cancel discards draft allocation"):
		return
	if not await choose_leather():
		return
	await press("ProfessionsQualityAccept")
	if not await wait_until(func(): return text("ProfessionsCraftStatus").contains("Slot 0 requires exactly 2"), "server rejects incomplete required slot allocation"):
		return
	await capture("3-authoritative-missing-slot")
	await press("ProfessionReagent0Slot")
	if not await wait_until(func(): return shown("ProfessionQuality193222Choice"), "authored scaled hide choices"):
		return
	await press("ProfessionQuality193222Choice")
	if not await wait_until(func(): return quantity(193222) == "2", "owned scaled hide allocation"):
		return
	await press("ProfessionsQualityAccept")
	if not await wait_until(func(): return text("ProfessionsCraftStatus").contains("CRAFTING_OPERATION_COMPOSITION"), "server validates all slots but refuses quality assignment"):
		return
	if shown("ProfessionsOutputSlot") or not control("ProfessionsCreate").disabled:
		fail("Validated reagents are not permission to invent a crafted output")
		return
	await capture("4-validated-selection-no-outcome")
	await press("ProfessionReagent1Slot")
	if not await wait_until(func(): return quantity(193208) == "6" and quantity(193210) == "4", "accepted allocation survives reopening"):
		return
	if not control("ProfessionQuality193211Choice").disabled or control("ProfessionQuality193211Quantity").editable:
		fail("Unowned tier3 reagent cannot be allocated")
		return
	await capture("5-accepted-tier-allocation")
	await press("ProfessionsQualityCancel")
	await press("ProfessionsCreate")
	await wait_frames(60)
	if text("ProfessionReagent0") != "2/2 Lustrous Scaled Hide" or text("ProfessionReagent1") != "10/10 Resilient Leather":
		fail("Refused operation changed observed bags")
		return
	print("FIXTURE MODERN_CRAFTING_QUALITY_DONE skin=", role, " recipe=375188 selected=193222x2+193208x6+193210x4 quality=UNASSIGNED")
	client.free()
	quit(0)

func typed_quantity_commit() -> bool:
	await press("ProfessionQuality193208Quantity")
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_A
		event.physical_keycode = KEY_A
		event.ctrl_pressed = true
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await type_text("9")
	await wait_frames(12)
	if quantity(193208) != "9":
		return fail("Focused input was overwritten before Enter/blur")
	await tap(KEY_ENTER)
	return await wait_until(func(): return quantity(193208) == "6", "Enter clamps typed quantity to six owned reagents")

func choose_leather() -> bool:
	if not await wait_until(func(): return shown("ProfessionQuality193208Choice"), "three authored leather tiers"):
		return false
	await press("ProfessionQuality193208Choice")
	if not await wait_until(func(): return quantity(193208) == "6", "tier1 clamped to six owned reagents"):
		return false
	if not control("ProfessionsQualityAccept").disabled:
		return fail("Partial allocation must disable Accept")
	await press("ProfessionQuality193210Choice")
	return await wait_until(func(): return quantity(193208) == "6" and quantity(193210) == "4" and not control("ProfessionsQualityAccept").disabled, "mixed6+4 allocation enables Accept")

func quantity(item: int) -> String:
	var field := control("ProfessionQuality%dQuantity" % item)
	return field.text if field is LineEdit else "missing"
