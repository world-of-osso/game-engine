extends "res://tests/world_nameplate_options_flow.gd"

# Run only with native_npc_visual_fixture nameplate-casts: its loopback server replicates
# the targeted enemy NPC's CastState and sends the game server's SpellFailure / SpellGo.
# Retail NamePlateCastingBarMixin: cast fills with icon and name, the player's kick turns
# it red "Interrupted: <player>", an uninterruptible cast shows the shield, SpellGo fills
# and fades it, a channel drains and fades when it ends, a completion failure reads Failed.
const KICKER := "Fixture Player"

var client: Node
var id := -1

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if not server.begins_with("127.0.0.1:") or server.ends_with(":0") or not prepare_assets():
		fail("Nameplate cast fixture needs its isolated assets and loopback server")
		return
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	if not await enter_and_target():
		return
	if not await check_kick():
		return
	if not await check_shielded_resolve():
		return
	if not await check_channel():
		return
	if not await check_failed():
		return
	print("FIXTURE NAMEPLATE_CASTS_DONE")
	client.free()
	quit(0)

func enter_and_target() -> bool:
	var error: String = client.connect_account(OS.get_environment("GODOT_TEST_SERVER"), "fixture", "fixture", false)
	if error != "" or not await wait_screen(client, "CharacterSelect", STARTUP_WAIT_MS):
		fail("Fixture authentication: " + error)
		return false
	var ui: Node = null
	var ui_deadline := Time.get_ticks_msec() + STARTUP_WAIT_MS
	while ui == null and Time.get_ticks_msec() < ui_deadline:
		ui = client.get_node_or_null("CharacterSelectUI")
		await process_frame
	if ui == null:
		fail("Character select UI never mounted: " + str(client.account_state()))
		return false
	await click_control(ui.find_child("CharCard_0", true, false))
	await click_control(ui.find_child("EnterWorld", true, false))
	var deadline := Time.get_ticks_msec() + 40000
	while Time.get_ticks_msec() < deadline and client.account_state().screen != "InWorld":
		await process_frame
	var npc := client.get_node_or_null("WorldUnits/Fixture Creature") as Node3D
	if npc == null:
		fail("Replicated NPC absent: " + str(client.account_state()))
		return false
	for area in npc.find_children("UnitPick", "Area3D", true, false):
		id = area.get_meta("unit_server_id")
	for attempt in range(8):
		if client.target_state().target == id:
			break
		await tap(KEY_TAB)
	if client.target_state().target != id:
		fail("Tab never targeted replicated enemy")
		return false
	return await turn_until_plate(client, id)

func cast() -> Dictionary:
	var plate: Dictionary = client.nameplate_state().get(id, {})
	return plate.get("cast", {})

# The cast bar `predicate` holds within `ms`.
func wait_cast(what: String, predicate: Callable, ms := 8000) -> bool:
	var deadline := Time.get_ticks_msec() + ms
	var frames := 0
	while Time.get_ticks_msec() < deadline:
		var current := cast()
		if predicate.call(current):
			return true
		frames += 1
		if frames % 60 == 0:
			print("CAST WAIT %s: %s" % [what, current])
		await process_frame
	fail("%s: %s" % [what, cast()])
	return false

func check_kick() -> bool:
	print("FIXTURE CAST_START")
	if not await wait_cast("Fireball cast bar", func(c): return c.get("bar_type") == "standard" and c.get("text") == "Fireball" and c.get("casting", false) and c.get("visible", false) and not c.get("shield_visible", true)):
		return false
	if not await wait_cast("Fireball icon", func(c): return c.get("icon_visible", false), 60000):
		return false
	var before: float = cast().fraction
	await wait_frames(20)
	if cast().fraction <= before:
		fail("Cast bar did not fill: %s -> %s" % [before, cast().fraction])
		return false
	print("FIXTURE CAST_KICK")
	var color := Color(0.96, 0.55, 0.73).to_html(false)
	var kicked := "Interrupted: [color=#%s]%s[/color]" % [color, KICKER]
	if not await wait_cast("Kick", func(c): return c.get("bar_type") == "interrupted" and c.get("text") == kicked and c.get("spark") == "Some(PipRed)" and not c.get("casting", true)):
		return false
	var stopped: float = cast().fraction
	await wait_frames(20)
	if absf(cast().fraction - stopped) > 1e-5 or cast().alpha < 0.99:
		fail("Interrupted bar did not hold: %s" % [cast()])
		return false
	return await wait_cast("Interrupted bar fade-out", func(c): return c.is_empty(), 4000)

func check_shielded_resolve() -> bool:
	print("FIXTURE CAST_SHIELDED")
	if not await wait_cast("Uninterruptible Frostbolt", func(c): return c.get("bar_type") == "uninterruptable" and c.get("shield_visible", false) and not c.get("icon_visible", true) and c.get("text") == "Frostbolt"):
		return false
	print("FIXTURE CAST_RESOLVE")
	if not await wait_cast("SpellGo finish", func(c): return c.get("fraction") == 1.0 and not c.get("casting", true) and c.get("spark") == "None" and c.get("bar_type") == "uninterruptable"):
		return false
	return await wait_cast("Finished bar fade-out", func(c): return c.is_empty(), 3000)

func check_channel() -> bool:
	print("FIXTURE CAST_CHANNEL")
	if not await wait_cast("Arcane Missiles channel", func(c): return c.get("bar_type") == "channel" and c.get("channeling", false) and c.get("text") == "Arcane Missiles"):
		return false
	var before: float = cast().fraction
	await wait_frames(20)
	if cast().fraction >= before:
		fail("Channel did not drain: %s -> %s" % [before, cast().fraction])
		return false
	print("FIXTURE CAST_CHANNEL_END")
	if not await wait_cast("Channel stop", func(c): return not c.get("channeling", true) and c.get("bar_type") == "channel" and c.get("alpha", 0.0) > 0.0):
		return false
	return await wait_cast("Channel fade-out", func(c): return c.is_empty(), 3000)

func check_failed() -> bool:
	print("FIXTURE CAST_FAIL_START")
	if not await wait_cast("Second Fireball", func(c): return c.get("bar_type") == "standard" and c.get("casting", false)):
		return false
	print("FIXTURE CAST_FAIL")
	return await wait_cast("Completion failure", func(c): return c.get("bar_type") == "interrupted" and c.get("text") == "Failed")

func wait_frames(count: int) -> void:
	for frame in count:
		await process_frame
