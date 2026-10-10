extends "res://tests/skyriding_bar_live.gd"

## Real matrix1251417 aura312 set499: Run5 -> FlyRun223 -> Run5 on expiry.
## Metamorphosis187827 instead swaps display68671, outside local imported coverage.
var skin := ""
const MATRIX_SPELL := 1251417

func run_test() -> void:
	Engine.max_fps = 30
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("ANIMREP_ACCOUNT")
	character = OS.get_environment("ANIMREP_CHARACTER")
	shots = OS.get_environment("ANIMREP_SHOTS")
	skin = OS.get_environment("ANIMREP_SKIN")
	if server != "127.0.0.1:5192" or not account.begins_with("fb_animrep_") or shots == "":
		fail("Owned private endpoint/account/capture directory required")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error: String = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail(error)
		return
	if not await enter_world():
		return
	player = client.get_node("WorldUnits/" + character) as Node3D
	client.set_world_minutes(720.0)
	client.set_camera_orbit(PI, -0.22, 7.0)
	var ready := OS.get_environment("ANIMREP_READY")
	FileAccess.open(ready, FileAccess.WRITE).store_string("online")
	if not await wait_until(func(): return client.spells_state().known.has(MATRIX_SPELL), 30000, "matrix spell learned"):
		return
	push_key(KEY_W, true)
	if not await wait_until(func(): return animation_id() == 5, 10000, "base Run5"):
		return
	await wait_frames(15)
	await capture("before")
	var sent: String = client.use_spell(MATRIX_SPELL)
	if sent != "":
		fail("Matrix cast: " + sent)
		return
	if not await wait_until(func(): return animation_id() == 223, 15000, "aura312 replacement FlyRun223"):
		return
	await wait_frames(15)
	await capture("active")
	print("ANIMREP ACTIVE skin=", skin, " spell=", MATRIX_SPELL, " source=5 destination=", animation_id(), " errors=", client.spells_state().errors)
	if not await wait_until(func(): return animation_id() == 5, 45000, "aura expiry restores Run5"):
		return
	await wait_frames(15)
	await capture("restored")
	push_key(KEY_W, false)
	print("ANIMREP_LIVE PASS skin=", skin, " destination=", animation_id(), " errors=", client.spells_state().errors)
	client.free()
	quit(0)

func animation_id() -> int:
	var animation := player.find_child("M2Animation", true, false)
	if animation == null:
		fail("Live player visual disappeared")
		return -1
	return animation.current_animation_id()

func capture(label: String) -> void:
	await RenderingServer.frame_post_draw
	var path := shots.path_join(skin + "-" + label + ".png")
	var image := root.get_texture().get_image()
	if image.save_png(path) != OK:
		fail("Capture write failed: " + path)
		return
	print("ANIMREP CAPTURE ", path, " animation=", animation_id())
