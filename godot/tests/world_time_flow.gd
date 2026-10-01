extends "res://tests/capture_world_view.gd"

## The world's time of day follows the server's game time (TrinityCore
## SMSG_LOGIN_SET_TIME_SPEED: the realm's local wall clock at real-time speed), not a fixed
## noon: after entering the world, the half-minute of the day the light samples matches the
## local clock of the machine the private server runs on, within a minute, and advances
## with it. Environment as capture_world_view.gd; VIEW_SHOTS gets a capture.

const TOLERANCE := 2.0

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("VIEW_ACCOUNT")
	character = OS.get_environment("VIEW_CHARACTER")
	shots = OS.get_environment("VIEW_SHOTS")
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	for sample in range(2):
		if sample == 1:
			await wait_real(65.0)
		var got: float = client.world_minutes()
		var want := local_half_minutes()
		var difference := absf(got - want)
		difference = minf(difference, 2880.0 - difference)
		print("FIXTURE WORLD_TIME sample=%d world_minutes=%.2f local=%.2f" % [sample, got, want])
		if difference > TOLERANCE:
			fail("World time %.2f half-minutes, local clock %.2f" % [got, want])
			return
	await capture("world_time.png")
	print("FIXTURE WORLD_TIME_DONE")
	client.free()
	quit(0)

func local_half_minutes() -> float:
	var now := Time.get_time_dict_from_system()
	return (now.hour * 3600 + now.minute * 60 + now.second) / 30.0
