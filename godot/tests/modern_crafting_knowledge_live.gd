extends "res://tests/crafting_ui_live.gd"

## Reuses the authoritative trait engine; fixture knowledge grants are not acquisition proof.
func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i(1920, 1080)
	root.content_scale_mode = Window.CONTENT_SCALE_MODE_VIEWPORT
	var endpoint := OS.get_environment("GODOT_TEST_SERVER")
	if endpoint != "127.0.0.1:5951" or not OS.get_environment("BANK_ACCOUNT").begins_with("fb_moderncraft_"):
		fail("Knowledge proof requires owned accounts on UDP5951")
		return
	role = OS.get_environment("MODERN_CRAFTING_SKIN")
	shots = OS.get_environment("BANK_SHOTS")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await process_frame
	if not await wait_until(func(): return not client.account_state().assets_starting, "asset startup before authentication"):
		return
	var error = client.connect_account(endpoint, OS.get_environment("BANK_ACCOUNT"), PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world(OS.get_environment("BANK_CHARACTER")):
		return
	Engine.max_fps = 60
	if client.account_state().ui_skin != role:
		fail("Observed skin differs from knowledge fixture")
		return
	if not await spend_knowledge():
		return
	print("FIXTURE MODERN_CRAFTING_KNOWLEDGE_DONE skin=", role)
	client.free()
	quit(0)

func click(target: Control) -> void:
	if not await wait_until(func(): return target.is_visible_in_tree(), "visible native input target " + str(target.name)):
		return
	await super.click(target)
