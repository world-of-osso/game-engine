extends "res://tests/toy_effects14_live.gd"
# Actual1973/16739, native Human male -> source10137; cancellation restores native body.
func proof_cases() -> Array:
	return [[1973, 16739, 10137, "Orb of Deception"]]

func capture(label: String) -> void:
	if label == "toy1973-display10137":
		await prove_transformed_locomotion()
		if failed: return
	await super.capture("orb-" + label)

func prove_transformed_locomotion() -> void:
	var unit := client.find_children(character, "Node3D", true, false)[0] as Node3D
	var animation := unit.get_node("NpcVisualRoot/NpcModel/M2Animation")
	var before := unit.global_position
	press_forward(true)
	var deadline := Time.get_ticks_msec() + 5000
	while animation.current_animation_id() != 5 and Time.get_ticks_msec() < deadline:
		await create_timer(0.1).timeout
	var clip: int = animation.current_animation_id()
	var moved := unit.global_position.distance_to(before)
	press_forward(false)
	receipts.append({"phase": "orb-transformed-locomotion", "clip": clip, "moved_yd": moved})
	write_receipts()
	if clip != 5: fail("transformed local model did not animate forward run"); return
	if moved < 0.1: fail("transformed local movement did not reach the world"); return
	await create_timer(0.3).timeout

func press_forward(down: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = KEY_W
	event.physical_keycode = KEY_W
	event.pressed = down
	root.push_input(event, true)
