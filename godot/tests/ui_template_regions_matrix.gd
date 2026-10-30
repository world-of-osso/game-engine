extends SceneTree
# Production-panel state regions; no registry, texture or material overrides.

var completion

func _initialize() -> void:
	call_deferred("run_cases")

func cases(forever: bool) -> Array:
	return [
		{"name":"PerksHover", "method":"show_template_blend_preview", "args":[forever,"TradingPost"], "signals":[["PerksProgramRotateLeftButton","mouse_entered"]], "parts":[["PerksProgramRotateLeftButton",0,[0,0,48,48],1.0,"blend"],["PerksProgramRotateLeftButton",1,[0,0,48,48],0.6,"add"]]},
		{"name":"PerksPushed", "method":"show_template_blend_preview", "args":[forever,"TradingPost"], "signals":[["PerksProgramRotateLeftButton","mouse_entered"],["PerksProgramRotateLeftButton","button_down"]], "parts":[["PerksProgramRotateLeftButton",0,[1,1,48,48],1.0,"blend"],["PerksProgramRotateLeftButton",1,[0,0,48,48],0.6,"add"]]},
		{"name":"MinimapHover", "method":"show_minimap_zoom_preview", "args":[forever,3], "signals":[["MinimapZoomIn","mouse_entered"]], "parts":[["MinimapZoomIn",0,[0,0,17,17],1.0,"blend"],["MinimapZoomIn",1,[0,0,17,17],1.0,"blend"]]},
		{"name":"MinimapDisabledIn", "method":"show_minimap_zoom_preview", "args":[forever,5], "signals":[], "parts":[["MinimapZoomIn",0,[0,0,17,17],1.0,"desaturated"]]},
		{"name":"MinimapDisabledOut", "method":"show_minimap_zoom_preview", "args":[forever,0], "signals":[], "parts":[["MinimapZoomOut",0,[0,0,17,9],1.0,"desaturated"]]},
		{"name":"GuildBankChecked", "method":"show_template_blend_preview", "args":[forever,"GuildBank"], "signals":[], "parts":[["GuildBankTab1Button",0,[0,0,36,34],1.0,"add"]]},
		{"name":"HousingDrawerHover", "method":"show_template_blend_preview", "args":[forever,"HousingStorageCollapsed"], "signals":[["HouseEditorStorageExpandButton","mouse_entered"]], "parts":[["HouseEditorStorageExpandButton",0,[-4,0,64,72],0.5,"add"]]},
		{"name":"TransmogHover", "method":"show_template_blend_preview", "args":[forever,"Transmog"], "signals":[["TransmogSlot16","mouse_entered"]], "parts":[["TransmogSlot16",0,[],1.0,"add"]]},
	]

func run_cases() -> void:
	root.size = Vector2i(1920,1080)
	root.content_scale_size = Vector2i.ZERO
	completion = preload("res://tests/completion.gd").new(self,16)
	var output := OS.get_environment("UI_TEMPLATE_REGIONS_CAPTURES")
	if not completion.require(not output.is_empty(),"UI_TEMPLATE_REGIONS_CAPTURES required"):
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

func check_panel(panel: Dictionary, forever: bool, output: String) -> String:
	var owner: Node = ClassDB.instantiate("RegistryUi")
	root.add_child(owner)
	var error: String = owner.callv(panel.method,panel.args)
	if not error.is_empty():
		owner.free()
		return panel.name + ": " + error
	await frames(8)
	for signal_pair in panel.signals:
		var control := owner.find_child(signal_pair[0],true,false) as Control
		if control == null or not control.is_visible_in_tree():
			owner.free()
			return panel.name + ": missing visible input owner " + signal_pair[0]
		control.emit_signal(signal_pair[1])
		error = owner.call("sync_input")
		if not error.is_empty():
			owner.free()
			return panel.name + ": input " + error
		await frames(3)
	var evidence: Array = []
	for expected in panel.parts:
		error = await check_part(owner,expected,evidence)
		if not error.is_empty():
			owner.free()
			return panel.name + ": " + error
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var skin := "forever" if forever else "modern"
	var path := output.path_join(skin + "-" + panel.name + ".png")
	if image == null or image.get_size() != Vector2i(1920,1080) or image.save_png(path) != OK:
		owner.free()
		return panel.name + ": capture failed or wrong dimensions"
	print("UI_TEMPLATE_REGIONS_NATIVE ",JSON.stringify({"skin":skin,"panel":panel.name,"parts":evidence,"capture":path}))
	owner.free()
	await frames(2)
	return ""

func check_part(owner: Node, expected: Array, evidence: Array) -> String:
	var frame := owner.find_child(expected[0],true,false) as Control
	if frame == null or not frame.is_visible_in_tree():
		return "missing visible region " + expected[0]
	var part: TextureRect
	var deadline := Time.get_ticks_msec() + 30000
	while Time.get_ticks_msec() < deadline:
		part = frame.get_node_or_null("Parts/Part" + str(expected[1])) as TextureRect
		if part != null and part.texture != null:
			break
		await process_frame
	if part == null or part.texture == null:
		return "unloaded region " + expected[0]
	var rect := [part.position.x,part.position.y,part.size.x,part.size.y]
	if not expected[2].is_empty():
		for index in range(4):
			if not is_equal_approx(rect[index],float(expected[2][index])):
				return expected[0] + " rect " + str(rect) + " expected " + str(expected[2])
	if not is_equal_approx(part.self_modulate.a,float(expected[3])):
		return expected[0] + " alpha " + str(part.self_modulate.a)
	var material := "blend"
	if part.material is CanvasItemMaterial:
		if part.material.blend_mode == CanvasItemMaterial.BLEND_MODE_ADD:
			material = "add"
	elif expected[4] == "desaturated":
		await RenderingServer.frame_post_draw
		var grayscale := count_grayscale_pixels(part)
		if grayscale < 3:
			return expected[0] + " lacks sampled grayscale pixels: " + str(grayscale)
		material = "desaturated"
	if material != expected[4]:
		return expected[0] + " material " + material + " expected " + expected[4]
	evidence.append({"region":expected[0],"part":expected[1],"rect":rect,"alpha":part.self_modulate.a,"material":material})
	return ""

func count_grayscale_pixels(part: TextureRect) -> int:
	var image := root.get_texture().get_image()
	var area := part.get_global_rect()
	var count := 0
	for y in range(maxi(0,int(area.position.y)),mini(image.get_height(),int(area.end.y))):
		for x in range(maxi(0,int(area.position.x)),mini(image.get_width(),int(area.end.x))):
			var color := image.get_pixel(x,y)
			var low := minf(color.r,minf(color.g,color.b))
			var high := maxf(color.r,maxf(color.g,color.b))
			if low > 0.1 and high < 0.95 and high - low < 0.02:
				count += 1
	return count
