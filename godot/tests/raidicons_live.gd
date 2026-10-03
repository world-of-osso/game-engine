extends "res://tests/unitrank_live.gd"

## Raid target icons against a private server: Tab-target the nearest enemy, pick Skull in the
## TargetFrame menu (UnitPopupRaidTarget8ButtonMixin → SetRaidTargetIcon("target", 8)), then
## wait for the server's RaidTargetIcons to put the Skull on its nameplate and the
## TargetFrame portrait; Skull again clears it (SetRaidTargetIcon toggles).
## Environment:
##   GODOT_TEST_SERVER       private server address (never 127.0.0.1:5000)
##   RAIDICONS_ACCOUNT / RAIDICONS_CHARACTER  account (password fbtest) and its first
##                           character, facing an enemy within Tab range
##   RAIDICONS_SHOTS         screenshot directory

const SKULL := 8

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("RAIDICONS_ACCOUNT")
	character = OS.get_environment("RAIDICONS_CHARACTER")
	shots = OS.get_environment("RAIDICONS_SHOTS")
	if server == "" or server == "127.0.0.1:5000" or account == "" or character == "" or shots == "":
		fail("Needs a private GODOT_TEST_SERVER, RAIDICONS_ACCOUNT/CHARACTER and RAIDICONS_SHOTS")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Live connection: " + error)
		return
	if not await enter_world():
		return
	if not await tab_target():
		return
	var target: int = client.target_state().target
	creature = client.target_state().target_name
	if not await pick_skull():
		return
	if not await wait_until(func(): return plate(target).get("raid_target") == SKULL and icon_shown(), 5000, "Skull on %s: plate %s icon %s plates %s rules %s" % [creature, plate(target), icon_state(), client.nameplate_state().keys(), client.nameplate_rules(target)]):
		return
	await wait_frames(5)
	var icon := control("UnitFramesUI", "TargetRaidTargetIcon")
	print("FIXTURE SKULL_PLATE ", target, " ", plate(target))
	print("FIXTURE SKULL_TARGET_FRAME ", icon.get_global_rect(), " frame ", control("UnitFramesUI", "TargetFrame").get_global_rect())
	await capture("01-skull-nameplate-and-target-frame.png")
	if not await pick_skull():
		return
	if not await wait_until(func(): return not plate(target).has("raid_target") and not icon_shown(), 5000, "Skull cleared"):
		return
	print("FIXTURE SKULL_CLEARED ", plate(target))
	await capture("02-skull-cleared.png")
	print("FIXTURE RAIDICONS_LIVE_DONE")
	client.free()
	quit(0)

## Right-click the TargetFrame and choose Skull.
func pick_skull() -> bool:
	await click_point(control("UnitFramesUI", "TargetFrame").get_global_rect().get_center(), MOUSE_BUTTON_RIGHT)
	var entry := control("UnitFramesUI", "UnitFrameContextMenuRaidTarget8")
	if entry == null or not entry.is_visible_in_tree():
		fail("TargetFrame menu has no Skull entry")
		return false
	await click(entry)
	await wait_frames(10)
	print("FIXTURE SKULL_PICKED target=%s" % client.target_state())
	return true

func plate(target: int) -> Dictionary:
	return client.nameplate_state().get(target, {})

func icon_shown() -> bool:
	var icon := control("UnitFramesUI", "TargetRaidTargetIcon")
	return icon != null and icon.is_visible_in_tree()

## TARGETNEARESTENEMY (Tab), turning until it selects a unit.
func tab_target() -> bool:
	for turn in range(36):
		push_key(KEY_TAB, true)
		await wait_frames(2)
		push_key(KEY_TAB, false)
		await wait_frames(10)
		if client.target_state().target != null:
			return true
		push_key(KEY_RIGHT, true)
		await wait_frames(6)
		push_key(KEY_RIGHT, false)
	fail("Tab selected no enemy: %s" % client.target_state())
	return false

func icon_state() -> String:
	var icon := control("UnitFramesUI", "TargetRaidTargetIcon")
	if icon == null:
		return "missing"
	return "%s visible=%s in_tree=%s rect=%s" % [icon.get_class(), icon.visible, icon.is_visible_in_tree(), icon.get_global_rect()]
