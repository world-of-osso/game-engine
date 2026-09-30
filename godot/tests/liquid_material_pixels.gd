extends SceneTree

## Non-water retail liquid materials (liquid_*.gdshader) GPU fixture. Each material is built
## from local DB2/CASC by WowTerrainLoader.load_liquid_material and lit at noon, then drawn
## as a quad over an opaque backing plane 2 yd below:
##   magma: Searing Gorge's LiquidObject 413 (LiquidType 7 "Slow Magma") is opaque (the
##     backing never shows through), lava-coloured, and animates with the material clock;
##   every other LiquidMaterial shader renders over the backing and animates.
## Env LIQUID_CASES limits the run to comma-separated case names; LIQUID_SHOTS saves each
## case's capture over the red backing there.

const NOON := 1440.0
const SIZE := Vector2i(96, 96)
const WATER_Y := 100.0
const CENTRE := Vector3(-7700.0, WATER_Y, 1300.0)
## name: [liquid_type, liquid_object, map_id]
const CASES := {
	"magma": [7, 413, 0],
	"slime": [4, 4, 0],
	"mercury": [350, 5, 0],
	"fog": [733, 0, 0],
	"ley_line": [868, 0, 0],
	"fel": [869, 0, 0],
	"swamp": [897, 0, 0],
	"azerite": [940, 0, 0],
}

var viewport: SubViewport
var quad: MeshInstance3D
var backing: StandardMaterial3D

func _initialize() -> void:
	call_deferred("run")

func run() -> void:
	if DisplayServer.get_name() == "headless":
		fail("Liquid material pixels require a display")
		return
	make_viewport()
	var only := OS.get_environment("LIQUID_CASES")
	for name in CASES:
		if only != "" and not (name in only.split(",")):
			continue
		var error := await check_case(name, CASES[name])
		if error != "":
			fail(name + ": " + error)
			return
	print("PASS: LIQUID_MATERIAL_PIXELS")
	quit(0)

func check_case(name: String, liquid: Array) -> String:
	var wow := Vector3(CENTRE.x, -CENTRE.z, CENTRE.y)
	var loader = ClassDB.instantiate("WowTerrainLoader")
	var result: Dictionary = loader.load_liquid_material(liquid[0], liquid[1], liquid[2], wow, NOON)
	if result.has("error"):
		return "load_liquid_material: " + str(result.error)
	var material: ShaderMaterial = result.material
	quad.set_surface_override_material(0, material)
	material.set_shader_parameter("animation_time_ms", 1000.0)
	quad.visible = false
	backing.albedo_color = Color(0.8, 0.1, 0.1)
	var bare := await capture()
	quad.visible = true
	var red := await capture()
	backing.albedo_color = Color(0.1, 0.1, 0.8)
	var blue := await capture()
	material.set_shader_parameter("animation_time_ms", 3000.0)
	var later := await capture()
	var shots := OS.get_environment("LIQUID_SHOTS")
	if shots != "":
		DirAccess.make_dir_recursive_absolute(shots)
		red.save_png(shots + "/" + name + ".png")
	var covered := changed_pixels(bare, red, 0.05)
	var moved := changed_pixels(blue, later, 0.004)
	var response := color_distance(mean(red), mean(blue)) / color_distance(Color(0.8, 0.1, 0.1), Color(0.1, 0.1, 0.8))
	print("LIQUID ", name, " covered=", covered, " moved=", moved, " response=", response, " colour=", mean(red))
	if covered < SIZE.x * SIZE.y / 2:
		return "Liquid changed only %d of %d pixels over the backing" % [covered, SIZE.x * SIZE.y]
	if moved < 200:
		return "Liquid did not animate with the clock: %d pixels moved" % moved
	if name in ["magma", "slime"]:
		return check_magma(name, mean(red), response)
	return ""

## Magma is opaque and its colour comes from its lava textures over the Int[2] orange.
func check_magma(name: String, colour: Color, response: float) -> String:
	if response > 0.02:
		return "Magma is not opaque: backing response %f" % response
	if name == "magma" and not (colour.r > 0.3 and colour.r > colour.g and colour.g > colour.b):
		return "Magma colour %s is not lava-like (r > g > b)" % colour
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
	camera.fov = 40.0
	viewport.add_child(camera)
	camera.look_at_from_position(CENTRE + Vector3(0.0, 8.0, 3.0), CENTRE, Vector3.UP)
	camera.current = true
	quad = MeshInstance3D.new()
	quad.mesh = liquid_quad()
	viewport.add_child(quad)
	var plane := MeshInstance3D.new()
	var mesh := PlaneMesh.new()
	mesh.size = Vector2(400.0, 400.0)
	plane.mesh = mesh
	plane.position = CENTRE - Vector3(0.0, 2.0, 0.0)
	backing = StandardMaterial3D.new()
	backing.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	plane.material_override = backing
	viewport.add_child(plane)

## A 40 yd liquid quad with authored vertex depth 80/255.
func liquid_quad() -> ArrayMesh:
	var vertices := PackedVector3Array()
	for corner in [Vector2(-20, -20), Vector2(20, -20), Vector2(-20, 20), Vector2(20, 20)]:
		vertices.append(CENTRE + Vector3(corner.x, 0.0, corner.y))
	var colors := PackedColorArray()
	colors.resize(4)
	colors.fill(Color(1.0, 1.0, 1.0, 80.0 / 255.0))
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

func mean(image: Image) -> Color:
	var total := Color(0, 0, 0, 0)
	for y in range(24, 72):
		for x in range(24, 72):
			total += image.get_pixel(x, y)
	return total / 2304.0

func changed_pixels(first: Image, second: Image, threshold: float) -> int:
	var changed := 0
	for y in SIZE.y:
		for x in SIZE.x:
			if color_distance(first.get_pixel(x, y), second.get_pixel(x, y)) > threshold:
				changed += 1
	return changed

func color_distance(a: Color, b: Color) -> float:
	return absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b)

func fail(message: String) -> void:
	push_error(message)
	quit(1)
