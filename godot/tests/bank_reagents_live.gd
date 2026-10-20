extends "res://tests/crafting_ui_live.gd"

## Native bank-reagent proof; run only in the lead-granted UDP5951 window.
## The character's two Linen Cloth sit only in its character bank or only in its Warband
## bank (BANK_REAGENT_SOURCE character|warband); empty bags. One real Bolt of Linen Cloth2963
## Create must count and consume them with the bank frame never opened.
const BOLT_SPELL := 2963
const BOLT_ITEM := 2996

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_bankreag_"):
		fail("Bank reagent proof requires owned bankreag accounts on UDP5951")
		return
	var source := OS.get_environment("BANK_REAGENT_SOURCE")
	if source != "character" and source != "warband":
		fail("Explicit BANK_REAGENT_SOURCE character or warband is required")
		return
	role = OS.get_environment("MODERN_CRAFTING_SKIN")
	shots = OS.get_environment("BANK_SHOTS")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	var startup_deadline := Time.get_ticks_msec() + 240000
	while client.account_state().assets_starting and Time.get_ticks_msec() < startup_deadline:
		await process_frame
	if client.account_state().assets_starting:
		fail("Asset startup exceeded240s before bank reagent proof")
		return
	var error = client.connect_account("127.0.0.1:5951", OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from bank reagent fixture")
		return
	role = "%s-%s" % [role, source]
	if not await craft_from_bank():
		return
	print("FIXTURE BANK_REAGENTS_DONE source=", source, " skin=", OS.get_environment("MODERN_CRAFTING_SKIN"))
	client.free()
	quit(0)

## Keep the last rendered frame of any failure for diagnosis.
func fail(message: String) -> bool:
	if shots != "":
		root.get_texture().get_image().save_png("%s/%s-fail.png" % [shots, role])
	return super.fail(message)

func craft_from_bank() -> bool:
	if bag_count(LINEN) != 0 or bag_count(BOLT_ITEM) != 0 or client.bank_state().open:
		return fail("Requires empty bags, no bolt and a closed bank: %s" % [client.merchant_state().bags])
	await tap(KEY_K)
	if not await wait_until(func(): return shown("ProfessionsSearchBox"), "profession book"):
		return false
	await replace_field("ProfessionsSearchBox", "Bolt of Linen")
	var recipe := "ProfessionRecipe%d" % BOLT_SPELL
	if not await wait_until(func(): return shown(recipe), "known Bolt of Linen Cloth"):
		return false
	await press(recipe)
	if not await wait_until(func(): return text("ProfessionReagent0") == "2/2 Linen Cloth" and not control("ProfessionsCreate").disabled, "banked 2/2 Linen and Create enabled"):
		return false
	await capture("b1-banked-ready")
	print("BANK REAGENTS READY reagent=", text("ProfessionReagent0"), " create_all=", control("ProfessionsCreateAll").text, " bag_linen=", bag_count(LINEN))
	await press("ProfessionsCreate")
	if not await wait_until(func(): return bag_count(BOLT_ITEM) == 1, "one actual2963 craft"):
		return false
	if not await wait_until(func(): return text("ProfessionReagent0") == "0/2 Linen Cloth" and control("ProfessionsCreate").disabled, "bank reagents consumed and Create disabled"):
		return false
	await capture("b2-crafted")
	print("BANK REAGENTS CRAFTED reagent=", text("ProfessionReagent0"), " bolts=", bag_count(BOLT_ITEM), " bag_linen=", bag_count(LINEN), " bank_open=", client.bank_state().open)
	return true
