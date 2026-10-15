extends "res://tests/toy_effects48_bashiok_live.gd"
# Observe actual first native roots, without turning/moving to repair startup state.
var first_root_frame: Dictionary = {}
var observed_frame := 0

func _initialize() -> void:
	process_frame.connect(observe_replication_roots)
	super._initialize()

func observe_replication_roots() -> void:
	observed_frame += 1
	if not is_instance_valid(client): return
	var units := client.find_child("WorldUnits", true, false)
	if units == null: return
	for unit in units.get_children():
		observe_root_transform(unit as Node3D)

func observe_root_transform(unit: Node3D) -> void:
	var id := unit.get_instance_id()
	if not first_root_frame.has(id): first_root_frame[id] = observed_frame
	var age: int = observed_frame - int(first_root_frame[id])
	if age not in [0, 1, 10, 60]: return
	receipts.append({"phase": "replication-root-transform", "name": str(unit.get_meta("unit_name")), "age_frames": age, "frame": observed_frame, "position": str(unit.global_position), "yaw": unit.global_rotation.y, "basis_z": str(unit.global_basis.z), "local_character": str(unit.get_meta("unit_name")) == character})
