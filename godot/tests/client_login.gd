extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var scene: PackedScene = load("res://scenes/client.tscn")
	var client = scene.instantiate()
	root.add_child(client)
	await process_frame
	var login = client.get_node_or_null("LoginUI")
	if login == null or not login is CanvasLayer:
		push_error("Client startup must attach the authored login UI")
		quit(1)
		return
	if login.find_child("UsernameInput", true, false) == null or login.find_child("PasswordInput", true, false) == null:
		push_error("Client login must expose authored credential controls")
		quit(1)
		return
	client.queue_free()
	print("PASS: native client starts with authored login credential controls")
	quit(0)
