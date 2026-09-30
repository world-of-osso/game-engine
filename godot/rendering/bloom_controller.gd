extends Node

const BloomEffect = preload("res://rendering/bloom_effect.gd")

var _effect = BloomEffect.new()


func _init() -> void:
	configure(false, 0.08)


func configure(enabled: bool, intensity: float) -> void:
	_effect.configure(enabled, intensity)


func _ready() -> void:
	get_tree().node_added.connect(_attach_camera)
	_scan_cameras(get_viewport())


func _scan_cameras(node: Node) -> void:
	_attach_camera(node)
	for child in node.get_children():
		_scan_cameras(child)


func _attach_camera(node: Node) -> void:
	if not node is Camera3D or node.get_viewport() != get_viewport():
		return
	var camera := node as Camera3D
	var compositor := camera.compositor
	if compositor != null and compositor.compositor_effects.has(_effect):
		return
	if compositor == null:
		compositor = Compositor.new()
	else:
		# Keep existing effects shared, but isolate this camera's effect list.
		compositor = compositor.duplicate() as Compositor
	var effects := compositor.compositor_effects
	effects.append(_effect)
	compositor.compositor_effects = effects
	camera.compositor = compositor
