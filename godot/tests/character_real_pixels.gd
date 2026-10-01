extends SceneTree

# Named characters and clothing built by the world's player loader
# (WowAssetLoader.load_player_customized), checked against character_appearance_oracle.gd,
# an independent model of Retail's appearance rules read straight from the DB2 exports:
# visible geosets, the bound composited textures, and every unambiguous pixel of
# close-up renders against a CPU evaluation (m2_wwv_oracle.gd's WebWowViewerCpp shading)
# of the oracle's geosets and textures on the same posed meshes, camera and light.
# Run with a rendered display (scripts/agent/headless-client.sh start-godot).
# CHARPIX_CASES=Name,Name limits the run; CHARPIX_EVIDENCE receives actual/expected/diff.

const Oracle := preload("res://tests/m2_wwv_oracle.gd")
const Appearance := preload("res://tests/character_appearance_oracle.gd")
const Real := preload("res://tests/m2_real_pixels.gd")
const DATA := "res://../data/"
const SIZE := 256
const BACKGROUND := Color(0.25, 0.35, 0.45)
const TOLERANCE := 10.0 / 255.0
const MIN_MATCH := 0.97
const MIN_PIXELS := 400
const MAX_LOD := 2.0
const SCENE := {
	"ambient": Vector3(0.40, 0.38, 0.36),
	"horizon": Vector3(0.30, 0.32, 0.36),
	"ground": Vector3(0.18, 0.16, 0.14),
	"direct": Vector3(0.55, 0.52, 0.48),
	"sun_direction": Vector3(-0.35, -0.85, -0.40),
}
# Close-ups by height band of the body (0 feet .. 1 crown); side 1 front, -1 back.
const VIEWS := {
	"head": [0.84, 1.0, 1.0], "torso": [0.55, 0.84, 1.0], "back": [0.5, 0.84, -1.0],
	"hips": [0.4, 0.62, 1.0], "legs": [0.15, 0.5, 1.0], "feet": [0.0, 0.16, 1.0],
}

# Item IDs from ItemSparse (build 12.1.0.69933). Choices by ChrCustomizationOption ID;
# options not listed take their first choice.
const CASES := [
	{
		"name": "HumanMaleRecruit", "race": 1, "sex": 0, "class": 1,
		"picks": {9: 3, 10: 22, 11: 46, 12: 64, 13: 78},
		"items": [
			{"slot": "Shirt", "item_id": 38}, {"slot": "Legs", "item_id": 39},
			{"slot": "Feet", "item_id": 40},
		],
		"views": ["head", "torso", "legs", "feet"],
	},
	{
		"name": "HumanMaleBareEyesight", "race": 1, "sex": 0, "class": 1,
		"picks": {9: 1, 11: 44, 6338: 45087},
		"items": [],
		"views": ["head", "torso", "legs"],
	},
	{
		"name": "HumanMaleArmored", "race": 1, "sex": 0, "class": 1, "picks": {},
		"items": [
			{"slot": "Shirt", "item_id": 6096}, {"slot": "Chest", "item_id": 2435},
			{"slot": "Legs", "item_id": 139}, {"slot": "Feet", "item_id": 849},
			{"slot": "Hands", "item_id": 850}, {"slot": "Waist", "item_id": 44670},
			{"slot": "Wrist", "item_id": 710}, {"slot": "Tabard", "item_id": 15197},
			{"slot": "Back", "item_id": 1190},
		],
		"views": ["torso", "back", "hips", "legs", "feet"],
	},
]

var loader: Object
var appearance: Appearance
var viewport: SubViewport
var camera: Camera3D
var evidence := ""
var images := {}

func _initialize() -> void:
	call_deferred("run_cases")

func run_cases() -> void:
	if not ClassDB.class_exists("WowAssetLoader"):
		push_error("WowAssetLoader not registered")
		quit(1)
		return
	evidence = OS.get_environment("CHARPIX_EVIDENCE")
	if not evidence.is_empty():
		DirAccess.make_dir_recursive_absolute(evidence)
	loader = ClassDB.instantiate("WowAssetLoader")
	appearance = Appearance.new()
	make_viewport()
	set_clock()
	var only := OS.get_environment("CHARPIX_CASES").split(",", false)
	var failed: Array[String] = []
	for case in CASES:
		if not only.is_empty() and not only.has(case.name):
			continue
		var problems := await run_case(case)
		for problem in problems:
			push_error("FAIL %s: %s" % [case.name, problem])
		if not problems.is_empty():
			failed.append(case.name)
	viewport.free()
	loader = null
	if failed.is_empty():
		print("ALL PASS")
	else:
		push_error("FAILED: %s" % ", ".join(failed))
	quit(0 if failed.is_empty() else 1)

func make_viewport() -> void:
	viewport = SubViewport.new()
	viewport.size = Vector2i(SIZE, SIZE)
	viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	viewport.own_world_3d = true
	viewport.msaa_3d = Viewport.MSAA_DISABLED
	viewport.screen_space_aa = Viewport.SCREEN_SPACE_AA_DISABLED
	root.add_child(viewport)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = BACKGROUND
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_DISABLED
	environment.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	environment.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var world_environment := WorldEnvironment.new()
	world_environment.environment = environment
	viewport.add_child(world_environment)
	camera = Camera3D.new()
	camera.fov = 30.0
	camera.near = 0.01
	camera.far = 100.0
	viewport.add_child(camera)
	camera.current = true

# A frozen material clock at time 0, so batch colours and texture matrices are static.
func set_clock() -> void:
	var clock: Node = ClassDB.instantiate("WowMaterialClock")
	clock.name = "M2MaterialClock"
	clock.process_mode = Node.PROCESS_MODE_DISABLED
	root.add_child(clock)

func load_image(fdid: int) -> Variant:
	if images.has(fdid):
		return images[fdid]
	var loaded: Dictionary = loader.load_blp(DATA + "textures/%d.blp" % fdid)
	var image: Variant = null
	if loaded.has("image"):
		image = loaded.image
		image.convert(Image.FORMAT_RGBA8)
	images[fdid] = image
	return image

func save(image: Image, name: String) -> void:
	if not evidence.is_empty():
		image.save_png(evidence.path_join(name + ".png"))

# --- one case ------------------------------------------------------------------------------

func run_case(case: Dictionary) -> Array[String]:
	var problems: Array[String] = []
	var app := appearance.appearance(case.race, case.sex, case["class"], case.picks, case.items)
	print("%s: chr model %d layout %d body %d choices %s" % [case.name, app.chr_model, app.layout, app.model_fdid, app.choices])
	print("  materials %s geosets %s displays %s item textures %s cape %d" % [app.materials, app.geosets, app.displays, app.item_textures, app.cape])
	var items := []
	for item in case.items:
		items.append({"slot": item.slot, "item_id": item.item_id})
	var result: Dictionary = loader.load_player_customized(case.race, case.sex, case["class"], items, app.choices)
	if result.has("error"):
		return ["load: %s" % result.error]
	var model: Node3D = result.node
	viewport.add_child(model)
	var animation := model.get_node_or_null("M2Animation")
	if animation != null:
		animation.process_mode = Node.PROCESS_MODE_DISABLED
	var source := String(model.get_meta("m2_source_path"))
	if not source.ends_with("/%d.m2" % app.model_fdid):
		problems.append("body model %s, oracle %d" % [source, app.model_fdid])
		model.free()
		return problems
	var m2 := Oracle.new(FileAccess.get_file_as_bytes(source), FileAccess.get_file_as_bytes(source.trim_suffix(".m2") + "00.skin"))
	var draws := collect_draws(m2, model, app, problems)
	if draws.is_empty():
		model.free()
		return problems
	var canvases := {}
	for draw in draws:
		for slot in draw.types:
			if slot > 0 and not canvases.has(slot):
				canvases[slot] = oracle_texture(app, slot)
	for error in appearance.errors:
		problems.append("oracle: " + error)
	appearance.errors.clear()
	check_textures(case, draws, canvases, problems)
	await process_frame
	for draw in draws:
		draw.mesh = (draw.node as MeshInstance3D).bake_mesh_from_current_skeleton_pose().surface_get_arrays(0)
		var material := (draw.node as MeshInstance3D).get_surface_override_material(0) as ShaderMaterial
		material.set_shader_parameter("ambient", SCENE.ambient)
		material.set_shader_parameter("horizon_ambient", SCENE.horizon)
		material.set_shader_parameter("ground_ambient", SCENE.ground)
		material.set_shader_parameter("direct", SCENE.direct)
		material.set_shader_parameter("sun_direction", SCENE.sun_direction)
		draw.images = draw_images(draw, canvases)
	for view in case.views:
		var line := await check_view(case, view, model, draws)
		if not line.is_empty():
			problems.append(line)
	model.free()
	return problems

# Every body batch the oracle shows, with its node; mismatched visibility is a problem.
func collect_draws(m2: Oracle, model: Node3D, app: Dictionary, problems: Array[String]) -> Array:
	var order: Array = []
	for index in m2.batch_count():
		var info := m2.batch(index)
		order.append([info.priority_plane, info.material_layer, index])
	order.sort_custom(func(a, b): return a[0] < b[0] or (a[0] == b[0] and (a[1] < b[1] or (a[1] == b[1] and a[2] < b[2]))))
	var parts := []
	for entry in order:
		parts.append(mesh_part(m2, m2.batch(entry[2]).submesh))
	var expected := appearance.visible_parts(app, parts)
	var actual := []
	var draws := []
	for position in order.size():
		var node := model.get_node_or_null("Batch%d" % position) as MeshInstance3D
		if node == null or int(node.get_meta("m2_mesh_part")) != parts[position]:
			problems.append("node Batch%d is not skin batch %d (mesh part %d)" % [position, order[position][2], parts[position]])
			return []
		if node.visible and not actual.has(parts[position]):
			actual.append(parts[position])
		if expected.has(parts[position]):
			var info := m2.batch(order[position][2])
			var material := m2.material(info, 0)
			var types := []
			for slot in material.textures.size():
				types.append(m2.texture_type(info, slot))
			draws.append({"node": node, "info": info, "material": material, "types": types, "part": parts[position]})
	actual.sort()
	if actual != expected:
		var extra := actual.filter(func(part): return not expected.has(part))
		var missing := expected.filter(func(part): return not actual.has(part))
		problems.append("geosets: shown but hidden by oracle %s, hidden but shown by oracle %s" % [extra, missing])
	print("  geosets %s" % [expected])
	return draws

static func mesh_part(m2: Oracle, submesh: int) -> int:
	return m2.skin.decode_u16(m2.array_at(m2.skin, 28).y + submesh * 48)

func oracle_texture(app: Dictionary, texture_type: int) -> Variant:
	if texture_type == 2:
		return null if app.cape == 0 else load_image(app.cape)
	return appearance.canvas(app, texture_type, load_image)

# Mip chains per texture slot: the file, or the oracle's canvas for replaceable types.
func draw_images(draw: Dictionary, canvases: Dictionary) -> Array:
	var result := []
	for slot in draw.material.textures.size():
		var image: Variant
		if draw.types[slot] == 0:
			image = load_image(draw.material.textures[slot][0])
		else:
			image = canvases.get(draw.types[slot])
		if image == null:
			return []
		result.append(Real.mip_chain((image as Image).duplicate()))
	return result

# --- bound textures ----------------------------------------------------------------------

# Each replaceable texture the visible batches bind, against the oracle's canvas at the
# same normalized texel centres (at the oracle mip of the bound texture's size).
func check_textures(case: Dictionary, draws: Array, canvases: Dictionary, problems: Array[String]) -> void:
	var checked := {}
	for draw in draws:
		var kind: int = draw.types[0]
		if kind == 0 or checked.has(kind):
			continue
		checked[kind] = true
		var texture: Texture2D = (draw.node as MeshInstance3D).get_surface_override_material(0).get_shader_parameter("base_texture")
		var want: Variant = canvases.get(kind)
		if texture == null or want == null:
			problems.append("texture type %d: bound %s, oracle %s" % [kind, texture, want])
			continue
		var got := texture.get_image()
		got.convert(Image.FORMAT_RGBA8)
		var expected: Image = want
		var levels := Real.mip_chain(expected.duplicate())
		var level := 0
		while level + 1 < levels.size() and levels[level].get_width() > got.get_width():
			level += 1
		var reference: Image = levels[level]
		var compared := 0
		var matched := 0
		var diff := Image.create_empty(got.get_width(), got.get_height(), false, Image.FORMAT_RGBA8)
		for y in got.get_height():
			for x in got.get_width():
				var uv := Vector2((x + 0.5) / got.get_width(), (y + 0.5) / got.get_height())
				var a := got.get_pixel(x, y)
				var b := Real.bilinear(reference, uv, false, false)
				var error := maxf(absf(a.r - b.r), maxf(absf(a.g - b.g), absf(a.b - b.b)))
				compared += 1
				if error <= TOLERANCE:
					matched += 1
				diff.set_pixel(x, y, Color(minf(absf(a.r - b.r) * 8.0, 1.0), minf(absf(a.g - b.g) * 8.0, 1.0), minf(absf(a.b - b.b) * 8.0, 1.0)))
		var name := "%s_type%d" % [case.name, kind]
		save(got, name + "_bound")
		save(expected, name + "_oracle")
		save(diff, name + "_diff")
		var ratio := float(matched) / maxf(compared, 1.0)
		var line := "texture type %d: bound %s (mipmaps %s), oracle %s; %.1f%% of texels within 10/255" % [kind, got.get_size(), got.has_mipmaps(), expected.get_size(), ratio * 100.0]
		print("  " + line)
		if got.get_size() != expected.get_size() or not got.has_mipmaps() or ratio < MIN_MATCH:
			problems.append(line)

# --- rendered pixels ---------------------------------------------------------------------

func check_view(case: Dictionary, view: String, model: Node3D, draws: Array) -> String:
	var band: Array = VIEWS[view]
	frame(draws, band)
	for frame_index in 3:
		await process_frame
		await RenderingServer.frame_post_draw
	var actual := viewport.get_texture().get_image()
	actual.convert(Image.FORMAT_RGBA8)
	var hits := rasterize(draws)
	var expected := actual.duplicate() as Image
	var diff := Image.create_empty(SIZE, SIZE, false, Image.FORMAT_RGBA8)
	diff.fill(Color.BLACK)
	var counts := {"compared": 0, "matched": 0, "edge": 0, "blended": 0, "minified": 0, "untextured": 0}
	var total_error := 0.0
	var worst := 0.0
	for y in range(1, SIZE - 1):
		for x in range(1, SIZE - 1):
			var list: Array = hits[y * SIZE + x]
			if list.is_empty():
				continue
			if is_edge(hits, x, y):
				counts.edge += 1
				continue
			var want: Variant = resolve(draws, list, Vector2(x + 0.5, y + 0.5), counts)
			if want == null:
				continue
			var got := actual.get_pixel(x, y)
			var error := maxf(absf(got.r - want.r), maxf(absf(got.g - want.g), absf(got.b - want.b)))
			counts.compared += 1
			total_error += error
			worst = maxf(worst, error)
			if error <= TOLERANCE:
				counts.matched += 1
			expected.set_pixel(x, y, want)
			diff.set_pixel(x, y, Color(minf(absf(got.r - want.r) * 8.0, 1.0), minf(absf(got.g - want.g) * 8.0, 1.0), minf(absf(got.b - want.b) * 8.0, 1.0)))
	var name := "%s_%s" % [case.name, view]
	save(actual, name + "_actual")
	save(expected, name + "_expected")
	save(diff, name + "_diff")
	var ratio := float(counts.matched) / maxf(counts.compared, 1.0)
	var line := "%s: compared %d matched %d (%.1f%%) mean %.1f max %.1f /255; skipped edge %d blended %d minified %d untextured %d" % [view, counts.compared, counts.matched, ratio * 100.0, total_error / maxf(counts.compared, 1.0) * 255.0, worst * 255.0, counts.edge, counts.blended, counts.minified, counts.untextured]
	print("  " + line)
	if counts.compared < MIN_PIXELS or ratio < MIN_MATCH:
		return line
	return ""

# Camera in front of (or behind) the height band of the posed meshes; the model faces +X.
func frame(draws: Array, band: Array) -> void:
	var bounds := AABB()
	var started := false
	for draw in draws:
		var transform: Transform3D = (draw.node as MeshInstance3D).global_transform
		for vertex in draw.mesh[Mesh.ARRAY_VERTEX]:
			var point: Vector3 = transform * vertex
			bounds = bounds.expand(point) if started else AABB(point, Vector3.ZERO)
			started = true
	var low := bounds.position.y + bounds.size.y * band[0]
	var high := bounds.position.y + bounds.size.y * band[1]
	var center := Vector3(bounds.get_center().x, (low + high) * 0.5, bounds.get_center().z)
	var radius := (high - low) * 0.6
	var distance := radius / sin(deg_to_rad(camera.fov * 0.5))
	var direction := Vector3(1.0, 0.15, 0.25).normalized() * float(band[2])
	camera.look_at_from_position(center + direction * distance, center, Vector3.UP)

class Hit:
	var t: float
	var draw: int
	var triangle: int

# Every front (or two-sided) triangle of every draw crossing each pixel centre.
func rasterize(draws: Array) -> Array:
	var origins := PackedVector3Array()
	var normals := PackedVector3Array()
	origins.resize(SIZE * SIZE)
	normals.resize(SIZE * SIZE)
	for y in SIZE:
		for x in SIZE:
			origins[y * SIZE + x] = camera.project_ray_origin(Vector2(x + 0.5, y + 0.5))
			normals[y * SIZE + x] = camera.project_ray_normal(Vector2(x + 0.5, y + 0.5))
	var hits := []
	hits.resize(SIZE * SIZE)
	for i in hits.size():
		hits[i] = []
	for draw_index in draws.size():
		var draw: Dictionary = draws[draw_index]
		var transform: Transform3D = (draw.node as MeshInstance3D).global_transform
		var two_sided: bool = draw.info.render_flags & 4 != 0
		var positions: PackedVector3Array = draw.mesh[Mesh.ARRAY_VERTEX]
		var indices: PackedInt32Array = draw.mesh[Mesh.ARRAY_INDEX]
		for triangle in indices.size() / 3:
			var a := transform * positions[indices[triangle * 3]]
			var b := transform * positions[indices[triangle * 3 + 1]]
			var c := transform * positions[indices[triangle * 3 + 2]]
			if camera.is_position_behind(a) or camera.is_position_behind(b) or camera.is_position_behind(c):
				continue
			var pa := camera.unproject_position(a)
			var pb := camera.unproject_position(b)
			var pc := camera.unproject_position(c)
			var low := pa.min(pb).min(pc).floor()
			var high := pa.max(pb).max(pc).ceil()
			if high.x < 0 or high.y < 0 or low.x >= SIZE or low.y >= SIZE:
				continue
			var normal := (b - a).cross(c - a)
			for y in range(maxi(int(low.y), 0), mini(int(high.y) + 1, SIZE)):
				for x in range(maxi(int(low.x), 0), mini(int(high.x) + 1, SIZE)):
					var direction := normals[y * SIZE + x]
					# Godot draws clockwise-wound triangles as front faces.
					if not two_sided and normal.dot(direction) <= 0.0:
						continue
					var hit := Real.intersect(origins[y * SIZE + x], direction, a, b, c)
					if hit.is_empty() or hit[1] < 0.0 or hit[2] < 0.0 or hit[1] + hit[2] > 1.0 or hit[0] <= 0.0:
						continue
					var record := Hit.new()
					record.t = hit[0]
					record.draw = draw_index
					record.triangle = triangle
					hits[y * SIZE + x].append(record)
	for list in hits:
		list.sort_custom(func(l, r): return l.t < r.t)
	return hits

# A pixel whose nearest surface differs from a neighbour's: a silhouette or a seam
# between batches, where coverage at the pixel centre decides.
static func is_edge(hits: Array, x: int, y: int) -> bool:
	var here: Array = hits[y * SIZE + x]
	for n in [Vector2i(1, 0), Vector2i(-1, 0), Vector2i(0, 1), Vector2i(0, -1)]:
		var there: Array = hits[(y + n.y) * SIZE + x + n.x]
		if there.is_empty() or there[0].draw != here[0].draw:
			return true
	# Two surfaces within 1 mm of each other: their order is a depth-precision tie.
	return here.size() > 1 and here[1].t - here[0].t < 0.001

# Surface attributes where the pixel ray meets the hit's triangle.
func surface(draw: Dictionary, triangle: int, pixel: Vector2) -> Dictionary:
	var arrays: Array = draw.mesh
	var transform: Transform3D = (draw.node as MeshInstance3D).global_transform
	var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
	var corner := [indices[triangle * 3], indices[triangle * 3 + 1], indices[triangle * 3 + 2]]
	var positions: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
	var uv: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
	var uv2: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV2]
	var hit := Real.intersect(camera.project_ray_origin(pixel), camera.project_ray_normal(pixel), transform * positions[corner[0]], transform * positions[corner[1]], transform * positions[corner[2]])
	var w := [1.0 - hit[1] - hit[2], hit[1], hit[2]]
	var result := {"position": Vector3.ZERO, "normal": Vector3.ZERO, "uv": Vector2.ZERO, "uv2": Vector2.ZERO}
	for k in 3:
		result.position += (transform * positions[corner[k]]) * w[k]
		result.normal += (transform.basis * normals[corner[k]]) * w[k]
		result.uv += uv[corner[k]] * w[k]
		result.uv2 += uv2[corner[k]] * w[k]
	result.normal = result.normal.normalized()
	return result

func uvs_at(draw: Dictionary, triangle: int, pixel: Vector2) -> Array:
	var world := surface(draw, triangle, pixel)
	var view := camera.get_camera_transform().affine_inverse()
	var uvs := Oracle.vertex_uvs(draw.material.vertex, view * world.position, (view.basis * world.normal).normalized(), world.uv, world.uv2, draw.material.matrices)
	return [uvs, world]

func lods(draw: Dictionary, triangle: int, pixel: Vector2, here: Array) -> Array:
	var base: Array = Real.sampled_uvs(draw.material.pixel, here)
	var steps := []
	for offset in [Vector2(1, 0), Vector2(0, 1)]:
		steps.append(Real.sampled_uvs(draw.material.pixel, uvs_at(draw, triangle, pixel + offset)[0]))
	var result := []
	for slot in draw.images.size():
		var size := Vector2(draw.images[slot][0].get_width(), draw.images[slot][0].get_height())
		var rho := 0.0
		for next in steps:
			rho = maxf(rho, ((next[slot] - base[slot]) * size).length())
		result.append(log(maxf(rho, 1e-6)) / log(2.0))
	return result

# The nearest surface's colour per WebWowViewer over the background; null (counted)
# where a blended batch covers it, its texture is minified past MAX_LOD or unknown.
func resolve(draws: Array, list: Array, pixel: Vector2, counts: Dictionary) -> Variant:
	for hit in list:
		var draw: Dictionary = draws[hit.draw]
		var material: Dictionary = draw.material
		if material.gx_blend > 1:
			counts.blended += 1
			return null
		if draw.images.is_empty():
			counts.untextured += 1
			return null
		var at := uvs_at(draw, hit.triangle, pixel)
		var lod := lods(draw, hit.triangle, pixel, at[0])
		var coords := Real.sampled_uvs(material.pixel, at[0])
		var texels := [Color.WHITE, Color.WHITE, Color.WHITE, Color.WHITE]
		for slot in draw.images.size():
			texels[slot] = Real.sample(draw.images[slot], coords[slot], lod[slot], material.textures[slot][1], material.textures[slot][2])
		var mesh: Color = material.mesh_color * float(at[0][3])
		var frag := Oracle.fragment(material.pixel, texels[0], texels[1], texels[2], texels[3], Oracle.rgb(mesh), material.weights)
		if Oracle.discards(material.gx_blend, frag[2], frag[3]):
			continue
		for value in lod:
			if value > MAX_LOD:
				counts.minified += 1
				return null
		var color := Oracle.light(frag[0], frag[1] * Oracle.rgb(mesh), at[1].normal, material.lit, SCENE)
		var dst := Real.to_linear(Oracle.rgb(BACKGROUND))
		return Real.encoded(Real.blend(material.gx_blend, color, Oracle.final_opacity(material.gx_blend, frag[2], mesh.a), dst))
	return Real.encoded(Real.to_linear(Oracle.rgb(BACKGROUND)))
