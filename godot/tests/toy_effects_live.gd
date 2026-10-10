extends SceneTree
# Owned private realm only; native rendered frames and observable toy/aura lifecycle.
var client: Node
var actor: Node3D
var output := OS.get_environment("TOYFX_OUTPUT")
var character := OS.get_environment("TOYFX_CHARACTER")
var receipts: Array = []
var frame_number := 0
var phase := "startup"

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_fixture")

func run_fixture() -> void:
	root.size = Vector2i(1920, 1080)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await wait_until(func(): return not client.account_state().assets_starting, "assets", 180000): return
	var error: String = client.connect_account("127.0.0.1:55382", OS.get_environment("TOYFX_ACCOUNT"), "fbtest", false)
	if error != "": fail(error); return
	if not await wait_until(func(): return control("CharCard_0") != null, "roster", 120000): return
	await click("CharCard_0")
	await click("EnterWorld")
	if not await wait_until(func(): return client.account_state().screen == "InWorld" and control("CollectionsMicroButton") != null, "world", 180000): return
	if not await wait_until(func():
		var nodes := client.find_children(character, "Node3D", true, false)
		if nodes.is_empty(): return false
		actor = nodes[0]
		return actor.find_children("PlayerModel", "Node3D", true, false).size() > 0, "native player visual", 120000): return
	await create_timer(2.0).timeout
	print("TOYFX_ACTOR ", actor.get_path(), " pos=", actor.position)
	var floor_y := actor.position.y
	await record_phase("native-scale", 1.0)
	await learn_world_enlarger()
	await click("CollectionsMicroButton")
	await click("ToyBoxSearchBox")
	await type_text("World Enlarger")
	if not await wait_until(func(): return control("ToySpellButton1Name") != null and control("ToySpellButton1Name").text == "World Enlarger", "toy search"): return
	await click("ToySpellButton1")
	if not await wait_until(func(): return has_aura(23126) and abs(actor.scale.x - 0.5) < 0.001, "replicated half scale"): return
	await tap(KEY_ESCAPE)
	await record_phase("toy18660-scale-half", 2.0)
	await cancel_aura(23126)
	if not await wait_until(func(): return not has_aura(23126) and abs(actor.scale.x - 1.0) < 0.001, "scale removal"): return
	await record_phase("scale-restored", 1.0)
	await prove_feather_fall(floor_y)

func learn_world_enlarger() -> void:
	await click("MainMenuBarBackpackButton")
	if not await wait_until(func(): return control("ContainerFrame0") != null, "backpack"): return
	var bag_name := ""
	for item in client.merchant_state().bags:
		if int(item.item_id) == 18660: bag_name = "ContainerFrame%dSlot%d" % [item.bag, item.slot]
	if bag_name == "": fail("World Enlarger18660 absent"); return
	await click(bag_name, MOUSE_BUTTON_RIGHT)
	if not await wait_until(func():
		for toy in client.toybox_state().learned:
			if int(toy.item_id) == 18660: return true
		return false, "toy learning"): return
	await tap(KEY_ESCAPE)

func prove_feather_fall(floor_y: float) -> void:
	#113542 lacks runtime item data; prove its authentic spell167273, not successful UseToy.
	if not command(OS.get_environment("TOYFX_ADMIN"), ["learn-spell", character, "167273"]): return
	# Main harness sends IPC and records its answer; this thread continues servicing it.
	var ready := FileAccess.open(output.path_join("feather-ready"), FileAccess.WRITE)
	ready.store_string(str(OS.get_process_id()))
	ready.close()
	if not await wait_until(func(): return has_aura(167273), "feather spell cast"): return
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, "-8949.95", "-132.49", str(floor_y + 20.0)]): return
	if not await wait_until(func(): return actor.position.y > floor_y + 15.0, "private elevated position"): return
	await record_phase("toy113542-spell-feather-fall", 3.3)
	if not await wait_until(func(): return actor.position.y <= floor_y + 0.5, "feather landing"): return
	# Its authentic ten-second duration can also expire during capture.
	if has_aura(167273): await cancel_aura(167273)
	if not await wait_until(func(): return not has_aura(167273), "feather removal"): return
	if not command(OS.get_environment("TOYFX_ADMIN"), ["set-position", character, "-8949.95", "-132.49", str(floor_y + 20.0)]): return
	if not await wait_until(func(): return actor.position.y > floor_y + 15.0, "ordinary fall setup"): return
	await record_phase("ordinary-fall-after-removal", 2.0)
	if not await wait_until(func(): return actor.position.y <= floor_y + 0.5, "ordinary landing"): return
	write_receipts()
	print("PASS: toy scale and authentic feather spell applied/rendered/reverted ", character, " frames=", frame_number)
	client.free()
	quit(0)

func command(binary: String, arguments: Array) -> bool:
	var result: Array = []
	var status := OS.execute(binary, arguments, result, true)
	print("TOYFX_COMMAND ", arguments, " exit=", status, " ", result)
	if status != 0: fail("private fixture command failed"); return false
	return true

func record_phase(label: String, seconds: float) -> void:
	phase = label
	var deadline := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < deadline:
		await RenderingServer.frame_post_draw
		var path := output.path_join("frame-%05d.png" % frame_number)
		if root.get_texture().get_image().save_png(path) != OK: fail("capture " + path); return
		var account = client.account_state()
		var server_position: Vector3 = account.local_server_position
		receipts.append({"phase": phase, "ms": Time.get_ticks_msec(), "scale": actor.scale.x,
			"position": [actor.position.x, actor.position.y, actor.position.z], "auras": client.aura_state(),
			"server_position": [server_position.x, server_position.y, server_position.z],
			"health": account.local_player_health})
		frame_number += 1
		await create_timer(0.1).timeout
	write_receipts()

func write_receipts() -> void:
	var file := FileAccess.open(output.path_join("receipts.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify(receipts, "\t"))
	file.close()

func has_aura(spell: int) -> bool:
	for aura in client.aura_state().buffs:
		if int(aura.spell_id) == spell: return true
	return false

func cancel_aura(spell: int) -> void:
	for aura in client.aura_state().buffs:
		if int(aura.spell_id) == spell:
			await click(aura.name, MOUSE_BUTTON_RIGHT)
			return
	fail("missing aura to cancel " + str(spell))

func control(name: String) -> Control:
	for node in client.find_children(name, "Control", true, false):
		if node.is_visible_in_tree(): return node
	return null

func wait_until(predicate: Callable, label: String, timeout: int = 30000) -> bool:
	var deadline := Time.get_ticks_msec() + timeout
	while Time.get_ticks_msec() < deadline:
		if predicate.call(): return true
		await process_frame
	fail("timeout: " + label + " account=" + str(client.account_state()))
	return false

func click(name: String, button: int = MOUSE_BUTTON_LEFT) -> void:
	var node := control(name)
	if node == null: fail("missing control " + name); return
	var point := node.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	await process_frame
	for down in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	await create_timer(0.1).timeout

func tap(code: Key) -> void:
	for down in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = down
		root.push_input(event, true)
		await process_frame
	await create_timer(0.1).timeout

func type_text(value: String) -> void:
	for character_key in value:
		var event := InputEventKey.new()
		event.unicode = character_key.unicode_at(0)
		event.keycode = OS.find_keycode_from_string(character_key.to_upper())
		event.physical_keycode = event.keycode
		event.pressed = true
		root.push_input(event, true)
		await process_frame
		event.pressed = false
		root.push_input(event, true)
		await process_frame

func fail(message: String) -> void:
	push_error("toyfx2 live: " + message)
	quit(1)
