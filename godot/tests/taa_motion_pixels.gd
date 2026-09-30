extends "res://tests/taa_options_pixels.gd"
## Observable production motion/settling acceptance, not temporal-equivalence proof.

const MOTION_DRAWS := 16
const CAMERA_DRAWS := 8
const SETTLED_DRAWS := 16
const STEP_PIXELS := 12.0
const EXPOSE_PIXELS := 480.0
const INTERIOR := Vector3(0.0, -0.12, 0.0)


func run_follow_up(client: Node, _path: String, directory: String) -> bool:
	var camera := root.get_node_or_null("AntiAliasFixtureCamera") as Camera3D
	var quad := find_fixture_quad()
	var menu := client.get_node_or_null("GameMenuUI") as CanvasLayer
	if camera == null or quad == null or menu == null:
		fail("Motion fixture requires base camera, root quad and real menu")
		return false
	var object_origin := quad.global_transform
	var camera_origin := camera.global_transform
	# Derive world steps from the authored perspective, never captured pixels.
	var focal := float(SIZE.y) / (2.0 * tan(deg_to_rad(camera.fov) * 0.5))
	var depth := camera_origin.origin.z - object_origin.origin.z
	var world_step := STEP_PIXELS * depth / focal
	var was_visible := menu.visible
	menu.hide()
	for frame in range(MOTION_DRAWS):
		quad.global_position = object_origin.origin + Vector3(world_step * (frame + 1), 0, 0)
		await RenderingServer.frame_post_draw
		var stage := "object-%02d" % frame
		if not capture_motion(directory, stage, project_interior(quad, camera, focal)):
			return false
	quad.global_transform = object_origin
	for frame in range(SETTLED_DRAWS):
		await RenderingServer.frame_post_draw
	# Positive camera translation makes the fixed quad move in the opposite screen direction.
	for frame in range(CAMERA_DRAWS):
		camera.global_position = camera_origin.origin + Vector3(world_step * (frame + 1), 0, 0)
		await RenderingServer.frame_post_draw
		var stage := "camera-%02d" % frame
		if not capture_motion(directory, stage, project_interior(quad, camera, focal)):
			return false
	camera.global_transform = camera_origin
	for frame in range(SETTLED_DRAWS):
		await RenderingServer.frame_post_draw
	var exposed := project_interior(quad, camera, focal)
	if not capture_motion(directory, "before-disocclusion", exposed):
		return false
	quad.global_position = object_origin.origin + Vector3(EXPOSE_PIXELS * depth / focal, 0, 0)
	for frame in range(SETTLED_DRAWS):
		await RenderingServer.frame_post_draw
	var displaced := project_interior(quad, camera, focal)
	if not capture_motion(directory, "disocclusion-settled", displaced, exposed):
		return false
	quad.global_transform = object_origin
	camera.global_transform = camera_origin
	for frame in range(SETTLED_DRAWS):
		await RenderingServer.frame_post_draw
	menu.visible = was_visible
	var restored := await capture_options_pixels(client, directory, "taa-motion-restored.png")
	if (
		not expect_renderer_aa("motion restored")
		or not expect_aa_pixels(restored, "motion restored")
	):
		return false
	print("PASS: fixed object/camera motion, settled disocclusion, exact UI and restored Taa edges")
	return true


func find_fixture_quad() -> MeshInstance3D:
	var fixture: MeshInstance3D = null
	# add_aa_scene attaches its unnamed quad directly to root, not GameClient.
	for child in root.get_children():
		if child is MeshInstance3D and child.mesh is QuadMesh:
			if fixture != null:
				fail("Motion fixture found multiple root QuadMesh instances")
				return null
			fixture = child as MeshInstance3D
	return fixture


func project_interior(quad: MeshInstance3D, camera: Camera3D, focal: float) -> Vector2:
	# Analytic perspective projection of a known local point inside the rotated quad.
	# No unproject_position or jittered production projection/output oracle.
	var view_point: Vector3 = (
		camera.global_transform.affine_inverse() * (quad.global_transform * INTERIOR)
	)
	return CENTER + Vector2(view_point.x, -view_point.y) * (focal / -view_point.z)


func capture_motion(
	directory: String, stage: String, white: Vector2, black: Vector2 = Vector2(-1, -1)
) -> bool:
	var image := root.get_texture().get_image()
	if not expect_motion_image(image, stage) or not expect_patch(image, white, Color.WHITE, stage):
		return false
	if black.x >= 0 and not expect_patch(image, black, Color.BLACK, stage):
		return false
	var error := image.save_png(directory.path_join("taa-motion-" + stage + ".png"))
	if error != OK:
		fail(stage + ": save motion capture: " + error_string(error))
		return false
	return true


func expect_motion_image(image: Image, stage: String) -> bool:
	if image == null or image.get_size() != SIZE:
		fail(stage + ": motion capture must be full 1280x720")
		return false
	for y in range(SIZE.y):
		for x in range(SIZE.x):
			var color := image.get_pixel(x, y)
			if (
				not is_finite(color.r)
				or not is_finite(color.g)
				or not is_finite(color.b)
				or not is_finite(color.a)
			):
				fail("%s: nonfinite pixel at (%d,%d)" % [stage, x, y])
				return false
	for y in range(UI_RECT.position.y, UI_RECT.end.y):
		for x in range(UI_RECT.position.x, UI_RECT.end.x):
			var expected := Color.WHITE if x < UI_RECT.position.x + 24 else Color.BLACK
			if image.get_pixel(x, y) != expected:
				fail("%s: exact higher UI changed at (%d,%d)" % [stage, x, y])
				return false
	return true


func expect_patch(image: Image, point: Vector2, expected: Color, stage: String) -> bool:
	var center := Vector2i(floori(point.x), floori(point.y))
	var patch := Rect2i(center - Vector2i(2, 2), Vector2i(5, 5))
	if not Rect2i(Vector2i.ZERO, SIZE).encloses(patch) or patch.intersects(UI_RECT):
		fail(stage + ": projected patch outside scene or under UI: " + str(point))
		return false
	for y in range(patch.position.y, patch.end.y):
		for x in range(patch.position.x, patch.end.x):
			var actual := image.get_pixel(x, y)
			if (
				absf(actual.r - expected.r) > BYTE_TOLERANCE
				or absf(actual.g - expected.g) > BYTE_TOLERANCE
				or absf(actual.b - expected.b) > BYTE_TOLERANCE
				or absf(actual.a - expected.a) > BYTE_TOLERANCE
			):
				fail(
					(
						"%s: projected patch (%d,%d)=%s expected %s within 1/255"
						% [stage, x, y, actual, expected]
					)
				)
				return false
	return true
