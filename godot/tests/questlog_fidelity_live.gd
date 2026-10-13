extends "res://tests/world_quest_flow.gd"

# Reuses real Northshire acceptance, progress and POI steps. Run with:
# QF_STEPS=check_skin,check_pickup,check_progress,check_map_objectives,check_quest_log_fidelity
# QF_SKIN=modern|forever. Owned private server only; no invented quest snapshots.
func check_skin() -> bool:
	var error: String = client.preview_world_skin(OS.get_environment("QF_SKIN") == "forever")
	if not error.is_empty():
		fail(error)
		return false
	return true

func check_quest_log_fidelity() -> bool:
	await tap(KEY_L)
	if not await wait_quest(func(s): return s.log_open, "L opens map quest dock"):
		return false
	if client.get_node_or_null("QuestLogUI") != null:
		fail("Standalone QuestLogUI survived migration")
		return false
	var map := client.get_node_or_null("WorldMapUI")
	if map == null or not quest_geometry(false):
		return false
	await capture("fidelity-list.png")
	await click_control(quest_control("WorldMapUI", "QuestLogTitle%d" % QUEST))
	if not await wait_frames(func(): return client.world_map_state().get("quest_details", false), "details replace list"):
		return false
	if quest_control("WorldMapUI", "QuestLogTitle%d" % QUEST) != null or not quest_geometry(true):
		fail("Quest list remains alongside details")
		return false
	await capture("fidelity-details.png")

	await click_control(quest_control("WorldMapUI", "QuestMapFrameTrackButton"))
	if not await wait_quest(func(s): return not Array(s.watched).has(QUEST), "server untrack"):
		return false
	await click_control(quest_control("WorldMapUI", "QuestMapFrameTrackButton"))
	if not await wait_quest(func(s): return Array(s.watched).has(QUEST), "server track"):
		return false
	await click_control(quest_control("WorldMapUI", "QuestMapFrameShareButton"))
	if not await capture_error("You are not in a party.", "fidelity-share-rejection.png"):
		return false

	await click_control(quest_control("WorldMapUI", "QuestMapFrameBackButton"))
	if not await wait_frames(func(): return not client.world_map_state().get("quest_details", true), "Back returns list"):
		return false
	await tap(KEY_L)
	if not await wait_quest(func(s): return not s.log_open, "L closes dock owner"):
		return false
	var poi := client.find_child("QuestBlock%dPOIButton" % QUEST, true, false) as Control
	if poi == null:
		fail("Tracked quest POI link missing")
		return false
	await click_control(poi)
	if not await wait_frames(func(): return client.world_map_state().get("quest_details", false), "tracker POI opens same dock details"):
		return false
	await capture("fidelity-tracker-link.png")

	await click_control(quest_control("WorldMapUI", "WorldMapMaximizeMinimizeButton"))
	if not await wait_frames(func(): return client.world_map_state().get("maximized", false), "map maximized"):
		return false
	await tap(KEY_L)
	if not await wait_frames(func(): return client.quest_state().log_open and not client.world_map_state().maximized, "L restores minimized dock"):
		return false
	await click_control(quest_control("WorldMapUI", "QuestMapFrameAbandonButton"))
	await frames(3)
	var yes := quest_control("StaticPopupUI", "StaticPopup1Button1")
	if yes == null or not yes.is_visible_in_tree():
		fail("Abandon confirmation missing")
		return false
	await capture("fidelity-abandon-confirm.png")
	await click_control(yes)
	if not await wait_quest(func(s): return not in_log(s, QUEST), "server abandon"):
		return false
	await capture("fidelity-abandoned.png")
	await tap(KEY_L)
	if not await wait_quest(func(s): return not s.log_open, "final L closes map owner"):
		return false
	print("PASS QUEST_LOG_FIDELITY skin=", OS.get_environment("QF_SKIN"), " L/list/details/Back/track/share-denial/POI/maximize/abandon")
	return true

func quest_geometry(details: bool) -> bool:
	var window := quest_control("WorldMapUI", "WorldMapBorderFrame")
	var dock := quest_control("WorldMapUI", "QuestMapFrame")
	if window == null or dock == null:
		fail("Map owner/dock missing")
		return false
	var scale: float = window.size.x / 1035.0
	if abs(window.size.y - 534.0 * scale) > 1.0 or dock.size.distance_to(Vector2(330, 506) * scale) > 1.0:
		fail("Owner/dock size differs from Retail XML")
		return false
	var relative: Vector2 = (dock.global_position - window.global_position) / scale
	if relative.distance_to(Vector2(702, 25)) > 1.0:
		fail("Dock anchors differ from Retail Lua: " + str(relative))
		return false
	var name := "QuestMapDetailsScrollFrame" if details else "QuestScrollFrame"
	var scroll := quest_control("WorldMapUI", name)
	var expected := Vector2(298, 430) if details else Vector2(308, 477)
	if scroll == null or scroll.size.distance_to(expected * scale) > 1.0:
		fail("Scroll viewport differs from XML: " + name)
		return false
	print("PASS XML_GEOMETRY ", name, " owner=", window.get_global_rect(), " dock=", dock.get_global_rect(), " viewport=", scroll.get_global_rect())
	return true
