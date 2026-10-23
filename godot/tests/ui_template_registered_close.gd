extends SceneTree
# Real GameClient domain adapters, no mounted registry or external server.
# Specifically guards pending-request cancellation before any Perks UI can exist.
var completion

func _initialize() -> void:
	call_deferred("run_cases")

func run_cases() -> void:
	completion = preload("res://tests/completion.gd").new(self, 2)
	var client: Node = ClassDB.instantiate("GameClient")
	if not completion.require(client != null, "real GameClient class missing"): return
	if not completion.require(client.has_method("probe_registered_window_closes"), "debug close probe missing"):
		client.free()
		return
	var state: Dictionary = client.call("probe_registered_window_closes")
	print("UI_TEMPLATE_REGISTERED_CLOSE ", JSON.stringify(state))
	client.free()
	if not completion.require(not state.has("error"), "registered domain adapter failed: " + str(state)): return
	if not completion.require(state.empty_handled == false, "empty sweep must remain unhandled"): return
	if not completion.case_done(): return
	if not completion.require(state.pending_before == true and state.ui_mounted == false, "fixture must have pending request without UI"): return
	if not completion.require(state.pending_handled == true and state.pending_after == false and state.repeat_handled == false, "pending close/repeat semantics changed"): return
	if not completion.case_done(): return
	completion.finish(true)
