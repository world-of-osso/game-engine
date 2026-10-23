extends SceneTree
# Actual GameClient unhandled_key_input, without a server or scene-tree lifecycle.
# GameMenuFrame Framework priority must run before CloseAllWindows AddOnPost.
func _initialize() -> void:
	call_deferred("run_probe")

func run_probe() -> void:
	var completion = preload("res://tests/completion.gd").new(self, 2)
	var client = ClassDB.instantiate("GameClient")
	if not completion.require(client != null, "real GameClient allocation"): return
	var state: Dictionary = client.probe_game_menu_escape_precedence()
	print("UI_TEMPLATE_ESCAPE_PRECEDENCE ", JSON.stringify(state))
	client.free()
	if not completion.require(not state.has("error"), str(state)): return
	if not completion.require(state.menu_before and state.pending_before, "menu and pending domain both eligible"): return
	if not completion.require(not state.menu_after_first and state.pending_after_first, "Framework menu Escape consumed pending panel cleanup"): return
	if not completion.case_done(): return
	if not completion.require(not state.menu_after_second and not state.pending_after_second, "next Escape must cancel pending panel without opening menu"): return
	if not completion.case_done(): return
	completion.finish(true)
