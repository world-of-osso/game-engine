extends "res://tests/questlog_fidelity_live.gd"

## Real account/UDP5931 only. Source Feero4484/quest976/path35873.
## QF_STEPS=check_skin,check_escort; QF_ACCOUNT=fb_escort_modern|fb_escort_forever.
## Ambient world population is excluded by private fixture, not actor stat edits.
const ESCORT_QUEST := 976
const ESCORT_NAME := "Feero Ironhand"
const AMBUSH_NAMES := ["Dark Strand Assassin", "Forsaken Scout", "Aligar the Tormentor", "Balizar the Umbrage", "Caedakar the Vicious"]
var seen_ambushes: Dictionary = {}

func run_test() -> void:
	root.size = Vector2i(1920, 1080)
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5931":
		fail("Escort fixture requires owned UDP5931")
		return
	shots = OS.get_environment("QF_SHOTS").trim_suffix("/") + "/"
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	# Exercise the real --screen inworld startup path with owned credentials.
	# Character-select interaction is not this fixture's acceptance boundary.
	if not await wait_frames(func(): return client.account_state().screen == "InWorld" and client.account_state().local_player_position != null, "private startup in world", 180000):
		return
	if not await check_skin() or not await check_escort():
		fail("Native escort acceptance did not complete")
		return
	client.free()
	quit(0)

func check_escort() -> bool:
	if OS.get_environment("GODOT_TEST_SERVER") != "127.0.0.1:5931":
		fail("Escort fixture requires owned UDP5931")
		return false
	if not admin(["revive", character_name()]):
		return false
	if not await wait_frames(func(): return not player_dead(), "private account alive"):
		return false
	if not admin(["learn-spell", character_name(), "5176"]):
		return false
	if not admin(["learn-spell", character_name(), "5185"]):
		return false
	var giver := await locate(ESCORT_NAME)
	if giver.is_empty():
		return false
	if in_log(client.quest_state(), ESCORT_QUEST):
		if not await abandon_previous_escort():
			return false
	var visible := await find_unit(ESCORT_NAME)
	if not await open_giver(visible):
		return false
	var index: int = Array(client.quest_state().greeting_quests).find("Supplies to Auberdine")
	if index < 0:
		fail("Authentic quest976 not offered")
		return false
	await click_control(quest_control("QuestFrameUI", "QuestTitleButton%dText" % (index + 1)))
	if not await wait_quest(func(s): return s.get("page") == "Detail" and s.quest_id == ESCORT_QUEST, "escort976 detail"):
		return false
	await capture("01-accept.png")
	var start: Vector3 = unit_by_id(giver.id).global_position
	await click_control(quest_control("QuestFrameUI", "QuestFrameAcceptButton"))
	if not await wait_quest(func(s): return in_log(s, ESCORT_QUEST), "real escort acceptance"):
		return false
	var deadline := Time.get_ticks_msec() + 600000
	var walked := false
	var heard_source_speech := false
	var step := 0
	while Time.get_ticks_msec() < deadline:
		await frames(2)
		var entry := log_entry(client.quest_state(), ESCORT_QUEST)
		if entry.get("failed", false) or player_dead() or not client.unit_alive(giver.id):
			await capture("failed.png")
			fail("Private escort or player died before source credit")
			return false
		if not heard_source_speech:
			for label: Label in client.find_children("*", "Label", true, false):
				if label.is_visible_in_tree() and label.text.contains("We must move quickly."):
					heard_source_speech = true
					await capture("02-source-speech.png")
		if entry.get("completed", false):
			if not walked or not heard_source_speech or seen_ambushes.size() != AMBUSH_NAMES.size():
				fail("Credit without observed walking/source speech/all five authored ambusher identities")
				return false
			await capture("05-complete.png")
			print("PASS ESCORT976 skin=", OS.get_environment("QF_SKIN"), " walked=true ambushes=", seen_ambushes.keys(), " server_completed=true")
			return true
		var escort := unit_by_id(giver.id) as Node3D
		if escort == null:
			fail("Escort disappeared before credit")
			return false
		if not walked and escort.global_position.distance_to(start) > 5.0:
			walked = true
			await capture("02-walking.png")
		var attacked := false
		for name in AMBUSH_NAMES:
			for unit in units_named(name):
				var area: Node = unit.find_child("UnitPick", true, false)
				if area == null or unit.global_position.distance_to(player_position()) > 35.0:
					continue
				if not seen_ambushes.has(name):
					seen_ambushes[name] = true
					await capture("03-ambush-%s.png" % name.to_lower().replace(" ", "-"))
				var id: int = area.get_meta("unit_server_id")
				if client.unit_alive(id):
					await cast_live_spell(5176, id)
					await create_timer(2.5).timeout
					await cast_live_spell(5185, giver.id)
					await create_timer(3.5).timeout
					attacked = true
					break
			if attacked:
				break
		if not attacked and player_position().distance_to(escort.global_position) > 8.0:
			var to_escort: Vector3 = escort.global_position - player_position()
			var heading := rad_to_deg(atan2(to_escort.x, to_escort.z))
			if not await send_live_cli(["movement", "forward", "--seconds", "1.0", "--yaw-degrees", str(heading)]):
				return false
		step += 1
		if step % 10 == 0:
			print("ESCORT_TRACE position=", escort.global_position, " quest=", entry, " waves=", seen_ambushes.keys())
		await create_timer(0.2).timeout
	await capture("timeout.png")
	fail("Escort976 did not complete in600s; inspect authored path/ambush trace")
	return false

func abandon_previous_escort() -> bool:
	await tap(KEY_L)
	await click_control(quest_control("WorldMapUI", "QuestLogTitle976"))
	await click_control(quest_control("WorldMapUI", "QuestMapFrameAbandonButton"))
	if not await wait_frames(func(): return client.find_child("StaticPopup1Button1", true, false) != null, "previous attempt abandon confirmation"):
		return false
	await click_control(quest_control("StaticPopupUI", "StaticPopup1Button1"))
	if not await wait_quest(func(s): return not in_log(s, ESCORT_QUEST), "previous failed escort abandoned"):
		return false
	await tap(KEY_L)
	return true

func cast_live_spell(spell: int, target: int) -> void:
	await send_live_cli(["spell", "cast", "--spell", str(spell), "--target", str(target)])

func send_live_cli(arguments: Array) -> bool:
	var socket := "/tmp/game-engine-%d.sock" % OS.get_process_id()
	var argv := PackedStringArray(["--socket", socket] + arguments)
	var pid := OS.create_process(OS.get_environment("QF_CLI"), argv, false)
	if pid < 0:
		fail("Cannot launch owned native IPC command")
		return false
	var deadline := Time.get_ticks_msec() + 15000
	while OS.is_process_running(pid) and Time.get_ticks_msec() < deadline:
		await process_frame
	if OS.is_process_running(pid):
		OS.kill(pid)
		fail("Owned native IPC command timed out: " + str(arguments))
		return false
	var code := OS.get_process_exit_code(pid)
	if code != 0:
		fail("Owned native IPC command failed: %s exit%d" % [arguments, code])
		return false
	return true
