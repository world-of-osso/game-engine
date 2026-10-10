extends SceneTree

func _initialize() -> void:
	var client := GameClient.new()
	var snapshot := client.tooltip_state()
	var appearances = snapshot.get("appearance_collection")
	if appearances == null:
		push_error("Tooltip snapshot must expose the authoritative appearance collection")
		client.free()
		quit(1)
		return
	if appearances.size() != 0:
		push_error("A new client must have an empty appearance collection")
		client.free()
		quit(1)
		return
	appearances.append(214)
	if client.tooltip_state().appearance_collection.size() != 0:
		push_error("Mutating the returned snapshot must not change the client collection")
		client.free()
		quit(1)
		return
	client.free()
	print("PASS appearance collection snapshot is authoritative and read-only")
	quit(0)
