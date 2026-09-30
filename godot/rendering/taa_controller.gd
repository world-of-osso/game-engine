extends Node
## Root-viewport AA owner. Projection changes exist only across one draw.
## Bloom inserts legacy Hdr permanently for that camera's lifetime; keep that
## history-format state even while TAA is disabled or Bloom is later removed.

const TaaEffect = preload("res://rendering/taa_effect.gd")
const HDR_META := &"native_taa_legacy_hdr"
const HALTON := [
	Vector2(0, 0),
	Vector2(0, -0.16666666),
	Vector2(-0.25, 0.16666669),
	Vector2(0.25, -0.3888889),
	Vector2(-0.375, -0.055555552),
	Vector2(0.125, 0.2777778),
	Vector2(-0.125, -0.2777778),
	Vector2(0.375, 0.055555582),
]

var _active := false
var _bloom_enabled := false
var _camera: Camera3D
var _effect
var _saved_projection: Dictionary = {}
var _previous_jitter_uv := Vector2.ZERO


func configure(active: bool, bloom_enabled: bool) -> void:
	_bloom_enabled = bloom_enabled
	if bloom_enabled:
		_mark_hdr_cameras(get_viewport())
	_active = active
	if not active:
		_release_camera()


func _ready() -> void:
	get_tree().node_added.connect(_mark_hdr_camera)
	RenderingServer.frame_pre_draw.connect(_before_draw)
	RenderingServer.frame_post_draw.connect(_restore_projection)


func _exit_tree() -> void:
	RenderingServer.frame_pre_draw.disconnect(_before_draw)
	RenderingServer.frame_post_draw.disconnect(_restore_projection)
	_release_camera()


func _mark_hdr_camera(node: Node) -> void:
	if _bloom_enabled and node is Camera3D and node.get_viewport() == get_viewport():
		node.set_meta(HDR_META, true)


func _mark_hdr_cameras(node: Node) -> void:
	_mark_hdr_camera(node)
	for child in node.get_children():
		_mark_hdr_cameras(child)


func _before_draw() -> void:
	if not _active:
		return
	var camera := get_viewport().get_camera_3d()
	if camera == null:
		_release_camera()
		return
	if not is_instance_valid(_camera) or camera != _camera:
		_release_camera()
		_attach_camera(camera)
	if not _effect.snapshot().error.is_empty():
		_active = false
		_release_camera()
		push_error("Native TAA failed; projection jitter stopped")
		return
	if camera.projection != Camera3D.PROJECTION_PERSPECTIVE:
		_active = false
		_release_camera()
		push_error("Native TAA requires a perspective camera")
		return
	var output_size := get_viewport().get_texture().get_size()
	var size := Vector2i(output_size * get_viewport().scaling_3d_scale)
	if size.x <= 0 or size.y <= 0:
		return
	_apply_jitter(camera, size)


func _attach_camera(camera: Camera3D) -> void:
	_camera = camera
	_effect = TaaEffect.new()
	_previous_jitter_uv = Vector2.ZERO
	var compositor := camera.compositor
	if compositor == null:
		compositor = Compositor.new()
	else:
		compositor = compositor.duplicate() as Compositor
	var effects := compositor.compositor_effects
	# TAA precedes Bloom at the shared POST_TRANSPARENT callback.
	effects.push_front(_effect)
	compositor.compositor_effects = effects
	camera.compositor = compositor


func _apply_jitter(camera: Camera3D, size: Vector2i) -> void:
	_saved_projection = {
		"fov": camera.fov,
		"near": camera.near,
		"far": camera.far,
		"size": camera.size,
		"offset": camera.frustum_offset,
	}
	var phase: Vector2 = HALTON[Engine.get_frames_drawn() % HALTON.size()]
	var lens := phase * Vector2(1, -1)
	var jitter_uv := lens / Vector2(size.x, -size.y)
	_effect.configure(jitter_uv - _previous_jitter_uv, bool(camera.get_meta(HDR_META, false)))
	_previous_jitter_uv = jitter_uv
	var span := 2.0 * camera.near * tan(deg_to_rad(camera.fov) / 2.0)
	var pixels := size.x if camera.keep_aspect == Camera3D.KEEP_WIDTH else size.y
	camera.set_frustum(span, lens * span / pixels, camera.near, camera.far)


func _restore_projection() -> void:
	if _saved_projection.is_empty():
		return
	if is_instance_valid(_camera):
		_camera.size = _saved_projection.size
		_camera.frustum_offset = _saved_projection.offset
		_camera.set_perspective(
			_saved_projection.fov, _saved_projection.near, _saved_projection.far
		)
	_saved_projection.clear()


func _release_camera() -> void:
	_restore_projection()
	if _effect == null:
		_camera = null
		return
	if is_instance_valid(_camera) and _camera.compositor != null:
		var compositor := _camera.compositor.duplicate() as Compositor
		var effects := compositor.compositor_effects
		effects.erase(_effect)
		compositor.compositor_effects = effects
		_camera.compositor = null if effects.is_empty() else compositor
	_effect.dispose()
	_effect = null
	_camera = null
