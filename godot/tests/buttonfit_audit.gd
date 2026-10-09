extends SceneTree
# Opt-in native audit; invoked by the ignored launcher test, never normal capture runs.
var overflows: Array[String] = []
var constrained: Array[String] = []
var failures: Array[String] = []
var labels := 0
var screens := 0

func _initialize() -> void:
	call_deferred("run_audit")

func run_audit() -> void:
	if OS.get_environment("GODOT_BUTTONFIT_AUDIT") != "1":
		print("IGNORED: set GODOT_BUTTONFIT_AUDIT=1")
		quit(0)
		return
	root.size = Vector2i(1920, 1080)
	OS.set_environment("GODOT_CASTBAR_PHASE", "midcast")
	OS.set_environment("GODOT_CASTBAR_TIME", "100")
	var inventory = ClassDB.instantiate("RegistryUi")
	var methods: Array[String] = []
	for entry in inventory.get_method_list():
		var method := String(entry.name)
		if method.begins_with("show_") and entry.args.is_empty() and (method.ends_with("preview") or method in ["show_portrait_party", "show_forever_portrait_party", "show_login", "show_loading", "show_character_select", "show_character_create"]):
			methods.append(method)
	inventory.free()
	methods.sort()
	for forever in [false, true]:
		OS.set_environment("GODOT_CASTBAR_SKIN", "forever" if forever else "modern")
		for method in methods:
			var views := ["browse"]
			if method.ends_with("auction_preview"):
				views = ["browse", "item", "dialog", "inventory", "sell", "duration", "owned", "bids"]
			elif method == "show_auction_confirmation_preview":
				views = ["bid-popup", "buyout-popup"]
			for view in views:
				OS.set_environment("GODOT_AUCTION_VIEW", view)
				await audit_preview(method, view, forever)
		for service in ["merchant", "buyback", "inbox", "send", "openmail", "opening", "trade", "guildbank"]:
			await audit_preview("show_buttonfit_service_preview", service, forever)
		for page in ["log", "detail", "progress", "reward"]:
			await audit_quest(page, forever)
	overflows.sort()
	constrained.sort()
	for line in overflows:
		print("OVERFLOW ", line)
	for line in constrained:
		print("CONSTRAINED ", line)
	for line in failures:
		print("ERROR ", line)
	print("BUTTONFIT screens=", screens, " labels=", labels, " overflows=", overflows.size(), " constrained=", constrained.size(), " errors=", failures.size())
	quit(0 if overflows.is_empty() and failures.is_empty() else 1)

func audit_preview(method: String, view: String, forever: bool) -> void:
	var ui = ClassDB.instantiate("RegistryUi")
	root.add_child(ui)
	var error = ui.call(method, view, forever) if method == "show_buttonfit_service_preview" else ui.call(method)
	var case := "%s:%s:%s" % ["Forever" if forever else "Modern", method, view]
	if error == "":
		error = ui.call("buttonfit_reskin", forever)
	await settle_and_audit(ui, case, String(error))
	ui.queue_free()
	await process_frame

func audit_quest(page: String, forever: bool) -> void:
	var probe = ClassDB.instantiate("UiAuditProbe")
	root.add_child(probe)
	var error = probe.call("mount_quest_overflow", forever, page, false)
	var case := "%s:quest:%s" % ["Forever" if forever else "Modern", page]
	await settle_and_audit(probe, case, String(error))
	probe.queue_free()
	await process_frame

func settle_and_audit(ui: Node, case: String, error: String) -> void:
	if not error.is_empty():
		failures.append(case + ": " + error)
		return
	for frame in range(5):
		await process_frame
	audit_tree(ui, case)
	screens += 1
	print("COVERED ", case)

func audit_tree(node: Node, screen: String) -> void:
	if node is Label and node.is_visible_in_tree() and not node.text.is_empty():
		audit_label(node, screen)
	for child in node.get_children():
		audit_tree(child, screen)

func audit_label(label: Label, screen: String) -> void:
	labels += 1
	var font := label.get_theme_font("font")
	var font_size := label.get_theme_font_size("font_size")
	var measured := font.get_string_size(label.text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x
	var owner = label.get_parent().get_parent()
	var button_text: bool = owner is Button and label.name == "Text"
	# Fixed UIPanelButton Text is centred without inset. CreateAll explicitly calls FitToText (+40).
	var padding := 40.0 if button_text and owner.name == "ProfessionsCreateAll" else 0.0
	var width: float = owner.size.x - padding if button_text else label.size.x
	if measured <= width + 0.01:
		return
	var line := "%s %s text=%s measured=%.2f available=%.2f frame=%.2f font=%d" % [screen, str(label.get_path()), label.text.replace("\n", "\\n"), measured, width, owner.size.x if button_text else label.size.x, font_size]
	if not button_text and (label.autowrap_mode != TextServer.AUTOWRAP_OFF or label.text_overrun_behavior != TextServer.OVERRUN_NO_TRIMMING):
		constrained.append(line)
	else:
		overflows.append(line)
