extends SceneTree

# Renders one authored RegistryUi screen standalone for visual comparison.
# GODOT_CAPTURE_SCREEN: character_select | character_create | portrait_party | forever_portrait_party.
# GODOT_CAPTURE_PATH: PNG output.
# castbaranim_preview: GODOT_CASTBAR_SKIN, GODOT_CASTBAR_PHASE, GODOT_CASTBAR_TIME.

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var screen = OS.get_environment("GODOT_CAPTURE_SCREEN")
	var output = OS.get_environment("GODOT_CAPTURE_PATH")
	if screen == "sidebarbinds_both":
		var captured: bool = await capture_sidebarbinds_both(output)
		quit(0 if captured else 1)
		return
	if screen == "spellbook_both":
		var captured: bool = await capture_spellbook_both(output)
		quit(0 if captured else 1)
		return
	if screen == "auction_both":
		var captured: bool = await capture_auction_both(output)
		quit(0 if captured else 1)
		return
	if screen == "hudedit_both":
		await capture_hud_edit_both(output)
		return
	if screen == "npcportraits" or screen == "forever_npcportraits":
		await capture_npcportraits(output, screen.begins_with("forever"))
		return
	var ui = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var method = "show_" + screen
	if output.is_empty() or not ui.has_method(method):
		push_error("Set GODOT_CAPTURE_PATH and a valid GODOT_CAPTURE_SCREEN")
		quit(1)
		return
	var error = ui.call(method)
	if error != "":
		push_error(error)
		quit(1)
		return
	var uipolish: bool = screen in ["chatflush_preview", "achievement_preview", "forever_achievement_preview"]
	if screen == "chatflush_preview":
		RenderingServer.set_default_clear_color(Color(0.25, 0.4, 0.55))
	var trainer_preview: bool = screen in ["trainer_preview", "forever_trainer_preview"]
	var settle_frames: int = 240 if trainer_preview else 120 if screen in ["spellbook_preview", "forever_spellbook_preview", "auction_icons_preview", "forever_auction_icons_preview", "forever_damage_meter_preview", "achievement_preview", "forever_achievement_preview", "castbaranim_preview", "actionbars_options_preview", "forever_actionbars_options_preview", "forever_hudedit_preview"] else 3
	for frame in range(settle_frames):
		await process_frame
		if screen in ["trainer_preview", "forever_trainer_preview"]:
			var portrait = ui.find_child("TrainerPreviewPortrait", true, false)
			if portrait != null:
				var portrait_error = portrait.call("tick")
				if portrait_error != "":
					push_error(portrait_error)
					quit(1)
					return
		if uipolish:
			RenderingServer.force_draw()
		else:
			await RenderingServer.frame_post_draw
	if screen == "chatflush_preview":
		if not chatflush_corner_and_input(ui):
			ui.queue_free()
			quit(1)
			return
		await process_frame
		RenderingServer.force_draw()
	if trainer_preview and not OS.get_environment("GODOT_TRAINER_HOVER_SPELL").is_empty():
		if not await trainer_hover_content(ui):
			ui.queue_free()
			quit(1)
			return
	var image = root.get_texture().get_image()
	if image == null or image.is_empty() or image.save_png(output) != OK:
		push_error("UI capture requires a rendering display")
		quit(1)
		return
	if screen in ["achievement_preview", "forever_achievement_preview"]:
		if not achievement_header_matches_retail(ui, image):
			ui.queue_free()
			quit(1)
			return
	if screen == "chatflush_preview":
		if not await chat_backdrop_is_translucent(ui, output):
			ui.queue_free()
			quit(1)
			return
	if screen == "forever_damage_meter_preview":
		if not forever_meter_matches_reference(ui, image):
			ui.queue_free()
			quit(1)
			return
	if screen in ["portrait_party", "forever_portrait_party"]:
		if not offline_party_health_is_desaturated(image):
			ui.queue_free()
			quit(1)
			return
	if screen in ["guild_ranks_preview", "forever_guild_ranks_preview"]:
		if not guild_rank_footer_has_two_lines(image):
			ui.queue_free()
			quit(1)
			return
	if screen == "forever_minimap_preview":
		if not await forever_minimap_has_opaque_header_and_badge(ui, image, output):
			ui.queue_free()
			quit(1)
			return
	if screen == "castbaranim_preview":
		if not await castbar_snapshot_matches(ui, image, output):
			ui.queue_free()
			quit(1)
			return
	if screen in ["bosslayout_preview", "forever_bosslayout_preview"]:
		if not bosslayout_geometry_matches(ui):
			ui.queue_free()
			quit(1)
			return
	if screen in ["professions_preview", "forever_professions_preview"]:
		if not professions_geometry_and_content_match(ui):
			ui.queue_free()
			quit(1)
			return
	if screen in ["trainer_preview", "forever_trainer_preview"] and OS.get_environment("GODOT_TRAINER_EXPECT_RETAIL") == "1":
		if not trainer_geometry_and_content_match(ui):
			ui.queue_free()
			quit(1)
			return
	if screen in ["auction_icons_preview", "forever_auction_icons_preview"] and OS.get_environment("GODOT_AH_EXPECT_UNKNOWN") == "1":
		if not auction_missing_icons_match_reference(ui):
			ui.queue_free()
			quit(1)
			return
	print("PASS: rendered ", screen, " captured")
	ui.queue_free()
	quit(0)

# One cage run, real compositor mode required (not only a requested window size).
func capture_sidebarbinds_both(directory: String) -> bool:
	for entry in [["sidebarbinds_preview", "modern.png"], ["forever_sidebarbinds_preview", "forever.png"]]:
		var ui = ClassDB.instantiate("RegistryUi")
		root.add_child(ui)
		var error: String = ui.call("show_" + entry[0])
		if not error.is_empty():
			push_error(error)
			ui.queue_free()
			return false
		for frame in range(120):
			await process_frame
			await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		if image.get_size() != Vector2i(1920, 1080) or DisplayServer.window_get_size() != Vector2i(1920, 1080):
			push_error("Sidebar capture requires real 1920x1080 window and framebuffer")
			ui.queue_free()
			return false
		for bar in ["MultiBarBottomLeft", "MultiBarBottomRight", "MultiBarRight", "MultiBarLeft"]:
			var button := ui.find_child(bar + "Button1", true, false) as Control
			var hotkey := ui.find_child(bar + "Button1HotKey", true, false) as Control
			if button == null or hotkey == null or not button.is_visible_in_tree() or not hotkey.is_visible_in_tree():
				push_error("Enabled sidebar button/hotkey missing: " + bar)
				ui.queue_free()
				return false
			print("PASS: ", entry[0], " ", bar, " button=", button.get_global_rect(), " hotkey=", hotkey.get_global_rect())
		if image.save_png(directory.path_join(entry[1])) != OK:
			ui.queue_free()
			return false
		print("PASS: ", entry[0], " rendered 1920x1080")
		ui.queue_free()
		await process_frame
	return true

# UI textures load asynchronously. Capture both HUD skins only after native visuals settle.
func capture_hud_edit_both(directory: String) -> void:
	var captured: bool = await capture_hud_edit_both_into(self, directory)
	quit(0 if captured else 1)

# The private live fixture reuses this exact offline capture path before creating GameClient.
static func capture_hud_edit_both_into(tree: SceneTree, directory: String) -> bool:
	for entry in [["hudedit_preview", "modern-edit.png"], ["forever_hudedit_preview", "forever-edit.png"]]:
		var ui = ClassDB.instantiate("RegistryUi")
		tree.root.add_child(ui)
		var error = ui.call("show_" + entry[0])
		if error != "":
			push_error(error)
			ui.queue_free()
			return false
		for frame in range(120):
			await tree.process_frame
			await RenderingServer.frame_post_draw
		var background := ui.find_child("EditModeSelection_player_frameBackground", true, false) as Control
		if background == null or not background.is_visible_in_tree() or abs(background.modulate.a - 0.7) > 0.001:
			push_error("Player mover highlight is not visibly projected at 0.7 opacity")
			ui.queue_free()
			return false
		var image = tree.root.get_texture().get_image()
		if image == null or image.is_empty() or image.save_png(directory.path_join(entry[1])) != OK:
			push_error("HUD edit capture requires rendered pixels")
			ui.queue_free()
			return false
		print("PASS: settled ", entry[0], " captured, highlight opacity=", background.modulate.a)
		if OS.get_environment("GODOT_HUDEDIT_MOVER_INVENTORY") == "1":
			if not await capture_hud_edit_movers(tree, ui, directory, entry[1].trim_suffix("-edit.png")):
				ui.queue_free()
				return false
		ui.call("finish_hudedit_preview")
		ui.queue_free()
		await tree.process_frame
	return true

# Offline selected-label pixel inventory, using the same registered roots as the client.
static func capture_hud_edit_movers(tree: SceneTree, ui: Node, directory: String, skin: String) -> bool:
	var keys: PackedStringArray = ui.call("hudedit_preview_keys")
	if keys.size() != 19:
		push_error("Default HUD mover inventory excludes disabled Action Bars 4 and 5 (19 systems)")
		return false
	var crops: Array[Image] = []
	for key in keys:
		var error: String = ui.call("select_hudedit_preview", key)
		if not error.is_empty():
			push_error(error)
			return false
		for frame in range(3):
			await tree.process_frame
			await RenderingServer.frame_post_draw
		var label := ui.find_child("EditModeSelection_" + key + "Label", true, false) as Control
		var manager := ui.find_child("EditModeManagerFrame", true, false) as Control
		if label == null or manager == null or not label.is_visible_in_tree():
			push_error("Selected HUD label missing: ", key)
			return false
		var rect := label.get_global_rect()
		if not hud_edit_label_is_foreground(ui, label):
			return false
		if rect.intersects(manager.get_global_rect()):
			push_error("Selected HUD label covered by manager: ", key)
			return false
		var image := tree.root.get_texture().get_image()
		var region := Rect2i(rect.grow(8)).intersection(Rect2i(Vector2i.ZERO, image.get_size()))
		var crop := image.get_region(region)
		crops.append(crop)
		if crop.save_png(directory.path_join(skin + "-mover-" + key + ".png")) != OK:
			push_error("HUD mover pixel capture failed: ", key)
			return false
		if key == "objective_tracker" and image.save_png(directory.path_join(skin + "-tracker-selected.png")) != OK:
			return false
		print("PASS: ", skin, " selected mover ", key, " label=", rect)
	var sheet := Image.create(1280, 1210, false, crops[0].get_format())
	sheet.fill(Color(0.1, 0.1, 0.1, 1.0))
	for index in range(crops.size()):
		var crop := crops[index]
		var cell := Vector2i((index % 2) * 640, (index / 2) * 110)
		var position := cell + Vector2i((640 - crop.get_width()) / 2, 20)
		sheet.blit_rect(crop, Rect2i(Vector2i.ZERO, crop.get_size()), position)
	return sheet.save_png(directory.path_join(skin + "-all-19-labels.png")) == OK

# Actual native image controls must not paint over any selected system label.
static func hud_edit_label_is_foreground(ui: Node, label: Control) -> bool:
	for node in ui.find_children("*", "Control", true, false):
		var paint := node as Control
		if not (paint is TextureRect or paint is ColorRect):
			continue
		if not paint.is_visible_in_tree() or paint.modulate.a <= 0.0:
			continue
		if paint.get_global_rect().intersects(label.get_global_rect()) and hud_edit_paint_depth(paint) > hud_edit_paint_depth(label):
			push_error("Native paint covers selected HUD label: ", label.name, " by ", paint.name)
			return false
	return true

static func hud_edit_paint_depth(item: CanvasItem) -> int:
	var parent := item.get_parent()
	var inherited := hud_edit_paint_depth(parent) if item.z_as_relative and parent is CanvasItem else 0
	return item.z_index + inherited

# Cached Retail TrainerUI.xml:28-105,127-219; TrainerUI.lua:187-307.
func trainer_geometry_and_content_match(ui: Node) -> bool:
	var portrait_host = ui.find_child("TrainerPreviewPortrait", true, false)
	if portrait_host == null:
		push_error("Trainer preview lacks the real masked portrait host")
		return false
	var portrait_state: Dictionary = portrait_host.call("portrait_state")
	if not portrait_state.get("visible", false) or not portrait_state.get("mask_loaded", false) or not portrait_state.get("model_shown", false):
		push_error("Trainer offline portrait model/mask did not settle")
		return false
	if portrait_state.get("mask_fdid", 0) != 130924:
		push_error("Trainer portrait must use Retail CircleMask")
		return false
	var frame := ui.find_child("ClassTrainerFrame", true, false) as Control
	var list := ui.find_child("ClassTrainerScrollBox", true, false) as Control
	var row := ui.find_child("ClassTrainerService2963", true, false) as Control
	var icon := ui.find_child("ClassTrainerService2963Icon", true, false) as Control
	var name := ui.find_child("ClassTrainerService2963Name", true, false) as Label
	var train := ui.find_child("ClassTrainerTrainButton", true, false) as Control
	var filter := ui.find_child("ClassTrainerFilterDropdown", true, false) as Control
	if frame == null or list == null or row == null or icon == null or name == null or train == null or filter == null:
		push_error("Trainer preview lacks production frame/list/row/icon/footer/filter")
		return false
	if frame.size != Vector2(338, 424) or list.size != Vector2(302, 330) or row.size != Vector2(298, 47) or icon.size != Vector2(36, 36):
		push_error("Trainer frame/list/row/icon dimensions differ from Retail")
		return false
	var background := ui.find_child("ClassTrainerTrainerBackground", true, false) as Control
	if background == null or background.size != Vector2(308, 338) or background.position != Vector2(6, 61):
		push_error("Trainer background must expand the ScrollBox by -3,+4 / +3,-4")
		return false
	if train.size != Vector2(80, 22) or filter.size != Vector2(100, 18):
		push_error("Trainer footer/filter dimensions differ from Retail")
		return false
	if list.get_global_rect().position - frame.get_global_rect().position != Vector2(9, 65):
		push_error("Trainer list no longer follows the Retail inset")
		return false
	print("TRAINER_NATIVE icon=", icon.position, " name=", name.position, " row=", row.get_global_rect(), " list=", list.get_global_rect())
	# Centred 36px icon in a 47px row has a half-pixel anchor; native image/text rasterization snaps independently.
	if icon.position.x != 6 or name.position.x != 48 or absf(icon.position.y - 5.5) > 0.5 or absf(name.position.y - 6.5) > 0.5:
		push_error("Trainer row icon/name anchors differ from Retail")
		return false
	if name.text != "Bolt of Linen Cloth" or not name.get_theme_color("font_color").is_equal_approx(Color.html("ffd200")):
		push_error("Trainer selected row must retain neutral Retail gold name")
		return false
	for entry in [[0, "1"], [1, "25"], [2, "50"]]:
		var amount := ui.find_child("ClassTrainerService2963CostAmount%d" % entry[0], true, false) as Label
		var coin := ui.find_child("ClassTrainerService2963CostCoin%d" % entry[0], true, false) as Control
		if amount == null or coin == null or amount.text != entry[1] or not amount.get_theme_color("font_color").is_equal_approx(Color.html("ff2020")):
			push_error("Trainer unaffordable cost must show red denominations with coins")
			return false
		if amount.get_global_rect().end.x > coin.get_global_rect().position.x + 0.1 or coin.size != Vector2(13, 13):
			push_error("Trainer coin overlaps its amount or has wrong size")
			return false
	var known := ui.find_child("ClassTrainerService3275Requirements", true, false) as Label
	if known == null or known.text != "Already known" or ui.find_child("ClassTrainerService3275CostAmount0", true, false) != null:
		push_error("Trainer known service must show Already known without money")
		return false
	if ui.find_child("ClassTrainerService2963Selected", true, false) == null or ui.find_child("ClassTrainerGreeting", true, false) != null:
		push_error("Trainer needs texture selection and no invented greeting pane")
		return false
	print("PASS: trainer338x424/list302x330/rows298x47/icon36/footer80x22/filter100x18; gold names; selected texture; red1g25s50c; known without money")
	return true

# Retail ProfessionsRecipeList.xml:94-217; ReagentSlotBase.xml:6-27; RankBar.xml:5-61.
func professions_geometry_and_content_match(ui: Node) -> bool:
	var frame := ui.find_child("ProfessionsFrame", true, false) as Control
	var slot := ui.find_child("ProfessionReagent0Slot", true, false) as Control
	var name := ui.find_child("ProfessionReagent0", true, false) as Label
	var count := ui.find_child("ProfessionReagent0Count", true, false)
	var sufficient := ui.find_child("ProfessionReagent1", true, false) as Label
	var rank := ui.find_child("ProfessionsSkillText", true, false) as Label
	var output_slot := ui.find_child("ProfessionsOutputSlot", true, false) as Control
	if frame == null or slot == null or name == null or sufficient == null or rank == null or output_slot == null:
		push_error("Profession preview lacks production frame/slots/rank")
		return false
	if frame.size != Vector2(942, 658) or slot.size != Vector2(39, 39) or output_slot.size != Vector2(54, 54):
		push_error("Profession frame or item slot geometry differs")
		return false
	if name.text != "2/3 Bolt of Linen Cloth" or sufficient.text != "4/1 Coarse Thread" or rank.text != "Classic Tailoring 35/300":
		push_error("Profession preview snapshot content differs")
		return false
	if name.get_global_rect().position.x < slot.get_global_rect().end.x:
		push_error("Reagent name overlaps item slot")
		return false
	if count != null:
		push_error("Owned/needed count must not be drawn on reagent icon")
		return false
	if not name.get_theme_color("font_color").is_equal_approx(Color.html("a0a0a0")) or not sufficient.get_theme_color("font_color").is_equal_approx(Color.WHITE):
		push_error("Reagent count/name availability color differs")
		return false
	var skill_ups := {3275: false, 3276: true, 2963: true, 2964: true, 7623: true, 7624: true}
	for spell in skill_ups:
		var recipe := ui.find_child("ProfessionRecipe%dLabel" % spell, true, false) as Label
		if recipe == null or not recipe.get_theme_color("font_color").is_equal_approx(Color.html("e2dcd6")):
			push_error("Native recipe neutral label color differs: ", spell)
			return false
		var indicator := ui.find_child("ProfessionRecipe%dSkillUp" % spell, true, false)
		if (indicator != null) != skill_ups[spell]:
			push_error("Recipe skill-up indicator differs: ", spell)
			return false
	print("PASS: professions 942x658, neutral selected/unselected labels, skill-up indicators, adjacent 2/3 and 4/1 names with availability colors, 39px reagent/54px output slots, rank35/300")
	return true

# Retail TargetFrame.xml:367-370; TargetFrame.lua:957-966,1015-1020.
func bosslayout_geometry_matches(ui: Node) -> bool:
	var tracker := ui.find_child("ObjectiveTrackerFrame", true, false) as Control
	var minimap := ui.find_child("MinimapCluster", true, false) as Control
	if tracker == null or minimap == null:
		push_error("Boss preview requires production tracker and minimap")
		return false
	var tracker_rect := tracker.get_global_rect()
	for index in range(1, 6):
		var boss := ui.find_child("Boss%dTargetFrame" % index, true, false) as Control
		if boss == null or boss.is_visible_in_tree() != (index <= 2):
			push_error("Boss preview must show exactly two engaged bosses")
			return false
		if index > 2:
			continue
		var boss_rect := boss.get_global_rect()
		print("BOSSLAYOUT_NATIVE boss", index, "=", boss_rect, " tracker=", tracker_rect)
		if boss_rect.size != Vector2(133, 51) or tracker_rect.position.y <= boss_rect.end.y:
			push_error("Boss size or managed tracker position differs")
			return false
		if boss_rect.intersects(tracker_rect) or boss_rect.intersects(minimap.get_global_rect()):
			push_error("Boss overlaps tracker or minimap")
			return false
	return true

# Retail Mainline/Blizzard_AchievementUI.lua:298 and XML:1973-1984.
func achievement_header_matches_retail(ui: Node, image: Image) -> bool:
	var points := ui.find_child("AchievementFrameHeaderPoints", true, false) as Label
	var shield := ui.find_child("AchievementFrameHeaderShield", true, false) as Control
	if points == null or shield == null or points.text != "10" or not shield.is_visible_in_tree():
		push_error("Retail achievement header requires 10 points and a visible shield")
		return false
	var text_rect := points.get_global_rect()
	var shield_rect := shield.get_global_rect()
	print("ACHIEVEMENT_HEADER_NATIVE points=", text_rect, " shield=", shield_rect)
	if shield_rect.size != Vector2(20, 20) or absf(shield_rect.position.x - text_rect.end.x - 3.0) > 0.01:
		push_error("Achievement shield size/right gap differs from Retail")
		return false
	if absf(shield_rect.get_center().y - text_rect.get_center().y - 1.0) > 0.01:
		push_error("Achievement shield vertical offset differs from Retail")
		return false
	var white_pixels := 0
	var gold_pixels := 0
	for y in range(int(text_rect.position.y), int(text_rect.end.y)):
		for x in range(int(text_rect.position.x), int(text_rect.end.x)):
			var color := image.get_pixel(x, y)
			if minf(color.r, minf(color.g, color.b)) > 0.75:
				white_pixels += 1
	for y in range(int(shield_rect.position.y), int(shield_rect.end.y)):
		for x in range(int(shield_rect.position.x), int(shield_rect.end.x)):
			var color := image.get_pixel(x, y)
			if color.r > 0.4 and color.g > 0.2 and color.b < 0.3:
				gold_pixels += 1
	if white_pixels < 10 or gold_pixels < 10:
		push_error("Achievement header did not render: white=", white_pixels, " gold=", gold_pixels)
		return false
	print("PASS: Retail achievement header 10, 20x20 shield, right gap 3 and vertical offset 1; white=", white_pixels, " gold=", gold_pixels)
	return true

# Verify FlareUI's exact native tint and visible transmission over black/white.
# The Blizzard texture has its own alpha; final pixels are not a flat 60% fill.
func chat_backdrop_is_translucent(ui: Node, output: String) -> bool:
	var panel: Rect2 = ui.find_child("ChatFrame1FlareSkin", true, false).get_global_rect()
	var controls: Array[Image] = []
	for background in [Color.BLACK, Color.WHITE]:
		RenderingServer.set_default_clear_color(background)
		await process_frame
		RenderingServer.force_draw()
		controls.append(root.get_texture().get_image())
	if controls[1].save_png(output.get_basename() + "-white-background.png") != OK:
		push_error("Cannot save chat translucency control")
		return false
	var center := ui.find_child("ChatFrame1FlareSkin", true, false).find_child("Part0", true, false) as TextureRect
	if center == null or not center.self_modulate.is_equal_approx(Color(1, 1, 1, 0.6)):
		push_error("Chat backdrop tint differs from FlareUI white at alpha 0.6")
		return false
	for offset in [Vector2(15, 50), Vector2(200, 50), Vector2(400, 50)]:
		var sample := Vector2i(panel.position + offset)
		var difference := controls[1].get_pixelv(sample) - controls[0].get_pixelv(sample)
		for channel in [difference.r, difference.g, difference.b]:
			if channel < 0.39 or channel > 0.99:
				push_error("Chat backdrop must be translucent, not opaque or absent: ", sample, " ", difference)
				return false
	print("PASS: rendered Forever backdrop preserves texture alpha with white tint at alpha 0.6")
	return true

# Production native controls: corner compensation, no neighbour overlap, usable input.
func chatflush_corner_and_input(ui: Node) -> bool:
	var panel = ui.find_child("ChatFrame1FlareSkin", true, false).get_global_rect()
	var input = ui.find_child("ChatFrame1EditBox", true, false)
	var input_rect: Rect2 = input.get_global_rect()
	print("CHATFLUSH_NATIVE panel ", panel, "; input ", input_rect)
	if absf(panel.position.x + 2.0) > 0.01 or absf(panel.end.y - 2.0 - 1080.0) > 0.01:
		push_error("Chat visible edge is not corner-flush: ", panel)
		return false
	for name in ["MainActionBar", "MultiBarBottomLeft", "MultiBarBottomRight", "MainActionBarLeftEndCap", "MainActionBarRightEndCap", "PetActionBar", "PlayerFrame", "BagsBar", "DamageMeterFlareSkin"]:
		var other: Rect2 = ui.find_child(name, true, false).get_global_rect()
		print("CHATFLUSH_NATIVE ", name, " ", other)
		if panel.intersects(other) or input_rect.intersects(other):
			push_error("Chat/input overlaps ", name, ": ", other)
			return false
	if not Rect2(0, 0, 1920, 1080).encloses(input_rect) or not input.is_visible_in_tree():
		push_error("Chat input not visible inside screen: ", input_rect)
		return false
	input.grab_focus()
	input.insert_text_at_caret("chatflush input proof")
	if not input.has_focus() or input.text != "chatflush input proof":
		push_error("Chat input did not accept text")
		return false
	print("PASS: Forever corner, native overlaps and focused text input")
	return true

# Assert actual rendered endpoints, native font size and native icon/title rectangles.
func forever_meter_matches_reference(ui: Node, image: Image) -> bool:
	var title = ui.find_child("DamageMeterTypeName", true, false)
	var title_rect: Rect2 = title.get_global_rect()
	for name in ["DamageMeterSettingsIcon", "DamageMeterSessionDropdownIcon"]:
		var icon = ui.find_child(name, true, false)
		var icon_rect: Rect2 = icon.get_global_rect()
		if absf(icon_rect.get_center().y - title_rect.get_center().y) > 0.5:
			push_error(name, " not centred on title: ", icon_rect, " vs ", title_rect)
			return false
	var threat = ui.find_child("DamageMeterThreatTabName", true, false)
	if threat == null or threat.text != "Threat":
		push_error("Default second tab is not Threat")
		return false
	var expected_colors = [Color(0.25, 0.78, 0.92), Color(0.53, 0.53, 0.93), Color(0.96, 0.55, 0.73), Color(0.78, 0.61, 0.43), Color(0.0, 0.44, 0.87)]
	for row in range(1, 6):
		for suffix in ["Name", "Value"]:
			var label = ui.find_child("DamageMeterEntry%d%s" % [row, suffix], true, false)
			if label.get_theme_font_size("font_size") != 12:
				push_error("Row font is not 12")
				return false
		var fill = ui.find_child("DamageMeterEntry%dStatusBar" % row, true, false)
		var rect: Rect2 = fill.get_global_rect()
		for stop in [0, 1]:
			var x = rect.position.x + 2 if stop == 0 else rect.end.x - 3
			var expected: Color = expected_colors[row - 1] * (0.5 if stop == 0 else 1.0)
			for y in [rect.position.y + 4, rect.end.y - 5]:
				var pixel = image.get_pixel(int(x), int(y))
				if maxf(absf(pixel.r - expected.r), maxf(absf(pixel.g - expected.g), absf(pixel.b - expected.b))) > 0.035:
					push_error("Fill stop/highlight mismatch row ", row, " stop ", stop, " at ", Vector2(x, y), ": ", pixel, " vs ", expected)
					return false
	print("PASS: Forever meter rendered stops, fonts, Threat tab and icon centres")
	return true

# Production display begins below the 22-unit band; no tile pixels leak into it.
# Badge's gold ring must draw inside its derived rect, with empty space to the magnifier.
func forever_minimap_has_opaque_header_and_badge(ui: Node, image: Image, output: String) -> bool:
	if image.get_size() != Vector2i(1920, 1080):
		push_error("Minimap capture requires 1920x1080")
		return false
	for x in range(1700, 1900):
		var color = image.get_pixel(x, 29)
		if maxf(color.r, maxf(color.g, color.b)) > 1.0 / 255.0:
			push_error("Map leaked into the minimap title band at ", x, ": ", color)
			return false
	var badge = ui.find_child("MinimapDayNightBadge", true, false) as Control
	var launcher = ui.find_child("MinimapLauncherButton", true, false) as Control
	var band = ui.find_child("MinimapTitleBand", true, false) as Control
	var tracker = ui.find_child("ObjectiveTrackerFrameHeaderBackground", true, false) as Control
	if badge == null or launcher == null or band == null or tracker == null:
		push_error("Minimap capture controls missing")
		return false
	var badge_rect = Rect2(1662, 226, 33, 32)
	if badge.get_global_rect() != badge_rect or launcher.get_global_rect() != Rect2(1624, 230, 30, 30):
		push_error("Badge or approved magnifier placement changed")
		return false
	if band.get_global_rect() != Rect2(1668, 8, 244, 22):
		push_error("Title band size/inset changed")
		return false
	if badge_rect.intersects(launcher.get_global_rect()) or tracker.get_global_rect().intersects(Rect2(1660, 0, 260, 260)):
		push_error("Minimap overlaps approved neighboring controls")
		return false
	badge.hide()
	for frame in range(2):
		await process_frame
		await RenderingServer.frame_post_draw
	var without_badge = root.get_texture().get_image()
	if without_badge.save_png(output.get_basename() + "-without-badge.png") != OK:
		push_error("Cannot save badge isolation control")
		return false
	badge.show()
	var changed_pixels = 0
	for y in range(200, 275):
		for x in range(1624, 1920):
			if image.get_pixel(x, y) == without_badge.get_pixel(x, y):
				continue
			if not badge_rect.has_point(Vector2(x, y)):
				push_error("Badge changed pixels outside its rect: ", Vector2i(x, y))
				return false
			changed_pixels += 1
	if changed_pixels < 100:
		push_error("Badge art did not render: ", changed_pixels)
		return false
	print("PASS: opaque 22-unit minimap band; isolated badge pixels=", changed_pixels, "; magnifier/tracker clear")
	return true

# The rank disclaimer occupies two single-line rows inside the left column.
# Assert real rendered pixels; the previous 20px multiline label overlapped both rows.
func guild_rank_footer_has_two_lines(image: Image) -> bool:
	if image.get_width() != 1920 or image.get_height() != 1080:
		push_error("Guild rank capture requires 1920x1080")
		return false
	for band in [Vector2i(742, 762), Vector2i(764, 788)]:
		var gold_pixels = 0
		for y in range(band.x, band.y):
			for x in range(459, 667):
				var color = image.get_pixel(x, y)
				if color.r > 0.6 and color.g > 0.4 and color.b < 0.35:
					gold_pixels += 1
		if gold_pixels < 10:
			push_error("Guild rank disclaimer line missing in band ", band)
			return false
	return true

# Offline captures use force_draw, avoiding the protected WSL compositor's frame callback.
func capture_npcportraits(directory: String, forever: bool) -> void:
	if directory.is_empty() or DirAccess.make_dir_recursive_absolute(directory) != OK:
		push_error("Npc portraits require a writable capture directory")
		quit(1)
		return
	var skin_name = "forever" if forever else "modern"
	for window in ["bank", "mail", "open-mail", "guild-bank", "auction", "auction-gossip", "trade", "merchant"]:
		var fixture = ClassDB.instantiate("NpcPortraitPreview")
		root.add_child(fixture)
		var error: String = fixture.initialize(window, forever)
		if not error.is_empty():
			push_error(error)
			fixture.free()
			quit(1)
			return
		var live = window in ["bank", "auction", "auction-gossip", "trade", "merchant"]
		var ready = not live
		for attempt in range(1200 if live else 4):
			error = fixture.tick()
			if not error.is_empty():
				push_error(error)
				fixture.free()
				quit(1)
				return
			await process_frame
			RenderingServer.force_draw()
			if live:
				var state: Dictionary = fixture.portrait_state()
				ready = state.get("model_shown", false) and state.get("mask_loaded", false) and not state.get("pending", true)
				if ready:
					var expected = "player Local player race 10" if window == "trade" else "player Offline NPC race 1"
					if not str(state.appearance).begins_with(expected):
						push_error("Wrong portrait source: ", state)
						fixture.free()
						quit(1)
						return
					break
		if not ready:
			push_error("Portrait failed to settle: ", window, " ", fixture.portrait_state())
			fixture.free()
			quit(1)
			return
		for frame in range(4):
			await process_frame
			RenderingServer.force_draw()
		if not npcportraits_backgrounds_clear(fixture, window):
			fixture.free()
			quit(1)
			return
		var image = root.get_texture().get_image()
		var path = directory.path_join(skin_name + "-" + window + ".png")
		if image == null or image.is_empty() or image.save_png(path) != OK:
			push_error("Cannot save portrait capture ", path)
			fixture.free()
			quit(1)
			return
		print("PASS npcportraits ", skin_name, " ", window, " ", path)
		fixture.free()
		await process_frame
	quit(0)

func npcportraits_backgrounds_clear(fixture: Node, window: String) -> bool:
	if window == "guild-bank":
		return fixture.find_child("GuildBankFramePortrait", true, false) == null
	var prefixes = {"bank": "BankFrame", "mail": "MailFrame", "open-mail": "OpenMailFrame", "auction": "AuctionHouseFrame", "auction-gossip": "AuctionGossip", "trade": "TradeFrame", "merchant": "MerchantFrame"}
	var prefix: String = prefixes[window]
	var root_control = fixture.find_child(prefix, true, false) as Control
	var mask = Rect2(root_control.get_global_rect().position + Vector2(-3, -7), Vector2(58, 58))
	var backgrounds = [prefix + "Bg", prefix + "TopTileStreaks"]
	if window == "bank":
		backgrounds.append("BankFrameBackground")
	for name in backgrounds:
		var background = fixture.find_child(name, true, false) as Control
		if background == null or background.get_global_rect().intersects(mask):
			push_error("Background crosses portrait mask: ", name)
			return false
	return true

# Concrete four-member preview: third member at (22,147+2*63), health (45,19).
# Sample right of the Offline label, inside the full offline fill under both skins.
func offline_party_health_is_desaturated(image: Image) -> bool:
	for pixel in [Vector2i(130, 294), Vector2i(130, 296)]:
		var color = image.get_pixelv(pixel)
		var spread = maxf(color.r, maxf(color.g, color.b)) - minf(color.r, minf(color.g, color.b))
		if spread > 2.0 / 255.0 or color.r < 0.1:
			push_error("Offline party health is not filled/desaturated at ", pixel, ": ", color)
			return false
	return true

# Frozen production reducer snapshots, with rendered feedback isolation for finishes.
func castbar_snapshot_matches(ui: Node, image: Image, output: String) -> bool:
	var phase = OS.get_environment("GODOT_CASTBAR_PHASE")
	var skin = OS.get_environment("GODOT_CASTBAR_SKIN")
	var track = ui.find_child("CastingBarBackground", true, false) as Control
	var fill = ui.find_child("CastingBarFill", true, false) as Control
	var label = ui.find_child("CastingBarSpellName", true, false) as Label
	if track == null or fill == null or label == null:
		push_error("Cast snapshot controls missing")
		return false
	var track_rect = track.get_global_rect()
	var fill_rect = fill.get_global_rect()
	var fraction = 0.5 if phase == "midcast" else (0.0 if phase == "channel" else 1.0)
	if absf(fill_rect.size.x - track_rect.size.x * fraction) > 0.01:
		push_error("Cast snapshot fill mismatch: ", fill_rect, " track ", track_rect)
		return false
	if phase in ["interrupted", "failed"] and label.text != ("Interrupted" if phase == "interrupted" else "Failed"):
		push_error("Cast result label mismatch: ", label.text)
		return false
	var spark = ui.find_child("CastingBarSpark", true, false) as Control
	if phase == "midcast":
		if spark == null or absf(spark.get_global_rect().get_center().x - fill_rect.end.x) > 0.01:
			push_error("Cast spark is not on the fill edge")
			return false
	elif spark != null and spark.is_visible_in_tree():
		push_error("Finished cast kept its spark")
		return false
	print("CASTBAR_NATIVE ", skin, " ", phase, " track=", track_rect, " fill=", fill_rect, " label=", label.text)
	if phase in ["interrupted", "failed"]:
		# At 100.175, cumulative Retail XML translation is (-1,-1) in canvas coordinates.
		var expected_origin = Vector2(831, 903) if skin == "modern" else Vector2(826, 781)
		if track_rect.position != expected_origin:
			push_error("Cast shake lost an axis: ", track_rect.position, " expected ", expected_origin)
			return false
	var lit_pixels = 0
	for y in range(int(track_rect.position.y), int(track_rect.end.y)):
		for x in range(int(track_rect.position.x), int(track_rect.end.x)):
			var color = image.get_pixel(x, y)
			if maxf(color.r, maxf(color.g, color.b)) > 0.25:
				lit_pixels += 1
	if lit_pixels < 20:
		push_error("Cast art did not render")
		return false
	if phase == "midcast":
		return true
	if phase in ["finish", "channel"]:
		var flash = ui.find_child("CastingBarFlash", true, false) as Control
		if flash == null or absf(flash.modulate.a - 0.5) > 0.001:
			push_error("100.100 completion flash is not half-bright")
			return false
	else:
		var glow = ui.find_child("CastingBarInterruptGlow", true, false) as Control
		if glow == null or absf(glow.modulate.a - 0.825) > 0.001:
			push_error("100.175 interruption glow does not match its 1s fade")
			return false
	for name in ["CastingBarFlash", "CastingBarEnergyGlow", "CastingBarFlakes01", "CastingBarFlakes02", "CastingBarFlakes03", "CastingBarBaseGlow", "CastingBarWispGlow", "CastingBarSparkles01", "CastingBarSparkles02", "CastingBarInterruptGlow"]:
		var effect = ui.find_child(name, true, false) as Control
		if effect != null:
			effect.hide()
	for frame in range(2):
		await process_frame
		await RenderingServer.frame_post_draw
	var control = root.get_texture().get_image()
	if control.save_png(output.get_basename() + "-without-feedback.png") != OK:
		push_error("Cannot save cast feedback isolation control")
		return false
	if phase in ["interrupted", "failed"]:
		var sample = Vector2i(int(track_rect.position.x + track_rect.size.x * 0.75), int(track_rect.get_center().y))
		var red = control.get_pixelv(sample)
		if red.r < 0.35 or red.g > red.r * 0.25 or red.b > red.r * 0.25:
			push_error("Interrupted fill is not source red at ", sample, ": ", red)
			return false
		if skin == "forever" and maxf(absf(red.r - 1.0), maxf(red.g, red.b)) > 2.0 / 255.0:
			push_error("Forever interrupted fill is not (1,0,0): ", red)
			return false
		print("PASS: interrupted rendered colour ", red, " at ", sample)
	var changed_pixels = 0
	for y in range(image.get_height()):
		for x in range(image.get_width()):
			if image.get_pixel(x, y) != control.get_pixel(x, y):
				changed_pixels += 1
	if changed_pixels < 20:
		push_error("Cast feedback did not change rendered pixels")
		return false
	print("PASS: cast feedback changed pixels=", changed_pixels)
	return true

# Both skins and every bottom page; production projection, local mage data, no networking.
func capture_spellbook_both(directory: String) -> bool:
	for skin in ["modern", "forever"]:
		for page in ["spellbook", "specialization", "talents"]:
			OS.set_environment("GODOT_SPELLBOOK_TAB", page)
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var method = "show_spellbook_preview" if skin == "modern" else "show_forever_spellbook_preview"
			var error: String = ui.call(method)
			if not error.is_empty():
				push_error(error)
				ui.queue_free()
				return false
			for frame in range(120):
				await process_frame
				await RenderingServer.frame_post_draw
			var window_size := DisplayServer.window_get_size()
			var image := root.get_texture().get_image()
			if window_size != Vector2i(1920, 1080) or image == null or image.get_size() != window_size:
				push_error("Spellbook proof requires a real 1920x1080 window and viewport; window=", window_size, " viewport=", root.size)
				ui.queue_free()
				return false
			var geometry: Dictionary = {"window": [window_size.x, window_size.y]}
			for name in ["SpellBookRoot", "PlayerSpellsTab1", "PlayerSpellsTab2", "PlayerSpellsTab3", "SpellBookCategoryTab1", "SpellBookCategoryTab2"]:
				var control := ui.find_child(name, true, false) as Control
				if control != null and control.is_visible_in_tree():
					var rect := control.get_global_rect()
					geometry[name] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
			var prefix := directory.path_join(skin + "-" + page)
			if image.save_png(prefix + ".png") != OK:
				push_error("Cannot save spellbook pixels: ", prefix)
				ui.queue_free()
				return false
			var file := FileAccess.open(prefix + ".json", FileAccess.WRITE)
			if file == null:
				push_error("Cannot save spellbook geometry: ", prefix)
				ui.queue_free()
				return false
			file.store_string(JSON.stringify(geometry, "\t"))
			file.close()
			print("CAPTURE: ", prefix, " ", geometry)
			ui.queue_free()
			await process_frame
	return true

# Both skins, production auction projection, no GameClient or networking.
func capture_auction_both(directory: String) -> bool:
	for skin in ["modern", "forever"]:
		for view in ["browse", "item", "inventory", "sell", "duration", "owned", "bids", "dialog"]:
			OS.set_environment("GODOT_AUCTION_VIEW", view)
			var ui = ClassDB.instantiate("RegistryUi")
			root.add_child(ui)
			var method = "show_auction_preview" if skin == "modern" else "show_forever_auction_preview"
			var error = ui.call(method)
			if not error.is_empty():
				push_error(error)
				ui.queue_free()
				return false
			for frame in range(120):
				await process_frame
				await RenderingServer.frame_post_draw
			var prefix = directory.path_join(skin + "-" + view)
			# The headless display server only has the dummy renderer: record geometry, no pixels.
			if DisplayServer.get_name() != "headless" and not save_root_png(prefix + ".png"):
				push_error("Auction capture requires rendered pixels: ", prefix)
				ui.queue_free()
				return false
			var geometry: Dictionary = {}
			for name in ["AuctionHouseFrame", "AuctionHouseFrameSearchBox", "AuctionHouseFrameItemBuyFrameRow1TimeLeft", "AuctionHouseFrameAuctionsFrameBidsListRow1TimeLeft", "AuctionHouseFrameItemSellFrameDurationDropdown", "AuctionHouseFrameBuyDialog"]:
				var control := ui.find_child(name, true, false) as Control
				if control != null and control.is_visible_in_tree():
					var rect = control.get_global_rect()
					geometry[name] = [rect.position.x, rect.position.y, rect.size.x, rect.size.y]
			var file = FileAccess.open(prefix + ".json", FileAccess.WRITE)
			if file == null:
				push_error("Cannot write auction geometry: ", prefix)
				ui.queue_free()
				return false
			file.store_string(JSON.stringify(geometry, "\t"))
			file.close()
			if OS.get_environment("GODOT_AUCTION_ASSERT_BID_CLEAR") == "1" and view in ["item", "bids"]:
				if not auction_bid_copper_is_clear(ui):
					ui.queue_free()
					return false
			print("CAPTURE: ", prefix, " ", geometry)
			ui.queue_free()
			await process_frame
	return true

func save_root_png(path: String) -> bool:
	var image = root.get_texture().get_image()
	return image != null and not image.is_empty() and image.save_png(path) == OK

func auction_bid_copper_is_clear(ui: Node) -> bool:
	var copper := ui.find_child("AuctionHouseFrameBidAmountCopper", true, false) as Control
	var bid := ui.find_child("AuctionHouseFrameBidButton", true, false) as Control
	if copper == null or bid == null:
		push_error("Auction bid/copper controls missing")
		return false
	var copper_rect = copper.get_global_rect()
	var bid_rect = bid.get_global_rect()
	print("BID_CLEAR: copper=", copper_rect, " bid=", bid_rect)
	if copper_rect.end.x > bid_rect.position.x:
		push_error("Bid button covers copper input: ", copper_rect.end.x, " > ", bid_rect.position.x)
		return false
	return true

# Real row pointer hit, shared tooltip host; no GameClient/server or authored tooltip strings.
func trainer_hover_content(ui: Node) -> bool:
	var spell := int(OS.get_environment("GODOT_TRAINER_HOVER_SPELL"))
	var row := ui.find_child("ClassTrainerService%d" % spell, true, false) as Control
	if row == null or not row.is_visible_in_tree():
		push_error("Requested trainer hover row is not visible")
		return false
	var host = ClassDB.instantiate("RegistryUi")
	host.layer = 8
	root.add_child(host)
	var pointer := row.get_global_rect().get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = pointer
	motion.global_position = pointer
	root.push_input(motion)
	for frame in range(6):
		await process_frame
		var error: String = ui.call("update_trainer_preview_tooltip", host, pointer)
		if not error.is_empty():
			push_error(error)
			return false
		await RenderingServer.frame_post_draw
	var title := host.find_child("TooltipTitle", true, false) as Label
	if title == null or not title.is_visible_in_tree():
		push_error("Trainer hover lacks visible tooltip title")
		return false
	var lines: PackedStringArray = []
	for label in host.find_children("TooltipLine*", "Label", true, false):
		if label.is_visible_in_tree():
			lines.append(label.text)
	if not lines.has("Spell ID: %d" % spell):
		push_error("Trainer tooltip lost Spell ID")
		return false
	if spell == 2963:
		if title.text != "Bolt of Linen Cloth" or not lines.has("Linen Cloth (2)") or not lines.has("Item ID: 2996") or not lines.has("Requires Classic Tailoring (1)"):
			push_error("Recipe tooltip lacks crafted item/reagent/requirement content: " + str(lines))
			return false
	elif spell == 116:
		if title.text != "Frostbolt" or not lines.has("40 yd range"):
			push_error("Spell tooltip lacks full shared spell content: " + str(lines))
			return false
		var words: PackedStringArray = []
		for line in lines:
			words.append_array(line.split(" ", false))
		if not " ".join(words).contains("Frost damage"):
			push_error("Class spell tooltip lacks rendered description")
			return false
	elif spell == 212205:
		# Class create-item spell: keeps its spell tooltip, never the produced item's.
		var words: PackedStringArray = []
		for line in lines:
			words.append_array(line.split(" ", false))
		if title.text != "Create: Crimson Vial" or not " ".join(words).contains("share with allies"):
			push_error("Class create-item spell lacks its spell tooltip: " + str(lines))
			return false
		for line in lines:
			if line.begins_with("Item ID:"):
				push_error("Class create-item spell shows produced item content: " + str(lines))
				return false
	print("PASS: hovered trainer service ", spell, " title=", title.text, " lines=", lines)
	return true

# Compare actual bound native pixels, not RSX declarations. The first three files
# are absent in the regression asset set; rows 4/5 are known good controls.
func auction_missing_icons_match_reference(ui: Node) -> bool:
	var reference := auction_row_icon(ui, 14)
	if reference == null:
		push_error("Question-mark reference did not load")
		return false
	var expected := reference.get_image().get_data()
	for row in [1, 2, 3]:
		var texture := auction_row_icon(ui, row)
		if texture == null or texture.get_image().get_data() != expected:
			push_error("Missing auction row %d must bind INV_Misc_QuestionMark" % row)
			return false
	for row in [4, 5]:
		var texture := auction_row_icon(ui, row)
		if texture == null or texture.get_image().get_data() == expected:
			push_error("Available auction icon must retain its own pixels")
			return false
	print("PASS: three missing auction icons bind question-mark pixels; two good icons preserved")
	return true

func auction_row_icon(ui: Node, row: int) -> Texture2D:
	var frame := ui.find_child("AuctionHouseFrameBrowseResultsRow%dItemIcon" % row, true, false)
	if frame == null:
		return null
	var texture := frame.get_node_or_null("Parts/Part0") as TextureRect
	return null if texture == null else texture.texture
