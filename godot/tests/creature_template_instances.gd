extends SceneTree
# A Stormwind guard (creature display 3167) built twice as world units: the second visual
# instantiates the first one's template (rust/src/assets/creature_template.rs), makes no
# ShaderMaterial and shares all 11 with the first; it has the same nodes with its own
# animation players, and draws the same pixels. A third guard with its weapons drawn
# instantiates the same template (the sheath state is not part of its key) and holds its
# sword where a guard built drawn holds it; the world's request for it is answered by the
# template at once, without a worker load. A Dark Iron dwarf (display 8803) animates a
# shown material on global sequences only: its second visual makes no material either, and
# its material animation still animates the materials it shares.
const DISPLAY := 3167
const ANIMATED_DISPLAY := 8803
const SIZE := Vector2i(256, 320)
## The Stormwind guard's creature_equip_template row: sword, shield, crossbow.
const ITEMS := [
	{"slot": "MainHand", "item_id": 1899, "inventory_type": 13},
	{"slot": "OffHand", "item_id": 143, "inventory_type": 14},
	{"slot": "Ranged", "item_id": 2551, "inventory_type": 26},
]
const UNARMED := 0
const MELEE := 1
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
	var first: Dictionary = loader.load_world_creature_display(DISPLAY, ITEMS, UNARMED)
	var built: int = counters.take().counts.shader_material
	var second: Dictionary = loader.load_world_creature_display(DISPLAY, ITEMS, UNARMED)
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
	if not check_drawn_instance(loader, counters, a):
		return
	var images := await render([a, b])
	if not await check_shared_clock(loader, counters):
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
## and the visual's material animation samples it
## (the guard's scrolling inputs are second-texture transforms).
func check_shared_clock(loader, counters) -> bool:
	var first: Dictionary = loader.load_world_creature_display(ANIMATED_DISPLAY, [], UNARMED)
	counters.take()
	var second: Dictionary = loader.load_world_creature_display(ANIMATED_DISPLAY, [], UNARMED)
	var made: int = counters.take().counts.shader_material
	if first.has("error") or second.has("error"):
		fail(str(first.get("error", second.get("error"))))
		return false
	root.add_child(first.node)
	root.add_child(second.node)
	var animates := await clock_animates(second.node)
	print("CREATURE_TEMPLATE_CLOCK display=%d materials=%d animates=%s"
		% [ANIMATED_DISPLAY, made, animates])
	if made != 0 or not animates:
		fail("the Dark Iron dwarf's second visual made %d materials and animates: %s"
			% [made, animates])
		return false
	return true

## Whether the clock changes `visual`'s material inputs over a few advances: a global
## sequence can hold a value for a while, so one advance may land in a hold.
func clock_animates(visual: Node) -> bool:
	for delta in [1234.0, 777.0, 2500.0, 4321.0]:
		var before := matrices(visual)
		root.get_node("M2MaterialClock").advance_time_ms(delta)
		await process_frame
		await process_frame
		if matrices(visual) != before:
			return true
	return false

func matrices(visual: Node) -> Array:
	var found := []
	for mesh: MeshInstance3D in visual.find_children("*", "MeshInstance3D", true, false):
		var material := mesh.get_active_material(0) as ShaderMaterial
		if material != null:
			found.append([material.get_shader_parameter("texture_matrix_1"),
				material.get_shader_parameter("texture_matrix_2")])
	return found

## A drawn guard instantiates the unarmed guard's template: no material, and its sword
## sits on the attachment a guard built drawn puts it on, not where `unarmed` keeps it.
func check_drawn_instance(loader, counters, unarmed: Node) -> bool:
	counters.take()
	var drawn: Dictionary = loader.load_world_creature_display(DISPLAY, ITEMS, MELEE)
	var made: int = counters.take().counts.shader_material
	var reference: Dictionary = ClassDB.instantiate("WowAssetLoader").load_world_creature_display(DISPLAY, ITEMS, MELEE)
	if drawn.has("error") or reference.has("error"):
		fail(str(drawn.get("error", reference.get("error"))))
		return false
	var sword := sword_path(drawn.node)
	print("CREATURE_TEMPLATE_DRAWN materials=%d sword=%s reference=%s unarmed=%s"
		% [made, sword, sword_path(reference.node), sword_path(unarmed)])
	if made != 0:
		fail("the drawn guard made %d materials instead of instancing the template" % made)
		return false
	if sword == "" or sword != sword_path(reference.node) or sword == sword_path(unarmed):
		fail("the drawn guard's sword is at %s, a guard built drawn holds it at %s, unarmed at %s"
			% [sword, sword_path(reference.node), sword_path(unarmed)])
		return false
	# The world's request for a unit with the template's display and items is answered at
	# once by the template, with no worker load: the same nodes and sword as the drawn
	# instance, and no material.
	counters.take()
	var requested: Dictionary = loader.request_world_creature_display(DISPLAY, ITEMS, MELEE)
	var request_made: int = counters.take().counts.shader_material
	if requested.has("error"):
		fail(str(requested.error))
		return false
	var request_sword := sword_path(requested.node) if requested.ready else ""
	print("CREATURE_TEMPLATE_REQUEST ready=%s materials=%d sword=%s"
		% [requested.ready, request_made, request_sword])
	if not requested.ready or request_made != 0 or request_sword != sword \
			or describe(requested.node) != describe(drawn.node):
		fail("the request was ready %s with %d materials, its sword at %s and %s nodes"
			% [requested.ready, request_made, request_sword,
				"the drawn instance's" if requested.ready and describe(requested.node) == describe(drawn.node) else "other"])
		return false
	requested.node.free()
	drawn.node.free()
	reference.node.free()
	return true

## The path of the main-hand item's parent under the visual.
func sword_path(visual: Node) -> String:
	var sword := visual.find_child("EquipmentMainHand", true, false)
	return "" if sword == null else str(visual.get_path_to(sword.get_parent()))

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
