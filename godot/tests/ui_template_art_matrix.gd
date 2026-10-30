extends SceneTree
# Named-art production consumers; supplied states, no artwork or geometry overrides.
var completion

func _initialize() -> void:
	call_deferred("run_cases")

func cases(forever: bool) -> Array:
	var prefix := "show_forever_" if forever else "show_"
	return [
		["Trainer",prefix+"trainer_preview",[]],
		["Merchant","show_buttonfit_service_preview",["merchant-repair",forever]],
		["AuctionHouse",prefix+"auction_preview",[],"browse"],
		["AuctionDialog",prefix+"auction_preview",[],"dialog"],
		["Professions",prefix+"professions_preview",[]],
		["Spellbook",prefix+"spellbook_preview",[]],
		["TalentDialog","show_gameplay_panels_preview",[forever,"talent-dialog"]],
		["Transmog","show_template_blend_preview",[forever,"Transmog"]],
		["CharacterCurrency","show_template_blend_preview",[forever,"CharacterCurrency"]],
		["AdventureGuide","show_template_blend_preview",[forever,"AdventureGuide"]],
		["Achievements",prefix+"achievement_preview",[]],
		["AchievementSearch","show_achievement_full_results_preview",[forever,true]],
		["TradingPost","show_template_blend_preview",[forever,"TradingPost"]],
		["HousingStorage","show_template_blend_preview",[forever,"HousingStorageBasic"]],
		["HousingSettings","show_house_access_preview",[forever,false]],
		["ReadyCheck","show_ready_check_preview",[forever]],
		["GameMenu","show_buttonfit_service_preview",["menu",forever]],
		["GuildControl",prefix+"guild_ranks_preview",[]],
		["GuildMember","show_template_guild_art",[forever,"GuildMember"]],
		["GuildLog","show_template_guild_art",[forever,"GuildLog"]],
		["GuildNews","show_template_guild_art",[forever,"GuildNews"]],
		["Campsite","show_glue_menus_preview",["campsite",forever]],
		["DeleteCharacter","show_glue_menus_preview",["delete",forever]],
		["Chat","show_template_art_preview",[forever,"Chat"]],
		["DamageMeter","show_template_font_preview",[forever,"DamageMeter"]],
		["ToyBox","show_template_art_preview",[forever,"ToyBox"]],
		["Loading","show_template_art_preview",[forever,"Loading"]],
		["Bank",prefix+"bank_preview",[]],
		["CloseHover","show_buttonfit_service_preview",["merchant-repair",forever]],
		["ClosePushed","show_buttonfit_service_preview",["merchant-repair",forever]],
	]

func run_cases() -> void:
	root.size = Vector2i(1920,1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self,60)
	var output := OS.get_environment("UI_TEMPLATE_ART_CAPTURES")
	if not completion.require(not output.is_empty(),"UI_TEMPLATE_ART_CAPTURES required"):
		return
	if not completion.require(DirAccess.make_dir_recursive_absolute(output) == OK,"capture directory"):
		return
	for forever in [true]:
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
	OS.set_environment("GODOT_AUCTION_VIEW",panel[3] if panel.size() > 3 else "browse")
	var owner: Node = ClassDB.instantiate("RegistryUi")
	root.add_child(owner)
	var error: String = owner.callv(panel[1],panel[2])
	if not error.is_empty():
		owner.free()
		return panel[0]+": "+error
	await frames(60)
	if panel[0] in ["CloseHover","ClosePushed"]:
		error = await check_close(owner,panel[0] == "ClosePushed")
		if not error.is_empty():
			owner.free()
			return panel[0]+": "+error
	var textures := 0
	for part in owner.find_children("Part*","TextureRect",true,false):
		if part.is_visible_in_tree() and part.texture != null:
			textures += 1
	if textures == 0:
		owner.free()
		return panel[0]+": no loaded visible artwork"
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var skin := "forever" if forever else "modern"
	var path := output.path_join(skin+"-"+panel[0]+".png")
	if image == null or image.get_size() != Vector2i(1920,1080) or image.save_png(path) != OK:
		owner.free()
		return panel[0]+": capture failed or wrong dimensions"
	print("UI_TEMPLATE_ART_NATIVE ",JSON.stringify({"skin":skin,"panel":panel[0],"loaded_visible_parts":textures,"capture":path}))
	owner.free()
	await frames(2)
	return ""

func check_close(owner: Node, pushed: bool) -> String:
	var button: BaseButton
	for node in owner.find_children("*CloseButton","BaseButton",true,false):
		if node.is_visible_in_tree():
			button = node
			break
	if button == null:
		return "missing visible production close owner"
	var normal := button.get_node_or_null("Parts/Part0") as TextureRect
	if normal == null or normal.texture == null:
		return "missing close normal art"
	var normal_texture := normal.texture
	button.emit_signal("mouse_entered")
	if pushed:
		button.emit_signal("button_down")
	var error: String = owner.call("sync_input")
	if not error.is_empty():
		return error
	await frames(6)
	var base := button.get_node_or_null("Parts/Part0") as TextureRect
	var highlight := button.get_node_or_null("Parts/Part1") as TextureRect
	if base == null or base.texture == null or highlight == null or highlight.texture == null:
		return "close states did not project base and highlight"
	if base.position != Vector2.ZERO or base.size != button.size or highlight.size != button.size:
		return "close state geometry changed the input-owner extent"
	if not highlight.material is CanvasItemMaterial or highlight.material.blend_mode != CanvasItemMaterial.BLEND_MODE_ADD:
		return "close highlight lost authored ADD"
	if pushed and base.texture == normal_texture:
		return "close pushed state retained normal texture"
	return ""
