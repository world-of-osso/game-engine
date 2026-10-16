extends "res://tests/questlog_fidelity_live.gd"

## Real account/UDP5931 only. Source Feero4484/quest976/path35873.
## QF_STEPS=check_skin,check_escort; QF_ACCOUNT=fb_escort_modern|fb_escort_forever.
## Ambient world population is excluded by private fixture, not actor stat edits.
const ESCORT_QUEST := 976
const ESCORT_NAME := "Feero Ironhand"
const AMBUSH_NAMES := ["Dark Strand Assassin", "Forsaken Scout", "Aligar the Tormentor", "Balizar the Umbrage", "Caedakar the Vicious"]
var seen_ambushes: Dictionary = {}

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
		fail("Fresh escort976 must be unaccepted")
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
	var step := 0
	while Time.get_ticks_msec() < deadline:
		await frames(2)
		var entry := log_entry(client.quest_state(), ESCORT_QUEST)
		if entry.get("failed", false) or player_dead():
			await capture("failed.png")
			fail("Private escort or player died before source credit")
			return false
		if entry.get("completed", false):
			if not walked or seen_ambushes.size() != AMBUSH_NAMES.size():
				fail("Credit without observed walking/all five authored ambusher identities")
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
					await face_unit(id)
					await cast_live_spell(5176, id)
					await create_timer(2.5).timeout
					attacked = true
					break
			if attacked:
				break
		if not attacked and player_position().distance_to(escort.global_position) > 8.0:
			if not await approach(giver.id, 5.0):
				return false
		step += 1
		if step % 10 == 0:
			print("ESCORT_TRACE position=", escort.global_position, " quest=", entry, " waves=", seen_ambushes.keys())
		await create_timer(0.2).timeout
	await capture("timeout.png")
	fail("Escort976 did not complete in600s; inspect authored path/ambush trace")
	return false

func cast_live_spell(spell: int, target: int) -> void:
	var unit := unit_by_id(target)
	if unit == null:
		return
	await face_unit(target)
	var point := find_unit_point(unit, target)
	if point == Vector2.INF:
		print("ESCORT_CAST not visible target=", target)
		return
	await click(point, MOUSE_BUTTON_LEFT)
	await frames(3)
	var error: String = client.use_spell(spell)
	print("ESCORT_CAST spell=", spell, " target=", target, " result=", error)
