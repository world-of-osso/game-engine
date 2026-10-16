extends "res://tests/guild_lifecycle_live.gd"

const PERK_IDS := [78633, 83944, 83958, 83950, 83951]
const PERK_NAMES := ["Mount Up", "Hasty Hearth", "Mobile Banking", "The Quick and the Dead", "Guild Mail"]
var perk_mode := ""

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	Engine.max_fps = 30
	shots = OS.get_environment("BANK_SHOTS")
	diagnostics = OS.get_environment("GUILD_PERKS_DIAGNOSTICS")
	perk_mode = OS.get_environment("GUILD_PERKS_MODE")
	role = "perks-entry"
	assert(OS.get_environment("GODOT_TEST_SERVER") == "127.0.0.1:5891", "Private5891 only")
	assert(OS.get_environment("BANK_ACCOUNT") == "fb_guildcompletion_perkslead", "Private perk actor only")
	assert(perk_mode in ["controls", "readback"], "Explicit perk proof mode required")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	await wait_frames(30)
	if not await connect_private_account() or not await enter_world("Gcperkslead"):
		return
	for skin in ["Modern", "Forever"]:
		if not await select_skin(skin) or not await open_guild():
			return
		role = skin.to_lower() + "-perks"
		await press("CommunitiesGuildBenefitsTab")
		if not await wait_until(func(): return text("CommunitiesGuildPerksTitle") == "Guild Perks" and known_perks().size() == 5, "actual Benefits definitions and owner spell grants"):
			return
		for index in range(PERK_IDS.size()):
			if text("CommunitiesGuildPerk%dName" % PERK_IDS[index]) != PERK_NAMES[index]:
				fail("Wrong installed perk label")
				return
		await capture("overview")
		var tooltips: Array[Dictionary] = []
		for index in range(PERK_IDS.size()):
			await hover_at(control("CommunitiesGuildPerk%d" % PERK_IDS[index]).get_global_rect().get_center())
			if not await wait_until(func(): return text("TooltipTitle") == PERK_NAMES[index], "real spell tooltip " + str(PERK_IDS[index])):
				return
			var lines := tooltip_labels()
			if lines.size() < 2 or " ".join(lines).contains("$"):
				fail("Unresolved or absent actual spell description: " + str(lines))
				return
			tooltips.append({"id": PERK_IDS[index], "title": text("TooltipTitle"), "lines": lines})
			await capture("tooltip-%d" % PERK_IDS[index])
		write_json(skin.to_lower() + "-perks-tooltips.json", {"tooltips": tooltips, "owner": perk_observation()})
		if perk_mode == "controls" and not await prove_modified_link():
			return
		await close_guild()
	print("PASS GUILD_PERKS_NATIVE mode=", perk_mode, " known=", known_perks(), " both_skins=true")
	client.free()
	quit(0)

func perk_observation() -> Dictionary:
	var raw := str(client.call("guild_perks_observation"))
	var value: Variant = JSON.parse_string(raw)
	assert(value is Dictionary, "Real owner wire/chat observation required")
	return value

func known_perks() -> Array[int]:
	var known: Array[int] = []
	for id in perk_observation().known_spells:
		known.append(int(id))
	var actual: Array[int] = []
	for id in PERK_IDS:
		if int(id) in known:
			actual.append(int(id))
	return actual

func tooltip_labels() -> Array[String]:
	var lines: Array[String] = []
	var host := client.get_node_or_null("GameTooltipUI")
	if host == null:
		return lines
	for node in host.find_children("*", "Label", true, false):
		if node.is_visible_in_tree() and not node.text.is_empty():
			lines.append(node.text)
	return lines

func hover_at(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(2)

func prove_modified_link() -> bool:
	var point := control("CommunitiesGuildPerk78633").get_global_rect().get_center()
	await hover_at(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.button_mask = MOUSE_BUTTON_MASK_LEFT if pressed else 0
		event.shift_pressed = true
		event.pressed = pressed
		root.push_input(event, true)
		await wait_frames(2)
	if not await wait_until(func(): return shown("ChatFrame1EditBox") and perk_observation().draft_wire == "|Hspell:78633|h[Mount Up]|h", "real modified-click spell draft"):
		return false
	var edit := control("ChatFrame1EditBox") as LineEdit
	if edit.text != "[Mount Up]":
		return fail("Native edit displays raw markup or wrong metadata")
	await capture("chat-draft")
	await tap(KEY_ENTER)
	if not await wait_until(func(): return received_spell_label() != null, "actual server chat echo with a labeled spell link"):
		return false
	await hover_at(received_spell_label().get_global_rect().get_center())
	if not await wait_until(func(): return text("TooltipTitle") == "Mount Up", "received chat link real tooltip identity"):
		return false
	await capture("chat-link")
	write_json(role + "-chat.json", perk_observation())
	return true

func received_spell_label() -> Control:
	var host := client.get_node_or_null("ChatFrameUI")
	if host != null:
		for node in host.find_children("*", "Label", true, false):
			if node.is_visible_in_tree() and node.text == "[Mount Up]":
				return node
	return null

func wait_until(predicate: Callable, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		await RenderingServer.frame_post_draw
		if predicate.call():
			return true
	await capture("failure")
	return fail("Timed out waiting for " + what)

func fail(message: String) -> bool:
	print("FAIL GUILD_PERKS_NATIVE ", message)
	push_error(message)
	quit(1)
	return false
