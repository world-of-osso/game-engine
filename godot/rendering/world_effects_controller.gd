extends Node
## World-only contact shading. No camera attributes or compositor/UI passes.

const SSAO_RADIUS := 1.0
const SSAO_INTENSITY := 2.0
const SSAO_POWER := 1.5
const DEPTH_PREPASS := "rendering/driver/depth_prepass/enable"

var _ssao := false
var _original_prepass: Variant = null
# Originals remain untouched; Off restores their identity.
var _environment_originals: Dictionary = {}

func configure(ssao: bool) -> void:
	_ssao = ssao
	if ssao and _original_prepass == null:
		_original_prepass = ProjectSettings.get_setting(DEPTH_PREPASS)
		# Forward+ skips SSAO generation without the depth/normal prepass.
		ProjectSettings.set_setting(DEPTH_PREPASS, true)
	elif not ssao:
		_restore_prepass()
	if is_inside_tree():
		_scan_world(get_viewport())

func _ready() -> void:
	get_tree().node_added.connect(_queue_world_node)
	get_tree().node_removed.connect(_restore_removed_node)
	_scan_world(get_viewport())

func _exit_tree() -> void:
	for world in _environment_originals.keys():
		if is_instance_valid(world):
			_restore_environment(world)
	_restore_prepass()

func _restore_prepass() -> void:
	if _original_prepass != null:
		ProjectSettings.set_setting(DEPTH_PREPASS, _original_prepass)
		_original_prepass = null

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
	if node is WorldEnvironment and node.name == "Environment" and node.get_parent().name == "WorldLighting":
		_apply_ssao(node as WorldEnvironment)

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

func _restore_environment(world: WorldEnvironment) -> void:
	if _environment_originals.has(world):
		world.environment = _environment_originals[world]
		_environment_originals.erase(world)

func _restore_removed_node(node: Node) -> void:
	if node is WorldEnvironment:
		_restore_environment(node as WorldEnvironment)
