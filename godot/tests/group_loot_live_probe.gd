extends SceneTree

# Observation-only companion to the production startup JS on a private realm.
var client: Node
var last_state := ""

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	root.size = Vector2i(1280, 720)
	var scene: PackedScene = load("res://scenes/client.tscn")
	client = scene.instantiate()
	root.add_child(client)
	while is_instance_valid(client):
		await create_timer(1.0).timeout
		var state: Dictionary = client.account_state()
		var observed := {
			"screen": state.screen,
			"skin": state.ui_skin,
			"layout": state.ui_layout,
			"character_id": state.selected_character_id,
			"character_name": state.selected_character_name,
			"health": state.local_player_health,
		}
		var encoded := JSON.stringify(observed)
		if encoded != last_state:
			print("GROUP_LOOT_OBSERVED ", encoded)
			last_state = encoded
			var path := OS.get_environment("GROUP_LOOT_PROBE")
			if not path.is_empty():
				var file := FileAccess.open(path, FileAccess.WRITE)
				if file == null:
					push_error("Cannot write group-loot observation: " + path)
					quit(1)
					return
				file.store_string(encoded)
				file.close()
