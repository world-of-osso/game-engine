extends "res://tests/debug_screen_flow_base.gd"

# `--screen nameplatedebug` (original `src/scenes/nameplate_debug.rs`,
# docs/specs/nameplate-debug.md): three preview owners on the in-world plate renderer.
# Necrotic Bolt (133) and Arcane Missiles (5143) loop on Retail nameplate cast bars with
# their spell icon and name; Space pauses and resumes; a real click selects a plate.
# Run: native_debug_screen_fixture nameplatedebug
const OWNERS := ["Zolramus Sorcerer", "Channeling Adept", "Training Guardian"]
const MIN_PLATE_PIXELS := 3000

var scene: Node

func check_live() -> bool:
	scene = await wait_node("NameplateDebug")
	if scene == null:
		return false
	var login := client.get_node_or_null("LoginUI") as CanvasLayer
	if login == null or login.visible:
		fail("Login UI is missing or still visible over the nameplate preview")
		return false
	if client.account_state().reply_received:
		fail("Nameplate preview contacted a server")
		return false
	if not await wait_icons():
		return false
	var plates: Dictionary = scene.debug_state().plates
	for owner in OWNERS:
		if not plates.has(owner) or plates[owner].name != owner:
			fail("No plate named %s: %s" % [owner, plates])
			return false
	if not expect_cast("Zolramus Sorcerer", "standard", "Necrotic Bolt") or not expect_cast("Channeling Adept", "channel", "Arcane Missiles"):
		return false
	if not await check_loop():
		return false
	if not await check_pause():
		return false
	return await check_select()

func cast(owner: String) -> Dictionary:
	return scene.debug_state().plates[owner].get("cast", {})

func expect_cast(owner: String, bar_type: String, text: String) -> bool:
	var bar := cast(owner)
	if bar.get("bar_type") != bar_type or bar.get("text") != text or not bar.get("visible", false) or not bar.get("icon_visible", false) or bar.get("shield_visible", true):
		fail("%s cast bar: %s" % [owner, bar])
		return false
	return true

# The spell catalog loads on a worker; the icons follow.
func wait_icons() -> bool:
	var deadline := Time.get_ticks_msec() + 90000
	while Time.get_ticks_msec() < deadline:
		var plates: Dictionary = scene.debug_state().plates
		if plates.has("Zolramus Sorcerer") and plates["Zolramus Sorcerer"].get("cast", {}).get("icon_visible", false):
			return true
		await process_frame
	fail("Spell icons never showed: %s" % [scene.debug_state()])
	return false

# The 5 s bolt restarts at the loop, the 6 s channel keeps draining.
func check_loop() -> bool:
	var bolt := cast("Zolramus Sorcerer").fraction as float
	var channel := cast("Channeling Adept").fraction as float
	await settle(400)
	if cast("Zolramus Sorcerer").fraction <= bolt or cast("Channeling Adept").fraction >= channel:
		fail("Cast fills and channel drains: bolt %.3f -> %.3f, channel %.3f -> %.3f" % [bolt, cast("Zolramus Sorcerer").fraction, channel, cast("Channeling Adept").fraction])
		return false
	var deadline := Time.get_ticks_msec() + 8000
	var peak := 0.0
	while Time.get_ticks_msec() < deadline:
		var fraction: float = cast("Zolramus Sorcerer").get("fraction", 0.0)
		peak = maxf(peak, fraction)
		if peak > 0.9 and fraction < 0.2 and cast("Zolramus Sorcerer").get("casting", false):
			return true
		await process_frame
	fail("Necrotic Bolt never looped (peak %.3f): %s" % [peak, cast("Zolramus Sorcerer")])
	return false

func check_pause() -> bool:
	push_key(KEY_SPACE)
	await settle(100)
	var state: Dictionary = scene.debug_state()
	var frozen := cast("Channeling Adept").fraction as float
	await settle(600)
	var later: Dictionary = scene.debug_state()
	if not later.paused or later.elapsed["Channeling Adept"] != state.elapsed["Channeling Adept"] or absf(cast("Channeling Adept").fraction - frozen) > 1e-5 or not cast("Channeling Adept").visible or not later.caption.contains("Paused"):
		fail("Space did not pause with bars shown: %s" % [later])
		return false
	push_key(KEY_SPACE)
	await settle(300)
	if scene.debug_state().paused or cast("Channeling Adept").fraction >= frozen:
		fail("Space did not resume: %s" % [scene.debug_state()])
		return false
	return true

func check_select() -> bool:
	var rect: Rect2 = scene.debug_state().plates["Training Guardian"].frame_rect
	var at := rect.get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = at
	root.push_input(motion, true)
	mouse_button(MOUSE_BUTTON_LEFT, true, at)
	mouse_button(MOUSE_BUTTON_LEFT, false, at)
	await settle(100)
	var state: Dictionary = scene.debug_state()
	if state.selected != "Training Guardian" or not state.caption.ends_with("Selected: Training Guardian"):
		fail("Clicking the plate did not select it: %s" % [state])
		return false
	return true

func check_cli() -> bool:
	var tree := tree_reply("tree")
	for expected in ["NameplateDebug (", "Nameplates (", "CastBar (", "BorderShield (", "Spark ("]:
		if not tree.contains(expected):
			fail("CLI dump-tree lacks %s" % expected)
			return false
	var shot := screenshot()
	if shot == null:
		return false
	var layer := scene.get_node("Nameplates") as CanvasLayer
	layer.visible = false
	var hidden := await capture()
	layer.visible = true
	var shown := await capture()
	save(shown, "plates-live.png")
	var live_pixels := changed_pixels(shown, hidden)
	var cli_pixels := shared_changed_pixels(shot, shown, hidden)
	print("FIXTURE NAMEPLATEDEBUG_PLATE_PIXELS cli=%d live=%d" % [cli_pixels, live_pixels])
	if live_pixels < MIN_PLATE_PIXELS or cli_pixels < live_pixels * 0.7:
		fail("CLI screenshot shows %d of the live frame's %d plate pixels" % [cli_pixels, live_pixels])
		return false
	return true
