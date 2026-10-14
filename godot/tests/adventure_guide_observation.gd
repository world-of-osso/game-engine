extends SceneTree
# Read-only process boundary: received authority and displayed view must be distinct observations.
func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	var client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var state: Dictionary = client.adventure_guide_state()
	for field in ["renown_received_faction_count","journal_spell_catalog_ready","journal_item_catalog_ready"]:
		if not state.has(field):
			push_error("ADVENTURE_OBSERVATION_FAIL missing read-only field " + field)
			quit(1)
			return
	if state.renown_received_faction_count != -1 or state.renown_faction_count != -1:
		push_error("ADVENTURE_OBSERVATION_FAIL offline client fabricated a received/displayed snapshot")
		quit(1)
		return
	print("ADVENTURE_OBSERVATION_PASS distinct received/displayed/prerequisite state")
	quit(0)
