extends SceneTree

## Login while local CASC initializes. Environment:
##   GODOT_TEST_SERVER    server address (a private test server)
##   STARTUP_ACCOUNT      account with a character (password fbtest)
## The session must handle the login reply while the CASC startup worker still runs:
## with a cold resolver cache (ASSET_RESOLVER_CACHE_DIR pointing at an empty directory)
## the reply must arrive before startup finishes; the character select screen shows
## once startup has finished.
## Prints the seconds from start to the reply, to the end of CASC startup and to
## character select.

var client: Node

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("STARTUP_ACCOUNT")
	if server == "" or account == "":
		fail("GODOT_TEST_SERVER and STARTUP_ACCOUNT are required")
		return
	var started := Time.get_ticks_msec()
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, "fbtest", false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	var reply_s := -1.0
	var ready_s := -1.0
	var reply_while_starting := false
	var deadline := started + 300000
	while Time.get_ticks_msec() < deadline:
		await process_frame
		var state: Dictionary = client.account_state()
		var now := (Time.get_ticks_msec() - started) / 1000.0
		if reply_s < 0.0 and state.reply_received:
			reply_s = now
			reply_while_starting = state.assets_starting
		if ready_s < 0.0 and not state.assets_starting:
			ready_s = now
		if state.screen == "CharacterSelect" and state.character_count >= 1 and ready_s >= 0.0 \
				and client.get_node_or_null("CharacterSelectUI") != null:
			print("FIXTURE STARTUP_LOGIN reply_s=%.1f casc_ready_s=%.1f charselect_s=%.1f reply_while_starting=%s" % [reply_s, ready_s, now, reply_while_starting])
			if OS.get_environment("STARTUP_EXPECT_COLD") == "1" and not reply_while_starting:
				fail("The login reply waited for CASC startup")
				return
			client.free()
			quit(0)
			return
	fail("No character select within 300 s: " + str(client.account_state()))

func fail(message: String) -> void:
	push_error(message)
	print("FIXTURE FAIL ", message)
	if client != null and is_instance_valid(client):
		client.free()
	quit(1)
