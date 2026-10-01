extends SceneTree

## The native GameTooltip over its main sources with real mouse events, against a private
## server (docs/specs/unit-tooltip.md, "Native Godot coverage"). Environment:
##   GODOT_TEST_SERVER   server address (a private test server)
##   TOOLTIP_ACCOUNT / TOOLTIP_CHARACTER  account (password fbtest) and a level-10 Human
##                       mage at Brother Danil (-8904 -110.5 82.1) carrying Defias Rapier
##                       (1925) and Linen Cloth (2589), a Worn Shortsword (25) equipped
##   TOOLTIP_SHOTS       screenshot directory
## Hovers, in order: an action button (default anchor, cost/cast lines, Spell ID), the
## spellbook (ANCHOR_RIGHT), Arcane Intellect in the BuffFrame (time remaining, BOTTOMLEFT),
## Brother Danil in the world (vendor section from the server, Creature ID), the
## PlayerFrame and TargetFrame, a merchant cell (ANCHOR_RIGHT, Item ID), the Defias Rapier
## in the backpack (damage, stats) with Shift (Equipped comparison and deltas), the
## backpack button, the minimap zone text and clock, and a cooldown counting down.

const PASSWORD := "fbtest"
const ARCANE_INTELLECT := 1459
const ARCANE_BLAST := 30451
const CONJURE_REFRESHMENT := 190336
const VENDOR := "Brother Danil"
const BAR_KEYS := [KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0, KEY_MINUS, KEY_EQUAL]

var client: Node
var shots := "/tmp/claude/tooltips-live/"
var character := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("TOOLTIP_ACCOUNT")
	character = OS.get_environment("TOOLTIP_CHARACTER")
	if server == "" or account == "" or character == "":
		fail("GODOT_TEST_SERVER, TOOLTIP_ACCOUNT and TOOLTIP_CHARACTER are required")
		return
	if OS.get_environment("TOOLTIP_SHOTS") != "":
		shots = OS.get_environment("TOOLTIP_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	if not await wait_until(func(): return spells().catalog_ready and spells().known.has(ARCANE_INTELLECT), 60000, "spell catalog"):
		return
	await wait_frames(60)
	for step in [action_bar, spellbook, buff, world_unit, unit_frames, merchant, minimap, cooldown]:
		if not await step.call():
			return
	print("FIXTURE TOOLTIPS_LIVE_DONE")
	client.free()
	quit(0)

## A spell on the main bar: GameTooltip_SetDefaultAnchor, catalog lines, the Spell ID line.
func action_bar() -> bool:
	var slot: int = spells().bar.find(ARCANE_BLAST)
	if slot < 0:
		fail("Arcane Blast is not on the bar: " + str(spells().bar))
		return false
	await hover(control("MainActionBarUI", "ActionButton%d" % (slot + 1)))
	if not await wait_tooltip("Arcane Blast"):
		return false
	var state := tooltip()
	print("FIXTURE ACTION_BAR ", state)
	if not state.lines.has("Spell ID: 30451|") or not has_line(state, "sec cast"):
		return fail_state("Arcane Blast tooltip lines", state)
	if not at_default_anchor(state):
		return fail_state("Action button tooltip not at the default anchor", state)
	await capture("01-action-bar-arcane-blast.png")
	return true

## A spellbook item: ANCHOR_RIGHT on the item button.
func spellbook() -> bool:
	await press(KEY_P)
	if not await wait_until(func(): return spells().spellbook_open, 3000, "spellbook"):
		return false
	await wait_frames(10)
	var button := control("SpellBookUI", "SpellBookItem%dButton" % ARCANE_BLAST)
	if button == null:
		fail("No Arcane Blast in the spellbook")
		return false
	await hover(button)
	if not await wait_tooltip("Arcane Blast"):
		return false
	var state := tooltip()
	var owner := ui_rect(button)
	# BOTTOMLEFT on the button's TOPRIGHT, before the screen clamp.
	if absf(state.rect[0] - owner.end.x) > 1.5 or absf(state.rect[1] + state.rect[3] - owner.position.y) > 1.5:
		return fail_state("Spellbook tooltip not right of %s" % owner, state)
	print("FIXTURE SPELLBOOK ", state.rect, " owner ", owner)
	await capture("02-spellbook-arcane-blast.png")
	await press(KEY_P)
	return true

## Arcane Intellect in the BuffFrame: ANCHOR_BOTTOMLEFT, the time remaining.
func buff() -> bool:
	if not await cast(ARCANE_INTELLECT, "Arcane Intellect"):
		return false
	if not await wait_until(func(): return control("BuffFrameUI", "BuffButton0") != null, 5000, "BuffButton0"):
		return false
	await wait_frames(10)
	var button := control("BuffFrameUI", "BuffButton0")
	await hover(button)
	if not await wait_tooltip("Arcane Intellect"):
		return false
	var state := tooltip()
	print("FIXTURE BUFF ", state)
	if not has_line(state, "remaining") or not state.lines.has("Spell ID: 1459|"):
		return fail_state("Arcane Intellect tooltip lines", state)
	var owner := ui_rect(button)
	if absf(state.rect[0] + state.rect[2] - owner.position.x) > 1.5 or absf(state.rect[1] - owner.end.y) > 1.5:
		return fail_state("Buff tooltip not BOTTOMLEFT of %s" % owner, state)
	await capture("03-buff-arcane-intellect.png")
	return true

## Brother Danil under the cursor: name, level, the server's vendor section, Creature ID.
func world_unit() -> bool:
	var npc: Dictionary = await find_vendor()
	if npc.is_empty():
		return false
	await move_mouse(npc.point)
	if not await wait_until(func(): return tooltip().title == VENDOR and has_line(tooltip(), "Sells"), 8000, "vendor tooltip with the server's items"):
		return false
	var state := tooltip()
	print("FIXTURE WORLD_UNIT ", state)
	if not has_line(state, "Level ") or not has_line(state, "Creature ID: ") or not at_default_anchor(state):
		return fail_state("Vendor tooltip lines or anchor", state)
	await capture("04-world-brother-danil.png")
	# Right-click targets him (and opens his merchant window for the merchant step).
	await click_point(npc.point, MOUSE_BUTTON_RIGHT)
	return await wait_until(func(): return client.merchant_state().open, 8000, "merchant open")

## PlayerFrame and TargetFrame: the unit they show at the default anchor.
func unit_frames() -> bool:
	await hover(control("UnitFramesUI", "PlayerFrame"))
	if not await wait_tooltip(character):
		return false
	var state := tooltip()
	print("FIXTURE PLAYER_FRAME ", state)
	if not state.lines.has("Level 10 Human Mage (Player)|") or not at_default_anchor(state):
		return fail_state("PlayerFrame tooltip", state)
	await capture("05-player-frame.png")
	await hover(control("UnitFramesUI", "TargetFrame"))
	if not await wait_tooltip(VENDOR):
		return false
	print("FIXTURE TARGET_FRAME ", tooltip())
	await capture("06-target-frame.png")
	return true

## A merchant cell (ANCHOR_RIGHT), then the rapier in the backpack with and without Shift.
func merchant() -> bool:
	var cell := control("MerchantUI", "MerchantItem1")
	await hover(cell)
	if not await wait_until(func(): return tooltip().visible and tooltip().title == client.merchant_state().items[0], 3000, "merchant cell tooltip"):
		return false
	var state := tooltip()
	var owner := ui_rect(cell)
	print("FIXTURE MERCHANT ", state, " owner ", owner)
	if not has_line(state, "Item ID: ") or absf(state.rect[0] - owner.end.x) > 1.5:
		return fail_state("Merchant cell tooltip", state)
	await capture("07-merchant-cell.png")
	var rapier := bag_slot(1925)
	if rapier.is_empty():
		fail("No Defias Rapier in the bags: " + str(client.merchant_state().bags))
		return false
	await hover(control("MerchantUI", "ContainerFrame%dSlot%d" % [rapier.bag, rapier.slot]))
	if not await wait_tooltip("Defias Rapier"):
		return false
	state = tooltip()
	print("FIXTURE RAPIER ", state)
	if not has_line(state, "Damage|Speed 2.60") or not has_line(state, "damage per second") or not state.lines.has("+2 Agility|") or not state.shopping.is_empty():
		return fail_state("Rapier tooltip lines", state)
	await capture("08-bag-rapier.png")
	push_shift(true)
	if not await wait_until(func(): return tooltip().shopping.size() == 1, 2000, "Shift comparison"):
		return false
	var compare: Dictionary = tooltip().shopping[0]
	print("FIXTURE COMPARE ", compare)
	if compare.header != "Equipped" or compare.title != "Worn Shortsword" or not compare.lines.has("+1.3 Damage Per Second|"):
		return fail_state("Comparison tooltip", tooltip())
	await capture("09-bag-rapier-shift-compare.png")
	push_shift(false)
	if not await wait_until(func(): return tooltip().shopping.is_empty(), 2000, "comparison hidden after Shift"):
		return false
	await hover(control("MerchantUI", "MerchantFrameCloseButton"))
	await click_point(control("MerchantUI", "MerchantFrameCloseButton").get_global_rect().get_center(), MOUSE_BUTTON_LEFT)
	return await wait_until(func(): return not client.merchant_state().open, 3000, "merchant closed")

## The minimap zone text (ANCHOR_LEFT) and the clock.
func minimap() -> bool:
	var zone := control("MinimapUI", "MinimapZoneText")
	await hover(zone)
	if not await wait_until(func(): return tooltip().visible and has_line(tooltip(), "World Map"), 3000, "zone tooltip"):
		return false
	var state := tooltip()
	print("FIXTURE ZONE ", state)
	if state.title != "Elwynn Forest":
		return fail_state("Zone tooltip title", state)
	await capture("10-minimap-zone.png")
	await hover(control("MinimapUI", "TimeManagerClockTicker"))
	if not await wait_tooltip("Time Info"):
		return false
	print("FIXTURE CLOCK ", tooltip())
	await capture("11-minimap-clock.png")
	return true

## Conjure Refreshment's 15 s cooldown counts down in its action-bar tooltip while it
## stays up.
func cooldown() -> bool:
	if not spells().bar.has(CONJURE_REFRESHMENT):
		fail("Conjure Refreshment is not on the bar: " + str(spells().bar))
		return false
	if not await cast(CONJURE_REFRESHMENT, "Conjure Refreshment"):
		return false
	await hover(control("MainActionBarUI", "ActionButton%d" % (spells().bar.find(CONJURE_REFRESHMENT) + 1)))
	if not await wait_until(func(): return has_line(tooltip(), "Cooldown remaining"), 8000, "cooldown line; spells %s" % spells()):
		return false
	var first := line_with(tooltip(), "Cooldown remaining")
	await wait_real(2.2)
	var second := line_with(tooltip(), "Cooldown remaining")
	print("FIXTURE COOLDOWN ", first, " -> ", second)
	if first == second:
		return fail_state("Cooldown did not count down", tooltip())
	await capture("12-cooldown-countdown.png")
	return true

# --- helpers ---

func tooltip() -> Dictionary:
	return client.tooltip_state()

func spells() -> Dictionary:
	return client.spells_state()

func has_line(state: Dictionary, text: String) -> bool:
	return line_with(state, text) != ""

func line_with(state: Dictionary, text: String) -> String:
	for line in state.lines:
		if str(line).contains(text):
			return line
	return ""

func ui_scale() -> float:
	return maxf(minf(root.size.x / 1920.0, root.size.y / 1080.0), 2.0 / 3.0)

## A control's rect in UI units.
func ui_rect(control: Control) -> Rect2:
	var rect := control.get_global_rect()
	return Rect2(rect.position / ui_scale(), rect.size / ui_scale())

## BOTTOMRIGHT 9 left of and 85 above the screen's (GameTooltipDefaultContainer).
func at_default_anchor(state: Dictionary) -> bool:
	var screen := Vector2(root.size) / ui_scale()
	return absf(state.rect[0] + state.rect[2] - (screen.x - 9.0)) < 1.5 and absf(state.rect[1] + state.rect[3] - (screen.y - 85.0)) < 1.5

func wait_tooltip(title: String) -> bool:
	return await wait_until(func(): return tooltip().visible and tooltip().title == title, 3000, "%s tooltip" % title)

func fail_state(what: String, state: Dictionary) -> bool:
	fail("%s: %s" % [what, state])
	return false

func control(host: String, name: String) -> Control:
	var ui := client.get_node_or_null(host)
	return ui.find_child(name, true, false) as Control if ui != null else null

func bag_slot(item_id: int) -> Dictionary:
	for item in client.merchant_state().bags:
		if item.item_id == item_id:
			return item
	return {}

func camera() -> Camera3D:
	return root.get_viewport().get_camera_3d()

## The vendor's pick shape centre on screen, selected by the native ray; turn until seen.
func find_vendor() -> Dictionary:
	var deadline := Time.get_ticks_msec() + 60000
	var turned := 0
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var units = client.get_node_or_null("WorldUnits")
		if units == null:
			continue
		for unit in units.get_children():
			if str(unit.name) != VENDOR:
				continue
			var area := unit.find_child("UnitPick", true, false) as Area3D
			if area == null:
				continue
			var world_point := (area.get_child(0) as Node3D).global_position
			if not camera().is_position_in_frustum(world_point):
				continue
			var point := camera().unproject_position(world_point)
			var id = area.get_meta("unit_server_id")
			if UnitPicker.pick(camera(), point) == id:
				return {"id": id, "point": point}
		if turned < 60:
			push_key(KEY_RIGHT, true)
			await wait_frames(3)
			push_key(KEY_RIGHT, false)
			turned += 1
	var names := []
	var units = client.get_node_or_null("WorldUnits")
	if units != null:
		for unit in units.get_children():
			names.append(str(unit.name))
	fail("%s is not visible and unoccluded; units %s" % [VENDOR, names])
	return {}

func press_spell(spell: int) -> bool:
	var slot: int = spells().bar.find(spell)
	if slot < 0:
		fail("Spell %d is not on the main bar: %s" % [spell, spells().bar])
		return false
	await press(BAR_KEYS[slot])
	return true

## Cast `spell` once the GCD is over and wait for the server to take it.
func cast(spell: int, what: String) -> bool:
	if not await wait_until(func(): return spells().gcd_ms == 0, 5000, "GCD before " + what):
		return false
	if not await press_spell(spell):
		return false
	return await wait_until(func(): return spells().gcd_ms > 0, 3000, what + " accepted")

func wait_until(predicate: Callable, timeout_ms: int, what: String) -> bool:
	var deadline := Time.get_ticks_msec() + timeout_ms
	while Time.get_ticks_msec() < deadline:
		await process_frame
		if predicate.call():
			return true
	fail("Timed out waiting for %s: tooltip=%s" % [what, tooltip()])
	return false

func enter_world() -> bool:
	var deadline := Time.get_ticks_msec() + 60000
	var ui = null
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		ui = client.get_node_or_null("CharacterSelectUI")
		if state.reply_received and state.screen == "CharacterSelect" and state.character_count >= 1 and ui != null:
			break
	if ui == null:
		fail("No character select: " + str(client.account_state()))
		return false
	await click(ui.find_child("CharCard_0", true, false))
	var selected = ui.find_child("CharSelectCharacterName", true, false)
	if selected.text != character:
		fail("Card 0 is %s, not %s" % [selected.text, character])
		return false
	await click(ui.find_child("EnterWorld", true, false))
	deadline = Time.get_ticks_msec() + 120000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		if state.screen == "InWorld" and state.local_player_position != null and state.local_server_position != null and state.terrain.pending_count == 0 and not state.terrain.parsed_tiles.is_empty():
			print("FIXTURE IN_WORLD at ", state.local_player_position)
			await wait_frames(60)
			return true
	fail("Timed out entering the world: " + str(client.account_state()))
	return false

func press(code: Key) -> void:
	push_key(code, true)
	await wait_frames(2)
	push_key(code, false)
	await wait_frames(2)

func push_key(code: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.pressed = pressed
	root.push_input(event, true)

## Shift down or up as a key event carrying the modifier state.
func push_shift(pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = KEY_SHIFT
	event.physical_keycode = KEY_SHIFT
	event.pressed = pressed
	event.shift_pressed = pressed
	root.push_input(event, true)

func move_mouse(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	root.push_input(motion, true)
	Input.warp_mouse(point)
	await wait_frames(3)

func hover(target: Control) -> void:
	if target == null:
		fail("Missing control to hover")
		return
	await move_mouse(target.get_global_rect().get_center())

func click(target: Control) -> void:
	await click_point(target.get_global_rect().get_center(), MOUSE_BUTTON_LEFT)

func click_point(point: Vector2, button: MouseButton) -> void:
	await move_mouse(point)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.global_position = point
		event.button_index = button
		event.button_mask = (1 << (button - 1)) if pressed else 0
		event.pressed = pressed
		root.push_input(event, true)
		await process_frame
	await wait_frames(3)

func capture(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var error := image.save_png(shots + file)
	if error != OK:
		fail("Could not save " + file + ": " + str(error))

func wait_real(seconds: float) -> void:
	var deadline := Time.get_ticks_msec() + int(seconds * 1000)
	while Time.get_ticks_msec() < deadline:
		await process_frame

func wait_frames(count: int) -> void:
	for frame in range(count):
		await process_frame

func fail(message: String) -> void:
	push_error(message)
	quit(1)
