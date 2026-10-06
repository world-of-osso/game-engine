extends "res://tests/world_quest_flow.gd"

# Regression for the live fixture at 2 FPS: each held-input frame moves 3.5 yards.
# Sampling every six frames oscillates across an NPC without observing melee range.
var feet := Vector3.ZERO
var heading := PI / 2.0
var walking := false
var npc: Node3D

func run_test() -> void:
	npc = Node3D.new()
	root.add_child(npc)
	npc.position = Vector3(5.0, 0.0, 0.0)
	var reached := await approach(1, 3.0)
	if not reached or walking:
		push_error("Slow-frame NPC approach failed or left forward held: feet=%s held=%s" % [feet, walking])
		quit(1)
		return
	print("FIXTURE QUEST_FLOW_MOVEMENT_PASS feet=", feet)
	npc.free()
	quit(0)

func _process(_delta: float) -> bool:
	if walking:
		feet += Vector3(sin(heading), 0.0, cos(heading)) * 3.5
	return false

func player_position() -> Vector3:
	return feet

func unit_by_id(_id: int) -> Node:
	return npc

func yaw() -> float:
	return heading

func face_unit(_id: int) -> void:
	var to := npc.position - feet
	heading = atan2(to.x, to.z)

func face_direction(want: float) -> void:
	heading = want

func push_key(code: Key, pressed: bool) -> void:
	if code == KEY_W:
		walking = pressed
