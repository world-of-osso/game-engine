extends SceneTree

## Character select art arrives from the UI texture workers: frames drawn while their
## file loads show it once it arrives. Environment: GODOT_TEST_SERVER, CHARSELECT_ACCOUNT
## (password fbtest). Prints the empty TextureRect parts of the character select UI when
## it appears and once every requested file has arrived; fails when parts that had art
## still lack it, or when art never arrives.

const PASSWORD := "fbtest"
const ARRIVAL_WAIT_MS := 10000

var client: Node

func _initialize() -> void:
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("CHARSELECT_ACCOUNT")
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	var ui: Node = null
	var deadline := Time.get_ticks_msec() + 60000
	while ui == null and Time.get_ticks_msec() < deadline:
		await process_frame
		ui = client.get_node_or_null("CharacterSelectUI")
	if ui == null:
		fail("No character select UI")
		return
	var first := empty_parts(ui)
	var shown := Time.get_ticks_msec()
	var settled := first
	var frames := 0
	deadline = shown + ARRIVAL_WAIT_MS
	while not settled.is_empty() and Time.get_ticks_msec() < deadline:
		await process_frame
		frames += 1
		settled = empty_parts(ui)
	print("FIXTURE UI_TEXTURE_ARRIVAL parts=%d empty_first=%d empty_settled=%d after_ms=%d frames=%d settled_names=%s" % [
		count_parts(ui), first.size(), settled.size(), Time.get_ticks_msec() - shown, frames, settled])
	if not settled.is_empty():
		fail("%d parts still lack their art after %d ms" % [settled.size(), ARRIVAL_WAIT_MS])
		return
	print("FIXTURE UI_TEXTURE_ARRIVAL_DONE")
	client.free()
	quit(0)

func count_parts(node: Node) -> int:
	var count := 1 if node is TextureRect else 0
	for child in node.get_children():
		count += count_parts(child)
	return count

## Paths of TextureRect parts without a texture.
func empty_parts(node: Node) -> Array:
	var empty := []
	if node is TextureRect and node.texture == null:
		empty.append(str(node.get_parent().get_parent().name) + "/" + str(node.name))
	for child in node.get_children():
		empty.append_array(empty_parts(child))
	return empty

func fail(message: String) -> void:
	push_error(message)
	print("FIXTURE FAIL ", message)
	if client != null and is_instance_valid(client):
		client.free()
	quit(1)
