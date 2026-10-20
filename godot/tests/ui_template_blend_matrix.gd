extends SceneTree
# One production-panel pass per skin; supplied offline states, not realm/UDP parity.

var completion
var failures: Array[String] = []

func _initialize() -> void:
	call_deferred("run_cases")

func cases(forever: bool) -> Array:
	var result: Array = []
	for row in [
		["AdventureGuide", "EncounterJournalPage0", [["EncounterJournalPage0ButtonHighlight", true]]],
		["TradingPost", "PerksProgramProduct4", [["PerksProgramRefundIcon4", true], ["PerksProgramProduct4", true]]],
		["CharacterCurrency", "TokenHeader2", [["TokenHeader2HighlightLeft", true], ["TokenHeader2HighlightMiddle", true], ["TokenHeader2HighlightRight", true]]],
		["CollectionsPetSpellSelect", "", [["PetJournalSpellSelectSelected", true]]],
		["EntranceDifficulty", "EntranceDifficultyButton2", [["EntranceDifficultyHighlight2", true]]],
		["HousingStorageBasic", "HouseEditorStored1693", [["HouseEditorStored1693HoverBackground", true]]],
		["WorldMap", "", [["WorldMapHighlight", true], ["WorldMapPin0Highlight", true], ["QuestLogTitle332Selected", false]]],
		["GuildBank", "", [["GuildBankTab1Checked", true], ["GuildBankTab1Background", false]]],
		["GuildTabard", "", [["TabardFrameEmblemTopLeft", true], ["TabardFrameEmblemTopRight", true], ["TabardFrameEmblemBottomLeft", true], ["TabardFrameEmblemBottomRight", true]]],
		["AchievementStatistics", "", [["Achievement1068Stripe", false], ["Achievement1069Stripe", true], ["AchievementCategory14807Highlight", true]]],
	]:
		result.append({"name": row[0], "method": "show_template_blend_preview", "args": [forever, row[0]], "hover": row[1], "regions": row[2]})
	result.append({"name": "Merchant", "method": "show_buttonfit_service_preview", "args": ["merchant", forever], "hover": "MerchantRepairItemButton", "regions": [["MerchantRepairItemButtonHighlight", true]]})
	result.append({"name": "Trainer", "method": "show_forever_trainer_preview" if forever else "show_trainer_preview", "args": [], "hover": "ClassTrainerService100", "regions": [["ClassTrainerService100Highlight", true]]})
	result.append({"name": "Bank", "method": "show_forever_bank_preview" if forever else "show_bank_preview", "args": [], "hover": "BankFrameItem1", "regions": [["BankFrameTab1Selected", true], ["BankFrameItem1Highlight", true]]})
	result.append({"name": "GuildControl", "method": "show_forever_guild_ranks_preview" if forever else "show_guild_ranks_preview", "args": [], "hover": "GuildControlUIRankOrderFrameRank3ShiftUpButton", "regions": [["GuildControlUIRankOrderFrameRank3ShiftUpButtonHighlight", true]]})
	result.append({"name": "Follower", "method": "show_follower_layout_preview", "args": [forever, true], "hover": "GroupFinderFrameGroupButton1", "regions": [["GroupFinderFrameGroupButton1Highlight", true]]})
	result.append({"name": "StaticPopup", "method": "show_item_confirmation_shadow_preview", "args": [forever, true, true], "hover": "", "regions": [["ShadowConfirmSpinnerSparks", true]]})
	result.append({"name": "MenuNavigation", "method": "show_forever_menu_primitives_preview" if forever else "show_menu_primitives_preview", "args": [], "hover": "ContextInspect", "regions": [["ContextInspectHighlight", true]]})
	result.append({"name": "Timeline", "class": "UiAuditProbe", "method": "mount_encounter_timer_bars", "args": [forever], "hover": "", "regions": [["EncounterTimer2Spark", true]]})
	result.append({"name": "CastingBar", "method": "show_castbaranim_preview", "args": [], "hover": "", "regions": [["CastingBarFlash", true], ["CastingBarEnergyGlow", true]]})
	result.append({"name": "Minimap", "method": "show_minimap_zoom_preview", "args": [forever, 3], "hover": "MinimapZoomIn", "regions": [["MinimapZoomIn", false]]})
	result.append({"name": "Transmog", "method": "show_template_blend_preview", "args": [forever, "Transmog"], "hover": "TransmogSlot16", "regions": [["TransmogSlot16", true]]})
	return result

func run_cases() -> void:
	root.size = Vector2i(1920, 1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self, 42)
	var output := OS.get_environment("UI_TEMPLATE_BLEND_CAPTURES")
	if not completion.require(not output.is_empty(), "UI_TEMPLATE_BLEND_CAPTURES required"):
		return
	if not completion.require(DirAccess.make_dir_recursive_absolute(output) == OK, "capture directory"):
		return
	for forever in [false, true]:
		for panel in cases(forever):
			if not await check_panel(panel, forever, output):
				completion.fail(failures.back())
				return
			if not completion.case_done():
				return
	completion.finish(true)

func frames(count: int) -> void:
	for index in range(count):
		await process_frame

func check_panel(panel: Dictionary, forever: bool, output: String) -> bool:
	var before := failures.size()
	var skin := "forever" if forever else "modern"
	OS.set_environment("GODOT_CASTBAR_SKIN", skin)
	OS.set_environment("GODOT_CASTBAR_PHASE", "finish")
	OS.set_environment("GODOT_CASTBAR_TIME", "100.1")
	OS.set_environment("GODOT_BANK_PURCHASE", "0")
	var owner: Node = ClassDB.instantiate(panel.get("class", "RegistryUi"))
	root.add_child(owner)
	var error: String = owner.callv(panel.method, panel.args)
	if not error.is_empty():
		failures.append(panel.name + ": " + error)
	await frames(8)
	var modes: Array = []
	if panel.name == "HousingStorageBasic":
		hover_control(owner, "HouseEditorBasicDecorModeButton", true)
		await frames(4)
		await wait_for_art(owner, [["HouseEditorBasicDecorModeButtonHover", true]])
		check_region(owner, "HouseEditorBasicDecorModeButtonHover", true, modes)
		hover_control(owner, "HouseEditorBasicDecorModeButton", false)
		await frames(4)
	if not panel.hover.is_empty():
		hover_control(owner, panel.hover, true)
	await frames(4)
	await wait_for_art(owner, panel.regions)
	for region in panel.regions:
		check_region(owner, region[0], region[1], modes)
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := output.path_join(skin + "-" + panel.name + ".png")
	if image == null or image.save_png(path) != OK:
		failures.append(panel.name + " capture failed")
	print("UI_TEMPLATE_BLEND_NATIVE ", JSON.stringify({"skin": skin, "panel": panel.name, "modes": modes, "capture": path, "failures": failures.slice(before)}))
	owner.free()
	await frames(2)
	return failures.size() == before

func hover_control(owner: Node, name: String, entering: bool) -> void:
	var target := owner.find_child(name, true, false) as Control
	if target == null or not target.is_visible_in_tree():
		failures.append("missing visible hover owner " + name)
		return
	# Standard signal/input queue, never a registry or material override.
	target.emit_signal("mouse_entered" if entering else "mouse_exited")
	var error: String = owner.call("sync_input")
	if not error.is_empty():
		failures.append(name + " hover: " + error)

func wait_for_art(owner: Node, regions: Array) -> void:
	var deadline := Time.get_ticks_msec() + 30000
	while Time.get_ticks_msec() < deadline:
		var ready := true
		for expected in regions:
			var region: Node = owner.find_child(expected[0], true, false)
			var parts: Node = region.get_node_or_null("Parts") if region != null else null
			var last: Node = parts.get_child(parts.get_child_count() - 1) if parts != null and parts.get_child_count() > 0 else null
			if not last is TextureRect or last.texture == null:
				ready = false
		if ready:
			return
		await process_frame

func check_region(owner: Node, name: String, additive: bool, modes: Array) -> void:
	var region := owner.find_child(name, true, false) as Control
	if region == null or not region.is_visible_in_tree() or region.modulate.a <= 0.0:
		failures.append("missing visible region " + name)
		return
	var parts := region.get_node_or_null("Parts")
	if parts == null or parts.get_child_count() == 0:
		failures.append("missing projected art " + name)
		return
	var part := parts.get_child(parts.get_child_count() - 1) as CanvasItem
	if not part is TextureRect or part.texture == null:
		failures.append("unloaded projected artwork " + name)
		return
	var material := part.material
	if material is ShaderMaterial:
		# Only these masked cast FX use model/projection mode proof plus raster inspection.
		if name not in ["CastingBarFlash", "CastingBarEnergyGlow"]:
			failures.append(name + " unexpected shader; material mode unverified")
		modes.append({"region": name, "material": "masked shader; inspect capture", "expected_add": additive})
		return
	var actual := material is CanvasItemMaterial and material.blend_mode == CanvasItemMaterial.BLEND_MODE_ADD
	modes.append({"region": name, "additive": actual})
	if actual != additive:
		failures.append(name + " additive=" + str(actual) + " expected=" + str(additive))
