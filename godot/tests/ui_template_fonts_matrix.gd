extends SceneTree
# Production consumers, ordinary preview states; never mutate resolved font data.
var completion

func _initialize() -> void:
	call_deferred("run_cases")

func cases(forever: bool) -> Array:
	var prefix := "show_forever_" if forever else "show_"
	return [
		["Trainer",prefix+"trainer_preview",[]],
		["RaidInfo","show_gameplay_panels_preview",[forever,"raid"]],
		["GroupLoot","show_template_font_preview",[forever,"GroupLoot"]],
		["Professions",prefix+"professions_preview",[]],
		["AchievementSearch","show_achievement_full_results_preview",[forever,true]],
		["AdventureGuide","show_template_blend_preview",[forever,"AdventureGuide"]],
		["Achievements",prefix+"achievement_preview",[]],
		["Battleground","show_gameplay_panels_preview",[forever,"scoreboard"]],
		["Transmog","show_template_blend_preview",[forever,"Transmog"]],
		["Character","show_template_blend_preview",[forever,"CharacterCurrency"]],
		["Tooltip","show_template_font_preview",[forever,"Tooltip"]],
		["DamageMeter","show_template_font_preview",[forever,"DamageMeter"]],
		["StackSplit","show_stack_split_preview",[forever,false,true]],
		["Merchant","show_buttonfit_service_preview",["merchant-repair",forever]],
		["Spellbook",prefix+"spellbook_preview",[]],
		["WorldMap","show_template_blend_preview",[forever,"WorldMap"]],
		["Bank",prefix+"bank_preview",[]],
		["AuctionHouse",prefix+"auction_preview",[]],
		["MirrorTimer","show_mirror_timer_shadow_preview",[forever]],
	]

func run_cases() -> void:
	root.size = Vector2i(1920,1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self,38)
	var output := OS.get_environment("UI_TEMPLATE_FONTS_CAPTURES")
	if not completion.require(not output.is_empty(),"UI_TEMPLATE_FONTS_CAPTURES required"):
		return
	if not completion.require(DirAccess.make_dir_recursive_absolute(output) == OK,"capture directory"):
		return
	for forever in [false,true]:
		for panel in cases(forever):
			var error := await check_panel(panel,forever,output)
			if not error.is_empty():
				completion.fail(error)
				return
			if not completion.case_done():
				return
	completion.finish(true)

func frames(count: int) -> void:
	for index in range(count):
		await process_frame

func check_panel(panel: Array, forever: bool, output: String) -> String:
	var owner: Node = ClassDB.instantiate("RegistryUi")
	root.add_child(owner)
	var error: String = owner.callv(panel[1],panel[2])
	if not error.is_empty():
		owner.free()
		return panel[0]+": "+error
	await frames(60)
	var labels: Array = []
	for node in owner.find_children("*","Label",true,false):
		var label := node as Label
		if not label.is_visible_in_tree() or label.text.is_empty():
			continue
		var font := label.get_theme_font("font")
		if font == null:
			owner.free()
			return panel[0]+": missing font "+str(label.name)
		labels.append({"name":str(label.name),"text":label.text,"size":label.get_theme_font_size("font_size"),"shadow":label.get_theme_color("font_shadow_color").a,"offset":[label.get_theme_constant("shadow_offset_x"),label.get_theme_constant("shadow_offset_y")],"outline":label.get_theme_constant("outline_size")})
	if labels.is_empty():
		owner.free()
		return panel[0]+": no visible text"
	error = check_authored_fonts(panel[0],forever,labels)
	if not error.is_empty():
		owner.free()
		return error
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var skin := "forever" if forever else "modern"
	var path := output.path_join(skin+"-"+panel[0]+".png")
	if image == null or image.get_size() != Vector2i(1920,1080) or image.save_png(path) != OK:
		owner.free()
		return panel[0]+": capture failed or wrong dimensions"
	print("UI_TEMPLATE_FONTS_NATIVE ",JSON.stringify({"skin":skin,"panel":panel[0],"labels":labels,"capture":path}))
	owner.free()
	await frames(2)
	return ""

func check_authored_fonts(panel: String, forever: bool, labels: Array) -> String:
	var controls := 0
	for label in labels:
		var name: String = label.name
		if panel == "Spellbook" and ((name.begins_with("SpellBookItem") and (name.ends_with("Name") or name.ends_with("RequiredLevel"))) or (name.begins_with("SpellBookHeader") and name.ends_with("Text"))):
			if label.shadow != 0.0:
				return name+": SystemFont text acquired a shadow"
			controls += 1
		if panel == "Tooltip" and name in ["TooltipTitle","TooltipLine0Left"]:
			if label.shadow != 0.0 or label.size != (14 if name == "TooltipTitle" else 12):
				return name+": tooltip inherited font mismatch"
			controls += 1
		if panel == "DamageMeter" and not forever and name in ["DamageMeterSessionTimer","DamageMeterSessionName","DamageMeterTypeName"]:
			if label.shadow != 1.0 or label.offset != [1,1] or label.size != 13:
				return name+": GameFontNormalMed1 mismatch"
			controls += 1
		if panel == "GroupLoot" and name == "GroupLootFrame1Name":
			if label.shadow != 1.0 or label.offset != [1,1] or label.size != 12:
				return name+": GameFontNormal mismatch"
			controls += 1
	var required := 3 if panel == "DamageMeter" and not forever else 2 if panel == "Tooltip" else 1 if panel in ["Spellbook","GroupLoot"] else 0
	if controls < required:
		return panel+": authored font controls "+str(controls)+" expected at least "+str(required)
	return ""
