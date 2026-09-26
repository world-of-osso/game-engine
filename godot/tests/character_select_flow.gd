extends SceneTree

func _initialize():
    call_deferred("run")

func run():
    root.size = Vector2i(1280, 720)
    var client = load("res://scenes/client.tscn").instantiate()
    root.add_child(client)
    var server = OS.get_environment("GODOT_TEST_SERVER")
    if server.is_empty():
        printerr("GODOT_TEST_SERVER must explicitly select a local fixture server")
        client.free()
        quit(1)
        return
    var error = client.connect_account(server, "admin", "admin", false)
    if error != "":
        printerr(error)
        client.free()
        quit(1)
        return
    var deadline = Time.get_ticks_msec() + 15000
    while Time.get_ticks_msec() < deadline:
        await process_frame
        var state = client.account_state()
        if state.reply_received:
            if state.screen != "CharacterSelect":
                printerr("Fixture authentication rejected: ", state.status)
                client.free()
                quit(1)
                return
            var ui = client.get_node_or_null("CharacterSelectUI")
            if ui == null or client.get_node("LoginUI").visible:
                printerr("Successful authentication did not replace login with character selection")
                client.free()
                quit(1)
                return
            print("PASS: real authentication projects authored character selection; roster=", state.character_count)
            client.free()
            quit(0)
            return
    printerr("Timed out waiting for fixture authentication")
    client.free()
    quit(1)
