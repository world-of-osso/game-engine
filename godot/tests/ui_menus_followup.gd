extends SceneTree
# Source: SettingsPanel.xml; GameMenuFrame.lua139-216; MainMenuFrameTemplates.xml.
# Production screen builders, actual native mouse actions and source-relative geometry.
func _initialize() -> void:
	call_deferred("run_cases")

func run_cases() -> void:
	root.content_scale_size = Vector2i.ZERO
	var output := OS.get_environment("UI_MENUS_CAPTURE_PATH")
	if output.is_empty():
		push_error("Set UI_MENUS_CAPTURE_PATH")
		quit(1)
		return
	for forever in [true]:
		for case in ["menu", "settings", "addons", "search", "pending"]:
			if not await capture_case(output, case, forever):
				quit(1)
				return
	print("PASS ui-menus-followup both skins: XML geometry, native clicks, ten1920x1080 captures")
	quit(0)

func capture_case(output: String, case: String, forever: bool) -> bool:
	var ui = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var error: String
	if case == "menu":
		error = ui.call("show_buttonfit_service_preview", "menu", forever)
	elif case == "pending":
		error = ui.call("show_buttonfit_settings_pending_preview",forever)
	else:
		error = ui.call("show_buttonfit_settings_preview", "music" if case == "search" else "", case == "addons", forever)
	if not error.is_empty():
		push_error(error)
		return false
	for tick in range(120):
		await process_frame
		await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	if image.get_size() != Vector2i(1920,1080) or DisplayServer.window_get_size() != Vector2i(1920,1080):
		push_error("Compositor, window and framebuffer must be1920x1080")
		return false
	if not check_geometry(ui, case):
		return false
	if case == "settings" and not await hidden_apply_has_no_action(ui):
		return false
	var skin := "forever" if forever else "modern"
	if image.save_png(output.path_join("%s-%s.png" % [case,skin])) != OK:
		push_error("Cannot save capture")
		return false
	if case == "menu":
		if not await click_action(ui,"MenuBtnEditMode","menu_edit_mode") or not await click_action(ui,"MenuBtnMacros","menu_macros"):
			return false
	elif case == "settings":
		if not await click_action(ui,"OptionsAddOnsTabControl","options_tab:addons"):
			return false
	elif case == "addons":
		if not await click_action(ui,"OptionsGameTabControl","options_tab:game"):
			return false
	elif case == "pending":
		if not await click_action(ui,"OptionsApplyButton","options_apply"):
			return false
	elif not await click_action(ui,"OptionsSearchClear","options_search_clear"):
		return false
	if case != "menu" and not await click_action(ui,"OptionsCloseButton","options_close"):
		return false
	print("PASS capture ",case," ",skin)
	ui.queue_free()
	await process_frame
	return true

func check_geometry(ui: Node, case: String) -> bool:
	if case == "menu":
		var menu := ui.find_child("GameMenuPanel",true,false) as Control
		if menu == null or menu.size != Vector2(260,430):
			push_error("MainMenu source padding/section/button extents mismatch")
			return false
		var prior_y := -1.0
		for name in ["MenuBtnOptions","MenuBtnAddons","MenuBtnEditMode","MenuBtnSupport","MenuBtnMacros","MenuBtnLogout","MenuBtnExit","MenuBtnResume"]:
			var button := ui.find_child(name,true,false) as Control
			if button == null or button.size != Vector2(200,36) or button.position.y <= prior_y:
				push_error("GameMenu source order/36px button mismatch: "+name)
				return false
			prior_y = button.position.y
		return true
	var panel := ui.find_child("OptionsRoot",true,false) as Control
	var game := ui.find_child("OptionsGameTab",true,false) as Control
	var addons := ui.find_child("OptionsAddOnsTab",true,false) as Control
	var search := ui.find_child("OptionsSearchBox",true,false) as Control
	if panel == null or game == null or addons == null or search == null:
		push_error("Settings header missing")
		return false
	if panel.size != Vector2(920,724) or game.position != Vector2(32,27) or game.size.y != 37:
		push_error("SettingsPanel/GameTab XML mismatch")
		return false
	if addons.position != Vector2(32+game.size.x+5,27) or addons.size.y != 37:
		push_error("AddOnsTab relative TOPRIGHT+5 mismatch")
		return false
	if search.position != Vector2(552,34) or search.size != Vector2(350,22):
		push_error("SearchBox XML Container TOPRIGHT +(4,20) mismatch: "+str(search.get_rect()))
		return false
	var close := ui.find_child("OptionsCloseButton",true,false) as Control
	var apply := ui.find_child("OptionsApplyButton",true,false) as Control
	var defaults := ui.find_child("OptionsDefaultsButton",true,false) as Control
	var content := ui.find_child("OptionsContentPanel",true,false) as Control
	if close == null or apply == null or defaults == null or content == null:
		push_error("Retail footer/header controls missing")
		return false
	var close_offset := close.get_global_rect().position-panel.get_global_rect().position
	if close.size != Vector2(96,22) or close_offset != Vector2(808,686):
		push_error("CloseButton XML BOTTOMRIGHT(-16,16) mismatch: "+str(close_offset))
		return false
	if apply.is_visible_in_tree() != (case == "pending"):
		push_error("Apply must be shown only for actual uncommitted draft values")
		return false
	if case == "pending":
		var apply_offset := apply.get_global_rect().position-panel.get_global_rect().position
		if apply.size != Vector2(96,22) or apply_offset != Vector2(710,686):
			push_error("ApplyButton XML RIGHT->Close LEFT-2 mismatch")
			return false
	if defaults.is_visible_in_tree() != (case != "search"):
		push_error("Defaults header must hide during search")
		return false
	if defaults.is_visible_in_tree():
		var defaults_offset := defaults.get_global_rect().position-content.get_global_rect().position
		if defaults.size != Vector2(96,22) or defaults_offset != Vector2(533,16):
			push_error("DefaultsButton XML Header TOPRIGHT(-36,-16) mismatch")
			return false
	print("GEOMETRY XML ",game.get_rect()," ",addons.get_rect()," ",search.get_rect()," Close=",close_offset," Apply=",apply.is_visible_in_tree())
	return true

func hidden_apply_has_no_action(ui: Node) -> bool:
	var panel := ui.find_child("OptionsRoot",true,false) as Control
	# Hit the authored Apply rectangle, not the registry's unconditional action lookup.
	var point := panel.get_global_rect().position+Vector2(758,697)
	for down in [true,false]:
		var event := InputEventMouseButton.new()
		event.position = point
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = down
		root.push_input(event,true)
		await process_frame
	ui.call("sync_input")
	var action: String = ui.call("pop_action")
	if not action.is_empty():
		push_error("Hidden Apply rectangle emitted "+action)
		return false
	print("PASS hidden Apply has no native mouse action")
	return true

func click_action(ui: Node, name: String, expected: String) -> bool:
	var button := ui.find_child(name,true,false) as Control
	if button == null:
		push_error("Missing click target "+name)
		return false
	for down in [true,false]:
		var event := InputEventMouseButton.new()
		event.position = button.get_global_rect().get_center()
		event.button_index = MOUSE_BUTTON_LEFT
		event.pressed = down
		root.push_input(event,true)
		await process_frame
	ui.call("sync_input")
	var observed: String = ui.call("pop_action")
	if observed != expected:
		push_error("Native click "+name+" expected "+expected+" observed "+observed)
		return false
	print("PASS native click ",name," ",observed)
	return true
