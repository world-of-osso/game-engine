extends RefCounted

# One fixture-owned observer; records failures without stopping before both views.
var failures: Array[String] = []
var options_recorded := false
var options_metrics: Dictionary = {}
var money_metrics: Dictionary = {}

func record_options(menu: Node) -> void:
	options_recorded = true
	var panel := menu.find_child("OptionsContentPanel", true, false) as Control
	var options := menu.find_child("OptionsRoot", true, false) as Control
	var last := menu.find_child("ToggleRowauto_loot", true, false) as Control
	if panel == null or options == null or last == null:
		failures.append("Options content/root/last Auto Loot row missing")
		print("UI_OVERFLOW_OPTIONS missing panel=%s root=%s last=%s" % [panel, options, last])
		return
	var panel_rect := panel.get_global_rect()
	var root_rect := options.get_global_rect()
	options_metrics = {"content": panel_rect, "root": root_rect, "last_auto_loot": last.get_global_rect(), "rows": []}
	# HUD controls must remain available, not disappear to satisfy enclosure.
	for row_name in ["ToggleRowshow_minimap", "ToggleRowshow_action_bars", "ToggleRowshow_nameplates", "SliderRownameplate_distance", "ThicknessRownameplate_health_thickness", "ThicknessRownameplate_spellbar_thickness", "ToggleRowshow_health_bars", "ToggleRowshow_target_marker", "ToggleRowauto_loot"]:
		var required := menu.find_child(row_name, true, false) as Control
		if required == null or not required.is_visible_in_tree():
			failures.append("Options required HUD row missing/hidden: " + row_name)
	# Registry projection need not parent rows beneath their authored panel.
	for node in menu.find_children("*", "Control", true, false):
		var row := node as Control
		var row_name := str(row.name)
		if not (row_name.begins_with("ToggleRow") or row_name.begins_with("SliderRow") or row_name.begins_with("ThicknessRow") or row_name.begins_with("InfoRow") or row_name.begins_with("GhostRow") or row_name.begins_with("ActionRow")):
			continue
		if not row.is_visible_in_tree():
			continue
		var rect := row.get_global_rect()
		options_metrics.rows.append({"name": row_name, "rect": rect, "size": row.size, "minimum": row.get_combined_minimum_size()})
		if not panel_rect.encloses(rect) or not root_rect.encloses(rect):
			failures.append("Options visible row %s outside content/root: row=%s content=%s root=%s" % [row_name, rect, panel_rect, root_rect])
	if not last.is_visible_in_tree():
		failures.append("Options required last Auto Loot row hidden")
	if options_metrics.rows.is_empty():
		failures.append("Options has no visible list rows")
	print("UI_OVERFLOW_OPTIONS %s" % options_metrics)

func record_money(host: Node) -> void:
	var label := host.find_child("LootFrameElement2Text", true, false) as Label
	var card := host.find_child("LootFrameElement2", true, false) as Control
	if label == null or card == null:
		failures.append("First manual LootFrame money label/card missing")
		print("UI_OVERFLOW_MONEY missing label=%s card=%s" % [label, card])
		return
	var font := label.get_theme_font("font")
	var font_size := label.get_theme_font_size("font_size")
	var font_height := font.get_height(font_size)
	var line_count := label.get_line_count()
	var line_spacing := label.get_theme_constant("line_spacing")
	var minimum := label.get_combined_minimum_size()
	var line_heights: Array[float] = []
	var text_height := 0.0
	for line in range(line_count):
		var height := float(label.get_line_height(line))
		line_heights.append(height)
		text_height += maxf(height, font_height)
	text_height += maxi(0, line_count - 1) * line_spacing
	var shadow := Vector2.ZERO
	if label.get_theme_color("font_shadow_color").a > 0.0:
		shadow = Vector2(label.get_theme_constant("shadow_offset_x"), label.get_theme_constant("shadow_offset_y"))
	var rect := label.get_global_rect()
	var card_rect := card.get_global_rect()
	var scale := rect.size / label.size
	# LF.xml money label: x=5+37+8, y=4+37/2-19, 93x38; card 46 high.
	var authored := Rect2(card_rect.position + Vector2(50.0, 3.5) * scale, Vector2(93.0, 38.0) * scale)
	var needed := Vector2(minimum.x, maxf(minimum.y, text_height))
	var text_top := 0.0
	if label.vertical_alignment == VERTICAL_ALIGNMENT_CENTER:
		text_top = maxf(0.0, (label.size.y - needed.y) / 2.0)
	elif label.vertical_alignment == VERTICAL_ALIGNMENT_BOTTOM:
		text_top = maxf(0.0, label.size.y - needed.y)
	var paint := Rect2(rect.position + Vector2(minf(0.0, shadow.x), text_top + minf(0.0, shadow.y)) * scale, (needed + shadow.abs()) * scale)
	money_metrics = {"rect": rect, "size": label.size, "combined_minimum_size": minimum, "line_count": line_count, "visible_line_count": label.get_visible_line_count(), "line_height": label.get_line_height(), "line_heights": line_heights, "font_height": font_height, "theme_font_size": font_size, "line_spacing": line_spacing, "text_height": text_height, "shadow": shadow, "paint": paint, "card": card_rect, "card_size": card.size, "authored_label": authored, "scale": scale, "clip_text": label.clip_text, "vertical_alignment": label.vertical_alignment}
	print("UI_OVERFLOW_MONEY %s" % money_metrics)
	if not options_recorded:
		failures.append("Options metrics not recorded before manual money")
	if not label.is_visible_in_tree() or line_count != 3 or label.get_visible_line_count() != 3 or label.text != "1 Gold\n5 Silver\n2 Copper":
		failures.append("Money must display all three authored coin lines")
	if needed.x + absf(shadow.x) > 93.0 or needed.y + absf(shadow.y) > 38.0:
		failures.append("Money native paint/minimum exceeds authored 93x38 label: needed=%s shadow=%s" % [needed, shadow])
	if not authored.encloses(rect) or not card_rect.encloses(rect) or not authored.encloses(paint) or not card_rect.encloses(paint) or paint.size.y > 46.0 * scale.y:
		failures.append("Money native label/paint outside authored label or 46px card: rect=%s paint=%s authored=%s card=%s" % [rect, paint, authored, card_rect])
