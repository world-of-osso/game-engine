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
	var login_root = login.find_child("LoginRoot", true, false)
	if login_root.modulate.a >= 1.0:
		push_error("Login must start its original fade-in: alpha=" + str(login_root.modulate.a))
		quit(1)
		return
	var faded = Time.get_ticks_msec() + 1000
	while Time.get_ticks_msec() < faded:
		await process_frame
	if not is_equal_approx(login_root.modulate.a, 1.0):
		push_error("Login fade-in must reach full opacity after 0.75s: alpha=" + str(login_root.modulate.a))
		quit(1)
		return
	client.queue_free()
	print("PASS: native client starts with authored login credential controls and fades in")
	quit(0)
