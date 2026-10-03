extends SceneTree

## Retail water material (water.gdshader) GPU fixture. The material is Northshire's stream
## water, MH2O LiquidType 5 / LiquidObject 427, built from local DB2/CASC and lit by the
## authored light at the vineyard stream at noon. Two water quads (left/right) float over
## opaque backing planes; each check compares the two halves or two captures:
##   1. vertex depth: the deeper authored vertex depth hides more of the backing,
##   2. depth 0: an authored depth-0 vertex is invisible (shore cut),
##   3. thickness: backing far below the surface is fogged towards the underwater colour,
##   4. colour: shallow water over black matches the retail close/far river mix,
##   5. animation: the material clock moves the bump normals.

const WOW_POSITION := Vector3(-9030.0, -260.0, 70.2)
const WATER := Vector3(-9030.0, 70.2, 260.0)
const NOON := 1440.0
const SIZE := Vector2i(128, 64)
const HALF_WIDTH := 4.0
const BACKING_WIDTH := 400.0

var material: ShaderMaterial
## The scene light `load_liquid_material` bound (`TerrainLight::scene_state`).
var scene_light: Dictionary
var viewport: SubViewport
var water_root: Node3D
var backing_root: Node3D

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	if DisplayServer.get_name() == "headless":
		fail("Water material pixels require a display")
		return
	var loader = ClassDB.instantiate("WowTerrainLoader")
	var result: Dictionary = loader.load_liquid_material(5, 427, 0, WOW_POSITION, NOON)
	if result.has("error"):
		fail("load_liquid_material: " + str(result.error))
		return
	material = result.material
	scene_light = result.scene_light
	make_viewport()
	var checks := [check_vertex_depth, check_depth_zero, check_thickness, check_colour, check_animation]
	for check in checks:
		var error: String = await check.call()
		if error != "":
			fail(error)
			return
	print("PASS: WATER_MATERIAL_PIXELS")
	quit(0)

## Backing response: how much of a red→blue backing change shows through, per half.
func backing_response(depths: Vector2, backing_below: Vector2) -> Vector2:
	build(depths, backing_below, Color(0.8, 0.1, 0.1))
	var red := await capture()
	set_backing_color(Color(0.1, 0.1, 0.8))
	var blue := await capture()
	var baseline := color_distance(Color(0.8, 0.1, 0.1), Color(0.1, 0.1, 0.8))
	return Vector2(
		color_distance(mean(red, 0), mean(blue, 0)) / baseline,
		color_distance(mean(red, 1), mean(blue, 1)) / baseline)

func check_vertex_depth() -> String:
	var response := await backing_response(Vector2(10.0, 80.0) / 255.0, Vector2(2.0, 2.0))
	print("WATER depth response shallow=", response.x, " deep=", response.y)
	if not (response.x < 0.9 and response.y < response.x - 0.05):
		return "Deeper authored vertex depth did not hide more backing: %s" % response
	return ""

func check_depth_zero() -> String:
	var response := await backing_response(Vector2(0.0, 40.0 / 255.0), Vector2(2.0, 2.0))
	print("WATER depth0 response=", response.x, " wet=", response.y)
	if response.x < 0.98 or response.y > 0.9:
		return "Authored depth 0 is not invisible while depth 40 covers: %s" % response
	return ""

func check_thickness() -> String:
	var response := await backing_response(Vector2(40.0, 40.0) / 255.0, Vector2(1.0, 60.0))
	print("WATER thickness response thin=", response.x, " thick=", response.y)
	if not (response.y < response.x - 0.03):
		return "Backing 60 yd below is not fogged more than 1 yd below: %s" % response
	return ""

## Shallow water 2 yd over black: retail colour = mix(fogged backing, lit river colour,
## (1 - 0.1 fresnel) alpha) + f9 N·L direct lit colour, with close/far mixed by depth.
func check_colour() -> String:
	var depth := 20.0 / 255.0
	build(Vector2(depth, depth), Vector2(2.0, 2.0), Color.BLACK)
	var image := await capture()
	var coefficients: Vector4 = material.get_shader_parameter("depth_coefficients")
	var d := depth
	var mix_amount := clampf(coefficients.x + coefficients.y * d + coefficients.z * d * d + coefficients.w * d * d * d, 0.0, 1.0)
	var close: Vector4 = material.get_shader_parameter("river_close")
	var far: Vector4 = material.get_shader_parameter("river_far")
	var water := close.lerp(far, mix_amount)
	# The scene-lit material reads the scene light's global uniforms.
	var ambient: Vector3 = scene_light.ambient
	var direct: Vector3 = scene_light.direct
	var sun: Vector3 = scene_light.sun_direction
	var n_dot_l := clampf(-sun.y, 0.0, 1.0)
	var lit := Vector3(water.x, water.y, water.z) * (direct * n_dot_l + ambient)
	var fog_color: Vector3 = material.get_shader_parameter("underwater_fog_color")
	var fog: Vector3 = material.get_shader_parameter("underwater_fog")
	var visibility := minf(exp(-maxf(0.0, 2.0 - fog.x) * fog.z), clampf((1.0 / 0.7) * (1.0 - 2.0 / fog.y), 0.0, 1.0))
	var below := fog_color * (1.0 - visibility)
	var floats_8: Vector4 = material.get_shader_parameter("floats_8")
	var expected := below.lerp(lit, water.w) + floats_8.y * n_dot_l * direct * lit
	var got := mean(image, 0)
	print("WATER colour expected=", expected, " got=", got, " close=", close, " far=", far)
	var error := absf(got.r - expected.x) + absf(got.g - expected.y) + absf(got.b - expected.z)
	if error > 0.08:
		return "Shallow river colour %s differs from retail %s by %f" % [got, expected, error]
	return ""

func check_animation() -> String:
	build(Vector2(40.0, 40.0) / 255.0, Vector2(2.0, 2.0), Color(0.3, 0.3, 0.3))
	material.set_shader_parameter("animation_time_ms", 0.0)
	var first := await capture()
	var again := await capture()
	material.set_shader_parameter("animation_time_ms", 2000.0)
	var later := await capture()
	var stable := changed_pixels(first, again)
	var moved := changed_pixels(again, later)
	print("WATER animation stable=", stable, " moved=", moved)
	if stable != 0 or moved < 200:
		return "Water normals did not animate with the clock: stable=%d moved=%d" % [stable, moved]
	return ""

func make_viewport() -> void:
	viewport = SubViewport.new()
	viewport.size = SIZE
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	viewport.own_world_3d = true
	root.add_child(viewport)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color.BLACK
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	viewport.add_child(world_environment)
	var camera := Camera3D.new()
	camera.fov = 30.0
	camera.near = 0.5
	camera.far = 200.0
	viewport.add_child(camera)
	camera.look_at_from_position(WATER + Vector3(0.0, 8.0, 0.0), WATER, Vector3.FORWARD)
	camera.current = true
	water_root = Node3D.new()
	viewport.add_child(water_root)
	backing_root = Node3D.new()
	viewport.add_child(backing_root)

## Left and right water quads with authored vertex `depths` over backing planes
## `backing_below` yards under the surface.
func build(depths: Vector2, backing_below: Vector2, backing: Color) -> void:
	for child in water_root.get_children() + backing_root.get_children():
		child.free()
	for side in 2:
		var x0 := WATER.x - HALF_WIDTH * 2.0 + side * HALF_WIDTH * 2.0
		var quad := MeshInstance3D.new()
		quad.mesh = water_quad(x0, x0 + HALF_WIDTH * 2.0, depths[side])
		quad.set_surface_override_material(0, material)
		water_root.add_child(quad)
		# Each backing is the half-space under its half of the view, so rays that diverge
		# towards deep backing still end on it.
		var plane := MeshInstance3D.new()
		var mesh := PlaneMesh.new()
		mesh.size = Vector2(BACKING_WIDTH, BACKING_WIDTH)
		plane.mesh = mesh
		var centre := WATER.x + (side * 2 - 1) * BACKING_WIDTH / 2.0
		plane.position = Vector3(centre, WATER.y - backing_below[side], WATER.z)
		var backing_material := StandardMaterial3D.new()
		backing_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		backing_material.albedo_color = backing
		plane.material_override = backing_material
		backing_root.add_child(plane)

func set_backing_color(color: Color) -> void:
	for plane in backing_root.get_children():
		(plane.material_override as StandardMaterial3D).albedo_color = color

func water_quad(x0: float, x1: float, depth: float) -> ArrayMesh:
	var z0 := WATER.z - 20.0
	var z1 := WATER.z + 20.0
	var vertices := PackedVector3Array([
		Vector3(x0, WATER.y, z0), Vector3(x1, WATER.y, z0),
		Vector3(x0, WATER.y, z1), Vector3(x1, WATER.y, z1)])
	var colors := PackedColorArray()
	colors.resize(4)
	colors.fill(Color(1.0, 1.0, 1.0, depth))
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = PackedVector3Array([Vector3.UP, Vector3.UP, Vector3.UP, Vector3.UP])
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_INDEX] = PackedInt32Array([0, 1, 2, 1, 3, 2])
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh

func capture() -> Image:
	for frame in 3:
		await process_frame
		await RenderingServer.frame_post_draw
	return viewport.get_texture().get_image()

## Mean colour of the central 24x24 pixels of half `side`.
func mean(image: Image, side: int) -> Color:
	var total := Color(0, 0, 0, 0)
	var centre := Vector2i(SIZE.x / 4 + side * SIZE.x / 2, SIZE.y / 2)
	for y in range(centre.y - 12, centre.y + 12):
		for x in range(centre.x - 12, centre.x + 12):
			total += image.get_pixel(x, y)
	return total / 576.0

func changed_pixels(first: Image, second: Image) -> int:
	var changed := 0
	for y in SIZE.y:
		for x in SIZE.x:
			if color_distance(first.get_pixel(x, y), second.get_pixel(x, y)) > 0.004:
				changed += 1
	return changed

func color_distance(a: Color, b: Color) -> float:
	return absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
