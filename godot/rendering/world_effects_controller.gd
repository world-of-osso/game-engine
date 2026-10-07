extends Node
## World-only practical DOF and contact shading. No compositor/UI passes.

const FOCUS_DISTANCE := 15.0
const FOCUS_TRANSITION := 5.0
const BLUR_AMOUNT := 0.1
const SSAO_RADIUS := 1.0
const SSAO_INTENSITY := 2.0
const SSAO_POWER := 1.5

var _dof := false
var _ssao := false
# Originals remain untouched, including null camera attributes. Off restores identity.
var _camera_originals: Dictionary = {}
var _environment_originals: Dictionary = {}

func configure(dof: bool, ssao: bool) -> void:
	_dof = dof
	_ssao = ssao
	if is_inside_tree():
		_scan_world(get_viewport())

func _ready() -> void:
	get_tree().node_added.connect(_queue_world_node)
	get_tree().node_removed.connect(_restore_removed_node)
	_scan_world(get_viewport())

func _scan_world(node: Node) -> void:
	_apply_world_node(node)
	for child in node.get_children():
		_scan_world(child)

func _queue_world_node(node: Node) -> void:
	# node_added runs before all descendants/properties have finished attaching.
	_apply_world_node.call_deferred(node)

func _apply_world_node(node: Node) -> void:
	if not is_instance_valid(node) or not node.is_inside_tree() or node.get_viewport() != get_viewport():
		return
	if node is Camera3D and node.name == "WorldCamera":
		_apply_dof(node as Camera3D)
	elif node is WorldEnvironment and node.name == "Environment" and node.get_parent().name == "WorldLighting":
		_apply_ssao(node as WorldEnvironment)

func _apply_dof(camera: Camera3D) -> void:
	if not _dof:
		_restore_camera(camera)
		return
	if _camera_originals.has(camera):
		return
	var original := camera.attributes
	if original != null and not original is CameraAttributesPractical:
		push_error("World DOF requires practical camera attributes")
		return
	var attributes := CameraAttributesPractical.new() if original == null else original.duplicate() as CameraAttributesPractical
	attributes.dof_blur_far_enabled = true
	attributes.dof_blur_far_distance = FOCUS_DISTANCE
	attributes.dof_blur_far_transition = FOCUS_TRANSITION
	attributes.dof_blur_near_enabled = true
	attributes.dof_blur_near_distance = FOCUS_DISTANCE
	attributes.dof_blur_near_transition = FOCUS_TRANSITION
	attributes.dof_blur_amount = BLUR_AMOUNT
	_camera_originals[camera] = original
	camera.attributes = attributes

func _apply_ssao(world: WorldEnvironment) -> void:
	if not _ssao:
		_restore_environment(world)
		return
	if _environment_originals.has(world) or world.environment == null:
		return
	var original := world.environment
	var environment := original.duplicate() as Environment
	environment.ssao_enabled = true
	environment.ssao_radius = SSAO_RADIUS
	environment.ssao_intensity = SSAO_INTENSITY
	environment.ssao_power = SSAO_POWER
	# Retail ambient is emitted through custom light(), not engine ambient.
	environment.ssao_light_affect = 1.0
	environment.ssao_ao_channel_affect = 1.0
	_environment_originals[world] = original
	world.environment = environment

func _restore_camera(camera: Camera3D) -> void:
	if _camera_originals.has(camera):
		camera.attributes = _camera_originals[camera]
		_camera_originals.erase(camera)

func _restore_environment(world: WorldEnvironment) -> void:
	if _environment_originals.has(world):
		world.environment = _environment_originals[world]
		_environment_originals.erase(world)

func _restore_removed_node(node: Node) -> void:
	if node is Camera3D:
		_restore_camera(node as Camera3D)
	elif node is WorldEnvironment:
		_restore_environment(node as WorldEnvironment)
