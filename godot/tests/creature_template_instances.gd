extends SceneTree
# A Stormwind guard (creature display 3167) built twice as world units: the second visual
# instantiates the first one's template (rust/src/assets/creature_template.rs). The guard's
# animated material inputs all run on global sequences, so the second makes no
# ShaderMaterial and shares all 16 with the first, while the material clock still animates
# them; it has the same nodes with its own animation players, and draws the same pixels.
const DISPLAY := 3167
const SIZE := Vector2i(256, 320)
const BACKGROUND := Color(0.1, 0.2, 0.3)
## Largest per-channel difference two renders of one pose may show.
const TOLERANCE := 2.0 / 255.0
## Pixels the guard must cover in each render.
const MIN_COVERED := 8000

var viewports: Array[SubViewport] = []

func _initialize() -> void:
	call_deferred("run")

func fail(reason: String) -> void:
	push_error("CREATURE_TEMPLATE_FAIL " + reason)
	quit(1)

func run() -> void:
	var counters = ClassDB.instantiate("RebuildCounters")
	var loader = ClassDB.instantiate("WowAssetLoader")
	counters.take()
	var first: Dictionary = loader.load_world_creature_display(DISPLAY)
	var built: int = counters.take().counts.shader_material
	var second: Dictionary = loader.load_world_creature_display(DISPLAY)
	var instanced: int = counters.take().counts.shader_material
	if first.has("error") or second.has("error"):
		fail(str(first.get("error", second.get("error"))))
		return
	var a: Node3D = first.node
	var b: Node3D = second.node
	var nodes := describe(a)
	if nodes != describe(b):
		fail("the template's visual has other nodes:\n%s\n%s" % [nodes, describe(b)])
		return
	var shared := materials(a)
	var own := materials(b).keys().filter(func(id): return not shared.has(id))
	print("CREATURE_TEMPLATE nodes=%d built_materials=%d instance_materials=%d own=%d shared=%d"
		% [nodes.size(), built, instanced, own.size(), materials(b).size() - own.size()])
	if built == 0 or instanced != 0 or not own.is_empty():
		fail("the second visual made %d materials, has %d of its own and %d shared"
			% [instanced, own.size(), materials(b).size() - own.size()])
		return
	var players_a := a.find_children("*", "WowAnimationPlayer", true, false)
	var players_b := b.find_children("*", "WowAnimationPlayer", true, false)
	if players_a.is_empty() or players_b.size() != players_a.size() \
			or players_b.any(func(player): return players_a.has(player)):
		fail("animation players: %d built, %d instanced" % [players_a.size(), players_b.size()])
		return
	var images := await render([a, b])
	if not await clock_animates(b):
		fail("the material clock does not animate the shared materials")
		return
	var covered := covered_pixels(images[0])
	var difference := max_difference(images[0], images[1])
	print("CREATURE_TEMPLATE_PIXELS covered=%d max_difference=%.4f" % [covered, difference])
	if covered < MIN_COVERED or covered_pixels(images[1]) < MIN_COVERED:
		fail("the guard covers %d and %d pixels" % [covered, covered_pixels(images[1])])
		return
	if difference > TOLERANCE:
		fail("the instance draws other pixels: max channel difference %.4f" % difference)
		return
	print("CREATURE_TEMPLATE_PASS")
	quit(0)

## Whether a texture matrix of `visual`'s materials changes when the material clock runs
## (the guard's scrolling inputs are second-texture transforms).
func clock_animates(visual: Node) -> bool:
	var before := matrices(visual)
	root.get_node("M2MaterialClock").advance_time_ms(1234.0)
	await process_frame
	await process_frame
	return matrices(visual) != before

func matrices(visual: Node) -> Array:
	var found := []
	for mesh: MeshInstance3D in visual.find_children("*", "MeshInstance3D", true, false):
		var material := mesh.get_active_material(0) as ShaderMaterial
		if material != null:
			found.append([material.get_shader_parameter("texture_matrix_1"),
				material.get_shader_parameter("texture_matrix_2")])
	return found

## Every descendant's path and class, in tree order.
func describe(visual: Node) -> Array[String]:
	var nodes: Array[String] = []
	for node in visual.find_children("*", "", true, false):
		nodes.append("%s %s" % [visual.get_path_to(node), node.get_class()])
	return nodes

## The ShaderMaterials the visual's batches draw with, by instance ID.
func materials(visual: Node) -> Dictionary:
	var found := {}
	for mesh: MeshInstance3D in visual.find_children("*", "MeshInstance3D", true, false):
		var material := mesh.get_active_material(0) as ShaderMaterial
		if material != null:
			found[material.get_instance_id()] = true
	return found

## Each visual in its own world, posed as built (players paused), drawn in one frame.
func render(visuals: Array) -> Array[Image]:
	for visual: Node3D in visuals:
		var viewport := SubViewport.new()
		viewport.size = SIZE
		viewport.own_world_3d = true
		viewport.transparent_bg = false
		viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
		var environment := Environment.new()
		environment.background_mode = Environment.BG_COLOR
		environment.background_color = BACKGROUND
		var world := WorldEnvironment.new()
		world.environment = environment
		viewport.add_child(world)
		var sun := DirectionalLight3D.new()
		sun.rotation = Vector3(-0.8, 0.6, 0.0)
		viewport.add_child(sun)
		var camera := Camera3D.new()
		camera.position = Vector3(0.0, 1.0, 2.3)
		viewport.add_child(camera)
		root.add_child(viewport)
		viewport.add_child(visual)
		camera.look_at(Vector3(0.0, 0.95, 0.0))
		for player in visual.find_children("*", "WowAnimationPlayer", true, false):
			player.set_paused(true)
		viewports.append(viewport)
	for frame in range(4):
		await RenderingServer.frame_post_draw
	var images: Array[Image] = []
	for viewport in viewports:
		images.append(viewport.get_texture().get_image())
	return images

func covered_pixels(image: Image) -> int:
	var covered := 0
	for y in image.get_height():
		for x in image.get_width():
			var pixel := image.get_pixel(x, y)
			if absf(pixel.r - BACKGROUND.r) + absf(pixel.g - BACKGROUND.g) + absf(pixel.b - BACKGROUND.b) > 0.05:
				covered += 1
	return covered

func max_difference(a: Image, b: Image) -> float:
	var largest := 0.0
	for y in a.get_height():
		for x in a.get_width():
			var p := a.get_pixel(x, y)
			var q := b.get_pixel(x, y)
			largest = maxf(largest, maxf(absf(p.r - q.r), maxf(absf(p.g - q.g), absf(p.b - q.b))))
	return largest
