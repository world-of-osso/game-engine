extends RefCounted

# One fixture-owned observer; records failures without stopping before both views.
var failures: Array[String] = []
var options_recorded := false
var options_metrics: Dictionary = {}
var money_metrics: Dictionary = {}

# Nearest-grid position rounding has a half-pixel bound in logical space.
# This does not relax text height, font size, clipping or actual card enclosure.
const HALF_LAYOUT_PIXEL := 0.5

func record_options(menu: Node, category: String, section: String = "") -> void:
	options_recorded = true
	var panel := menu.find_child("OptionsContentPanel", true, false) as Control
	var options := menu.find_child("OptionsRoot", true, false) as Control
	var title := menu.find_child("OptionsSectionTitle", true, false) as Label
	var key := category + ("/" + section if not section.is_empty() else "")
	if panel == null or options == null:
		failures.append("Options content/root missing: " + key)
		print("UI_OVERFLOW_OPTIONS category=%s missing panel=%s root=%s" % [key, panel, options])
		return
	var panel_rect := panel.get_global_rect()
	var root_rect := options.get_global_rect()
	var metrics := {"category": category, "section": section, "title": title.text if title != null else "", "scale": options.get_global_transform().get_scale(), "content": panel_rect, "root": root_rect, "rows": [], "descendants": []}
	# HUD availability is required only while the actual HUD category is selected.
	if category == "hud":
		for row_name in ["ToggleRowshow_minimap", "ToggleRowshow_action_bars", "ToggleRowshow_nameplates", "SliderRownameplate_distance", "ThicknessRownameplate_health_thickness", "ThicknessRownameplate_spellbar_thickness", "ToggleRowshow_health_bars", "ToggleRowshow_target_marker", "ToggleRowauto_loot"]:
			var required := menu.find_child(row_name, true, false) as Control
			if required == null or not required.is_visible_in_tree():
				failures.append("Options required HUD row missing/hidden: " + row_name)
	# Include every content descendant, not just outer rows: labels, sliders,
	# nameplate cells and section buttons can overflow their containing list.
	var controls: Array = panel.find_children("*", "Control", true, false)
	# Also collect list rows and descendants if projection puts a row beside panel.
	for node in menu.find_children("*", "Control", true, false):
		var row := node as Control
		var row_name := str(row.name)
		if not is_options_list_row(row_name) or not row.is_visible_in_tree():
			continue
		metrics.rows.append({"name": row_name, "rect": row.get_global_rect(), "size": row.size, "minimum": row.get_combined_minimum_size()})
		if not controls.has(row):
			controls.append(row)
		for descendant in row.find_children("*", "Control", true, false):
			if not controls.has(descendant):
				controls.append(descendant)
	for node in controls:
		var control := node as Control
		if not control.is_visible_in_tree():
			continue
		var rect := control.get_global_rect()
		metrics.descendants.append({"name": str(control.name), "rect": rect, "size": control.size, "minimum": control.get_combined_minimum_size()})
		if not panel_rect.encloses(rect) or not root_rect.encloses(rect):
			failures.append("Options %s visible content %s outside content/root: rect=%s content=%s root=%s" % [key, control.name, rect, panel_rect, root_rect])
	if metrics.rows.is_empty() or metrics.descendants.is_empty():
		failures.append("Options %s has no visible list content" % key)
	options_metrics[key + "@" + str(metrics.scale)] = metrics
	print("UI_OVERFLOW_OPTIONS %s" % metrics)

func is_options_list_row(name: String) -> bool:
	if name in ["NameplateHealthSize", "NameplateCastSize", "NameplateFonts", "NameplateToggles", "NameplateReactionColors", "NameplateFriendlyCastColors", "NameplateChannelColors"]:
		return true
	for prefix in ["ToggleRow", "SliderRow", "ThicknessRow", "InfoRow", "GhostRow", "ActionRow", "KeybindingRow", "NameplateRow"]:
		if name.begins_with(prefix):
			return true
	return false

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
	# Width is native minimum geometry (currently possibly 1px), not a full
	# glyph-pixel oracle. The measured three-line height is the regression here.
	var needed := Vector2(minimum.x, maxf(minimum.y, text_height))
	var authored_position_enclosure := authored.grow_individual(HALF_LAYOUT_PIXEL * scale.x, HALF_LAYOUT_PIXEL * scale.y, HALF_LAYOUT_PIXEL * scale.x, HALF_LAYOUT_PIXEL * scale.y)
	var text_top := 0.0
	if label.vertical_alignment == VERTICAL_ALIGNMENT_CENTER:
		text_top = maxf(0.0, (label.size.y - needed.y) / 2.0)
	elif label.vertical_alignment == VERTICAL_ALIGNMENT_BOTTOM:
		text_top = maxf(0.0, label.size.y - needed.y)
	var paint := Rect2(rect.position + Vector2(minf(0.0, shadow.x), text_top + minf(0.0, shadow.y)) * scale, (needed + shadow.abs()) * scale)
	money_metrics = {"rect": rect, "size": label.size, "combined_minimum_size": minimum, "line_count": line_count, "visible_line_count": label.get_visible_line_count(), "line_height": label.get_line_height(), "line_heights": line_heights, "font_height": font_height, "theme_font_size": font_size, "line_spacing": line_spacing, "text_height": text_height, "shadow": shadow, "paint": paint, "card": card_rect, "card_size": card.size, "authored_label": authored, "authored_position_enclosure": authored_position_enclosure, "half_layout_pixel": HALF_LAYOUT_PIXEL, "scale": scale, "clip_text": label.clip_text, "vertical_alignment": label.vertical_alignment}
	print("UI_OVERFLOW_MONEY %s" % money_metrics)
	if not options_recorded:
		failures.append("Options metrics not recorded before manual money")
	if not label.is_visible_in_tree() or line_count != 3 or label.get_visible_line_count() != 3 or label.text != "1 Gold\n5 Silver\n2 Copper":
		failures.append("Money must display all three authored coin lines")
	if font_size != 12 or label.clip_text or card.size.y != 46.0:
		failures.append("Money must retain font size 12, unclipped text and 46px card: font=%s clip=%s card=%s" % [font_size, label.clip_text, card.size])
	if needed.x + absf(shadow.x) > 93.0 or needed.y + absf(shadow.y) > 38.0:
		failures.append("Money native paint/minimum exceeds authored 93x38 label: needed=%s shadow=%s" % [needed, shadow])
	if not authored_position_enclosure.encloses(rect) or not card_rect.encloses(rect) or not authored_position_enclosure.encloses(paint) or not card_rect.encloses(paint) or paint.size.y > 46.0 * scale.y:
		failures.append("Money native label/paint outside authored label or 46px card: rect=%s paint=%s authored=%s card=%s" % [rect, paint, authored, card_rect])
