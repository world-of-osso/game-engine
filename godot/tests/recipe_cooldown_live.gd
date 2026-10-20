extends "res://tests/crafting_ui_live.gd"

## Native recipe-cooldown proof; run only in the lead-granted UDP5951 window.
## craft: one real Prismatic Sphere28027 Create starts the 48h category1174 cooldown.
## readback: fresh client on a reopened server; observes the replayed cooldown, never casts.
const SPHERE := 28027
const SPHERE_ITEM := 22460
const SHARD := 22449

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_cooldown_"):
		fail("Recipe cooldown proof requires owned cooldown accounts on UDP5951")
		return
	role = OS.get_environment("MODERN_CRAFTING_SKIN")
	shots = OS.get_environment("BANK_SHOTS")
	var phase := OS.get_environment("COOLDOWN_PHASE")
	if phase != "craft" and phase != "readback":
		fail("Explicit COOLDOWN_PHASE craft or readback is required")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	var startup_deadline := Time.get_ticks_msec() + 240000
	while client.account_state().assets_starting and Time.get_ticks_msec() < startup_deadline:
		await process_frame
	if client.account_state().assets_starting:
		fail("Asset startup exceeded240s before cooldown proof")
		return
	var error = client.connect_account("127.0.0.1:5951", OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from cooldown fixture")
		return
	var done := false
	if phase == "craft":
		done = await craft_once()
	else:
		done = await readback_only()
	if not done:
		return
	print("FIXTURE RECIPE_COOLDOWN_DONE phase=", phase, " skin=", role)
	client.free()
	quit(0)

func select_sphere() -> bool:
	await tap(KEY_K)
	if not await wait_until(func(): return shown("ProfessionsSearchBox"), "profession book"):
		return false
	await replace_field("ProfessionsSearchBox", "Prismatic Sphere")
	var recipe := "ProfessionRecipe%d" % SPHERE
	if not await wait_until(func(): return shown(recipe), "known Prismatic Sphere"):
		return false
	await press(recipe)
	return true

## Retail CraftingPage gating: the cooldown text is shown and every craft control is off.
func cooldown_gated() -> bool:
	return shown("ProfessionsRecipeCooldown") and text("ProfessionsRecipeCooldown").begins_with("Cooldown remaining: ") \
		and control("ProfessionsCreate").disabled and control("ProfessionsCreateAll").disabled

func craft_once() -> bool:
	if bag_count(SHARD) != 12 or bag_count(SPHERE_ITEM) != 0:
		return fail("Craft requires the seeded 12 shards and no sphere")
	if not await select_sphere():
		return false
	if not await wait_until(func(): return not shown("ProfessionsRecipeCooldown") and not control("ProfessionsCreate").disabled, "ready Prismatic Sphere"):
		return false
	await capture("c1-ready")
	await press("ProfessionsCreate")
	if not await wait_until(func(): return bag_count(SPHERE_ITEM) == 1 and bag_count(SHARD) == 8, "one actual28027 craft consumes4 shards"):
		return false
	if not await wait_until(cooldown_gated, "48h cooldown shown and Create disabled"):
		return false
	await capture("c2-cooldown")
	print("COOLDOWN CRAFTED text=", text("ProfessionsRecipeCooldown"), " shards=", bag_count(SHARD), " spheres=", bag_count(SPHERE_ITEM))
	return true

func readback_only() -> bool:
	var successes = client.spells_state().cast_success.duplicate()
	var failure_seq: int = client.spells_state().cast_failure_seq
	if bag_count(SHARD) != 8 or bag_count(SPHERE_ITEM) != 1:
		return fail("Reopened bags differ: %s" % [client.merchant_state().bags])
	if not await select_sphere():
		return false
	if not await wait_until(cooldown_gated, "replayed cooldown shown and Create disabled"):
		return false
	await capture("r1-replayed-cooldown")
	if client.spells_state().casting != 0 or client.spells_state().cast_success != successes or client.spells_state().cast_failure_seq != failure_seq:
		return fail("Readback emitted a cast or refusal")
	print("COOLDOWN REOPENED text=", text("ProfessionsRecipeCooldown"), " shards=", bag_count(SHARD), " spheres=", bag_count(SPHERE_ITEM))
	return true
