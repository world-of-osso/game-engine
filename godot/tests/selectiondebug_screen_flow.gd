extends "res://tests/debug_screen_flow_base.gd"

# `--screen selectiondebug` (original `src/scenes/selection_debug/mod.rs`): five
# selection candidates in the authored selection debug screen. Down focuses the next
# candidate, a real click on row 3 selects it, Enter pins it; the CLI's dump-ui-tree
# shows that state and its screenshot the screen; Escape returns to Login.
# Run: native_debug_screen_fixture selectiondebug
const MIN_SCREEN_PIXELS := 20000

var scene: Node

func check_live() -> bool:
	scene = await wait_node("SelectionDebug")
	if scene == null:
		return false
	var login := client.get_node_or_null("LoginUI") as CanvasLayer
	if login == null or login.visible:
		fail("Login UI is missing or still visible over the selection debug screen")
		return false
	if client.account_state().reply_received:
		fail("Selection debug screen contacted a server")
		return false
	await settle(300)
	if not expect_state(0, false, "Initialized selection debug screen"):
		return false
	push_key(KEY_DOWN)
	await settle(100)
	if not expect_state(1, false, "Focused Quest NPC"):
		return false
	var row := scene.find_child("SelectionDebugRow_3", true, false) as Control
	if row == null or not row.is_visible_in_tree():
		fail("Row SelectionDebugRow_3 is not a visible control")
		return false
	click(row.get_global_rect().get_center())
	await settle(100)
	if not expect_state(3, false, "Selected Corpse / Invalid"):
		return false
	push_key(KEY_ENTER)
	await settle(200)
	return expect_state(3, true, "Pinned Corpse / Invalid")

func expect_state(index: int, pinned: bool, last_action: String) -> bool:
	var state: Dictionary = scene.debug_state()
	if state.selected_index != index or state.pinned != pinned or state.last_action != last_action:
		fail("Selection state %s, expected %d %s %s" % [state, index, pinned, last_action])
		return false
	return true

func click(at: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = at
	root.push_input(motion, true)
	mouse_button(MOUSE_BUTTON_LEFT, true, at)
	mouse_button(MOUSE_BUTTON_LEFT, false, at)

func check_cli() -> bool:
	var ui := tree_reply("ui")
	for expected in ["SelectionDebugRoot [Frame]", "SelectionDebugDetailLabel [FontString]", "text=\"Corpse / Invalid\"", "SelectionDebugLastActionValue [FontString]", "text=\"Pinned Corpse / Invalid\"", "SelectionDebugPin [Button]"]:
		if not ui.contains(expected):
			fail("CLI dump-ui-tree lacks %s:\n%s" % [expected, ui])
			return false
	if not tree_reply("tree").contains("SelectionDebugUI ("):
		fail("CLI dump-tree lacks the selection debug UI")
		return false
	var shot := screenshot()
	if shot == null:
		return false
	if shot.get_size() != SIZE:
		fail("CLI screenshot is %s, not %s" % [shot.get_size(), SIZE])
		return false
	var ui_layer := scene.get_node("SelectionDebugUI") as CanvasLayer
	ui_layer.visible = false
	var hidden := await capture()
	ui_layer.visible = true
	var shown := await capture()
	save(shown, "selection-live.png")
	var live_pixels := changed_pixels(shown, hidden)
	var cli_pixels := shared_changed_pixels(shot, shown, hidden)
	print("FIXTURE SELECTIONDEBUG_SCREEN_PIXELS cli=%d live=%d" % [cli_pixels, live_pixels])
	if live_pixels < MIN_SCREEN_PIXELS or cli_pixels < live_pixels * 0.9:
		fail("CLI screenshot shows %d of the live frame's %d screen pixels" % [cli_pixels, live_pixels])
		return false
	push_key(KEY_ESCAPE)
	await settle(200)
	if is_instance_valid(scene) and scene.is_inside_tree():
		fail("Escape did not leave the selection debug screen")
		return false
	if not (client.get_node("LoginUI") as CanvasLayer).visible:
		fail("Escape did not return to the Login screen")
		return false
	return true
