extends "res://tests/world_character_frame_flow.gd"

# Retail micro menu (MainMenuBarMicroButtons.lua) against a private game server: real
# clicks on CharacterMicroButton, PlayerSpellsMicroButton, QuestLogMicroButton and
# MainMenuMicroButton open their native window (the button shows pushed) and close it
# again; an open game menu disables every other button; the buttons without a native
# window are disabled, a click on one opens nothing, and its tooltip gives Retail's reason.
#
# Setup (game-server-admin, private UDP server): create-account fb_microbar fbtest,
# create-character fb_microbar <Name> 1 1.

const MB_ACCOUNT := "fb_microbar"
const NATIVE := ["CharacterMicroButton", "PlayerSpellsMicroButton", "QuestLogMicroButton", "MainMenuMicroButton"]
const DISABLED := {
	"ProfessionMicroButton": ["Professions (K)", "This system is currently disabled."],
	"AchievementMicroButton": ["Achievements (Y)", "This feature becomes available at level 10."],
	"HousingMicroButton": ["Housing Dashboard", "This action is not available right now"],
	"GuildMicroButton": ["Guild & Communities", "Unavailable"],
	"LFDMicroButton": ["Group Finder", "This system is currently disabled."],
	"CollectionsMicroButton": ["Warband Collections", "This system is currently disabled."],
	"EJMicroButton": ["Adventure Guide (J)", "This feature is not yet available."],
	"StoreMicroButton": ["Shop", "The shop is currently unavailable."],
}

func run_test() -> void:
	root.size = Vector2i(1600, 900)
	cf_shots = OS.get_environment("MICRO_SHOTS")
	cf_character = OS.get_environment("MICRO_CHARACTER")
	var server := OS.get_environment("GODOT_TEST_SERVER")
	if cf_shots == "" or cf_character == "" or not server.begins_with("127.0.0.1:") or server == "127.0.0.1:5000":
		fail("Set MICRO_SHOTS, MICRO_CHARACTER and a private GODOT_TEST_SERVER (not :5000)")
		return
	DirAccess.make_dir_recursive_absolute(cf_shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, MB_ACCOUNT, PASSWORD, false)
	if error != "":
		fail("Connection: " + error)
		return
	if not await cf_enter_world():
		return
	if not mb_expect_states(NATIVE, "Normal"):
		return
	await cf_capture("01-micro-menu-idle.png")
	for name in NATIVE:
		if not await mb_toggle(name):
			return
	for name in DISABLED:
		if not await mb_disabled(name):
			return
	if not await mb_hover_and_pushed_shot():
		return
	print("FIXTURE WORLD_MICRO_MENU_DONE")
	client.free()
	quit(0)

func mb_states() -> Dictionary:
	return client.character_frame_state().micro

func mb_expect_states(names: Array, expected: String) -> bool:
	var states := mb_states()
	for name in names:
		if states[name] != expected:
			fail("%s is %s, expected %s: %s" % [name, states[name], expected, states])
			return false
	return true

func mb_open(name: String) -> bool:
	match name:
		"CharacterMicroButton":
			return client.character_frame_state().open and cf_visible("CharacterFrame")
		"PlayerSpellsMicroButton":
			return client.get_node_or_null("SpellBookUI") != null
		"QuestLogMicroButton":
			return client.quest_state().log_open
		"MainMenuMicroButton":
			return client.get_node_or_null("GameMenuUI") != null
	return false

func mb_click(name: String) -> void:
	await click_control(cf_micro(name), MOUSE_BUTTON_LEFT)
	await frames(4)

# One click opens the window and pushes its button; a second click closes it.
func mb_toggle(name: String) -> bool:
	await mb_click(name)
	if not mb_open(name) or mb_states()[name] != "Pushed":
		fail("%s did not open its window: open %s, state %s" % [name, mb_open(name), mb_states()[name]])
		return false
	if name == "MainMenuMicroButton":
		var others := NATIVE.filter(func(other): return other != name)
		if not mb_expect_states(others, "Disabled"):
			return false
		await cf_capture("02-game-menu-open.png")
		# The native game menu's full-screen dim takes the pointer, so the second
		# click cannot reach the button; Escape (`ToggleGameMenu`) closes it.
		await tap(KEY_ESCAPE)
		await frames(4)
	else:
		await mb_click(name)
	if mb_open(name) or mb_states()[name] != "Normal":
		fail("%s did not close its window: open %s, state %s" % [name, mb_open(name), mb_states()[name]])
		return false
	print("FIXTURE MICRO_TOGGLE ", name)
	return true

# A button whose window is not converted yet is lit, opens nothing, and its tooltip is
# the Retail title only; the click shows the Retail unavailable line as an error.
func mb_disabled(name: String) -> bool:
	if mb_states()[name] != "Normal":
		fail("%s is %s, expected Normal" % [name, mb_states()[name]])
		return false
	await mb_click(name)
	for native in NATIVE:
		if mb_open(native):
			fail("Clicking %s opened %s" % [name, native])
			return false
	await hover_point(cf_micro(name).get_global_rect().get_center())
	await frames(8)
	var state: Dictionary = client.tooltip_state()
	var expected: Array = DISABLED[name]
	if not state.visible or not state.title.begins_with(expected[0]) or not state.lines.is_empty():
		fail("%s tooltip: %s" % [name, state])
		return false
	print("FIXTURE MICRO_UNCONVERTED ", name, " ", state.title)
	return true

# Character Info open (pushed portrait button) with the pointer on Talents & Spellbook
# (mouseover art), then the Shop tooltip.
func mb_hover_and_pushed_shot() -> bool:
	await mb_click("CharacterMicroButton")
	await hover_point(cf_micro("PlayerSpellsMicroButton").get_global_rect().get_center())
	await frames(8)
	var tooltip: Dictionary = client.tooltip_state()
	if mb_states().CharacterMicroButton != "Pushed" or not tooltip.visible or not tooltip.title.begins_with("Talents & Spellbook"):
		fail("Pushed/hover state: %s tooltip %s" % [mb_states(), tooltip])
		return false
	await cf_capture("03-pushed-and-hover.png")
	await hover_point(cf_micro("StoreMicroButton").get_global_rect().get_center())
	await frames(8)
	await cf_capture("04-disabled-tooltip.png")
	print("FIXTURE MICRO_SHOTS ", cf_shots)
	return true
