extends SceneTree
# Native boundary: real642 title painted at18, while the missing-theme-key default16 fits.
var completion: RefCounted

func _initialize() -> void:
	completion = load("res://tests/completion.gd").new(self, 1)
	call_deferred("run")

func run() -> void:
	completion.finish(await check_font_size())

func check_font_size() -> bool:
	var file := FileAccess.open(ProjectSettings.globalize_path("res://../data/db2/active/AdventureJournal.csv"), FileAccess.READ)
	if not completion.require(file != null, "installed AdventureJournal CSV absent"):
		return false
	var headers := file.get_csv_line()
	var title := ""
	while not file.eof_reached():
		var row := file.get_csv_line()
		if row.size() == headers.size() and row[headers.find("ID")] == "642":
			title = row[headers.find("Name_lang")]
			break
	if not completion.require(not title.is_empty(), "source642 title missing"):
		return false
	var font := FontFile.new()
	if not completion.require(font.load_dynamic_font(ProjectSettings.globalize_path("res://../data/fonts/MORPHEUS.TTF")) == OK, "authored Morpheus absent"):
		return false
	var ui := RegistryUi.new()
	root.add_child(ui)
	var label := Label.new()
	label.text = title
	label.add_theme_font_override("font", font)
	label.add_theme_font_size_override("font_size", 18)
	label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS_FORCE
	var painted_width := font.get_string_size(title, HORIZONTAL_ALIGNMENT_LEFT, -1, 18).x
	var old_width := font.get_string_size(title, HORIZONTAL_ALIGNMENT_LEFT, -1, label.get_theme_font_size("font")).x
	var boundary := (painted_width + old_width) / 2.0
	label.custom_maximum_size = Vector2(boundary, 30)
	label.size = Vector2(boundary, 30)
	root.add_child(label)
	for frame in 3:
		await process_frame
	print("SUGGESTION_FONT_SIZE_BOUNDARY title=%s painted_size=%s old_size=%s painted_width=%s old_width=%s actual_width=%s" % [title, label.get_theme_font_size("font_size"), label.get_theme_font_size("font"), painted_width, old_width, label.size.x])
	if not completion.require(painted_width > label.size.x and old_width < label.size.x, "native18/16 truncation boundary not exercised"):
		return false
	if not completion.require(ui.journal_suggestion_font_truncated(label), "painted18 truncates but product measured wrong16 and reported fitting"):
		return false
	return completion.case_done()
