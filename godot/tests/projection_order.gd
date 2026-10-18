extends SceneTree

var completion = preload("res://tests/completion.gd").new(self, 4)

func _initialize() -> void:
	call_deferred("run_completion")

func run_completion() -> void:
	completion.finish(await check_order())

func check_order() -> bool:
	root.size = Vector2i(1920, 1080)
	var ui: Node = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var error: String = ui.call("show_projection_order_preview")
	if not completion.require(error == "", error):
		return false
	for frame in range(4):
		await process_frame
	var failures := []
	for pair in [["OrderTooltip", "OrderDialog"], ["OrderHigh", "OrderMedium"], ["OrderRaised", "OrderUnraised"], ["OrderLater", "OrderEarlier"]]:
		var top := ui.find_child(pair[0], true, false) as Control
		var bottom := ui.find_child(pair[1], true, false) as Control
		if not completion.require(top != null and bottom != null, "actual projected pair"):
			return false
		var point := top.get_global_rect().get_center()
		var hit: String = ui.call("projection_order_preview_hit", float(point.x), float(point.y))
		var rendered_above := top.z_index > bottom.z_index or (top.z_index == bottom.z_index and top.get_parent() == bottom.get_parent() and top.get_index() > bottom.get_index())
		print("PROJECTION_ORDER_PAIR ", JSON.stringify({"top": pair[0], "bottom": pair[1], "top_z": top.z_index, "bottom_z": bottom.z_index, "pointer": hit, "rendered_above": rendered_above}))
		if hit != pair[0] or not rendered_above:
			failures.append(str(pair[0]) + " render/pointer order mismatch")
		elif not completion.case_done():
			return false
	# Reordering reused siblings must not change the registry's existing ID tiebreak.
	if not completion.require(ui.call("projection_order_preview_reverse_siblings") == ""):
		return false
	var later := ui.find_child("OrderLater", true, false) as Control
	var earlier := ui.find_child("OrderEarlier", true, false) as Control
	var reused_point := later.get_global_rect().get_center()
	var reused_hit: String = ui.call("projection_order_preview_hit", float(reused_point.x), float(reused_point.y))
	print("PROJECTION_ORDER_REUSED_IDS later_index=", later.get_index(), " earlier_index=", earlier.get_index(), " pointer=", reused_hit)
	if reused_hit != "OrderLater" or later.get_index() <= earlier.get_index():
		failures.append("reused sibling ID render/pointer order mismatch")
	if not completion.require(failures.is_empty(), "; ".join(failures)):
		return false
	ui.free()
	return true
