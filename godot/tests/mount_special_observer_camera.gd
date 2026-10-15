extends Camera3D
# Fixture-only view: follow the real replicated actor after GameClient updates its camera.
var actor: Node3D

func _process(_delta: float) -> void:
	global_position = actor.global_position + Vector3(-9, 6, -10)
	look_at(actor.global_position + Vector3(0, 2, 0))
	make_current()
