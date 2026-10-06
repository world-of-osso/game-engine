extends "res://tests/world_quest_flow.gd"

# Drive the fixture's real input events at deterministic 500 ms movement frames.
# Six-frame walks skip melee range; key turns oscillate past the 0.15 rad tolerance.
class InputClient extends Node:
	var heading := 0.0
	var turn_axis := 0.0
	var right_held := false

	func _input(event: InputEvent) -> void:
		if event is InputEventKey:
			if event.keycode == KEY_LEFT:
				turn_axis = 1.0 if event.pressed else 0.0
			elif event.keycode == KEY_RIGHT:
				turn_axis = -1.0 if event.pressed else 0.0
		elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_RIGHT:
			right_held = event.pressed
		elif event is InputEventMouseMotion and right_held:
			heading -= event.relative.x * 0.01

	func _process(_delta: float) -> void:
		heading += turn_axis * 2.5 * 0.5

	func account_state() -> Dictionary:
		return {"camera_yaw": heading - PI}

var feet := Vector3.ZERO
var walking := false
var npc: Node3D
var input_client: InputClient

func run_test() -> void:
	input_client = InputClient.new()
	client = input_client
	root.add_child(client)
	await super.face_direction(0.4)
	if abs(wrapf(0.4 - yaw(), -PI, PI)) >= 0.15 or input_client.turn_axis != 0.0 or input_client.right_held:
		push_error("Slow-frame turn failed or left input held: yaw=%s" % yaw())
		quit(1)
		return
	input_client.heading = PI / 2.0
	npc = Node3D.new()
	root.add_child(npc)
	npc.position = Vector3(5.0, 0.0, 0.0)
	var reached := await approach(1, 3.0)
	if not reached or walking:
		push_error("Slow-frame NPC approach failed or left forward held: feet=%s held=%s" % [feet, walking])
		quit(1)
		return
	print("FIXTURE QUEST_FLOW_MOVEMENT_PASS feet=", feet, " yaw=", yaw())
	npc.free()
	client.free()
	quit(0)

func _process(_delta: float) -> bool:
	if walking:
		feet += Vector3(sin(yaw()), 0.0, cos(yaw())) * 3.5
	return false

func player_position() -> Vector3:
	return feet

func unit_by_id(_id: int) -> Node:
	return npc

func yaw() -> float:
	return input_client.heading

func face_unit(_id: int) -> void:
	var to := npc.position - feet
	input_client.heading = atan2(to.x, to.z)

func face_direction(want: float) -> void:
	input_client.heading = want

func push_key(code: Key, pressed: bool) -> void:
	if code == KEY_W:
		walking = pressed
	else:
		super.push_key(code, pressed)
