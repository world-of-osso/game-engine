extends SceneTree

# Real-model M2 batch pixels: named retail models loaded by the native WowAssetLoader and
# drawn by the production material, one batch at a time, against m2_wwv_oracle.gd's CPU
# evaluation of WebWowViewerCpp's retail rules at every unambiguous magnified pixel.
# Deterministic: perspective camera on the batch bounds, fixed scene light, frozen material
# clock and rest pose. Evidence (actual/expected/diff PNG) goes to M2_REAL_EVIDENCE.

const Oracle := preload("res://tests/m2_wwv_oracle.gd")
const DATA := "res://../data/"
const SIZE := 192
const BACKGROUND := Color(0.25, 0.35, 0.45)
const TOLERANCE := 10.0 / 255.0
const MIN_MATCH := 0.97
const SCENE := {
	"ambient": Vector3(0.40, 0.38, 0.36),
	"horizon": Vector3(0.30, 0.32, 0.36),
	"ground": Vector3(0.18, 0.16, 0.14),
	"direct": Vector3(0.55, 0.52, 0.48),
	"sun_direction": Vector3(-0.35, -0.85, -0.40),
}

# fdid, skin batch index (file order), view direction (Godot axes), zoom (<1 closer),
# focus (0..1 inside the batch bounds), time (material clock ms), minimum compared pixels.
const CASES := [
	{"name": "cenarius_additive_glow", "fdid": 1261840, "batch": 1, "view": Vector3(0.2, 0.2, 1.0), "zoom": 0.5},
	{"name": "twilight_dragonspawn_additive_mod2x_scroll", "fdid": 356426, "batch": 0, "view": Vector3(0.3, 0.2, 1.0), "zoom": 0.5, "time": 600},
	{"name": "shade_additive_env", "fdid": 125854, "batch": 2, "view": Vector3(0.2, 0.2, 1.0), "zoom": 0.6},
	{"name": "elwynn_waterfall_uv_scroll", "fdid": 189958, "batch": 0, "view": Vector3(1.0, 0.2, 0.2), "time": 700},
	{"name": "lava_plug_opaque_uv_scroll", "fdid": 189165, "batch": 2, "view": Vector3(0.2, 1.0, 0.3), "zoom": 0.15, "time": 1300},
	{"name": "worldtree_portal_uv_rotation", "fdid": 197068, "batch": 0, "view": Vector3(0.0, 0.2, 1.0), "zoom": 0.3, "time": 900},
	{"name": "ash_tree_alpha_key", "fdid": 189253, "batch": 0, "view": Vector3(1.0, 0.1, 0.3), "zoom": 0.4},
	{"name": "dalaran_tree_mod2xna_alpha_key", "fdid": 1414285, "batch": 0, "view": Vector3(1.0, 0.1, 0.3), "zoom": 0.3},
	{"name": "chopper_env_0x8000", "fdid": 1016630, "batch": 0, "view": Vector3(0.6, 0.3, 1.0), "zoom": 0.3},
	{"name": "helm_env_0x8000", "fdid": 1036841, "batch": 0, "view": Vector3(0.3, 0.2, 1.0)},
	{"name": "blood_elf_jar_mod2x_env", "fdid": 192082, "batch": 1, "view": Vector3(0.3, 0.3, 1.0)},
	{"name": "cave_crystal_mod_env", "fdid": 190809, "batch": 1, "view": Vector3(0.3, 0.3, 1.0)},
	{"name": "twilight_tent_t2", "fdid": 305645, "batch": 2, "view": Vector3(0.6, 0.3, 1.0)},
	{"name": "dalaran_chair_addalpha_alpha", "fdid": 1271625, "batch": 0, "view": Vector3(0.6, 0.4, 1.0), "zoom": 0.5},
	{"name": "nightmare_rays_edge_fade", "fdid": 1399962, "batch": 0, "view": Vector3(0.6, 0.2, 1.0), "zoom": 0.3},
	{"name": "human_male_hd_env_glow", "fdid": 1011653, "batch": 110, "view": Vector3(0.0, 0.1, 1.0)},
]

var loader: Object
var viewport: SubViewport
var camera: Camera3D
var evidence := ""
var ray_origins := PackedVector3Array()
var ray_normals := PackedVector3Array()

func _initialize() -> void:
	call_deferred("run_cases")

func run_cases() -> void:
	if not ClassDB.class_exists("WowAssetLoader"):
		push_error("WowAssetLoader not registered")
		quit(1)
		return
	evidence = OS.get_environment("M2_REAL_EVIDENCE")
	if not evidence.is_empty():
		DirAccess.make_dir_recursive_absolute(evidence)
	loader = ClassDB.instantiate("WowAssetLoader")
	make_viewport()
	var only := OS.get_environment("M2_REAL_CASE")
	var failed: Array[String] = []
	for case in CASES:
		if not only.is_empty() and case.name != only:
			continue
		if not await run_case(case):
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
	camera.far = 1000.0
	viewport.add_child(camera)
	camera.current = true
	var sun := DirectionalLight3D.new()
	sun.shadow_enabled = false
	viewport.add_child(sun)

func read_bytes(path: String) -> PackedByteArray:
	return FileAccess.get_file_as_bytes(path)

# The node of skin batch `source`: WebWowViewer draws every batch (skipping one only
# while its opacity is ~0), in a stable sort by priority plane and material layer.
func node_index(oracle: Oracle, source: int) -> int:
	var order: Array = []
	for index in oracle.batch_count():
		var info := oracle.batch(index)
		order.append([info.priority_plane, info.material_layer, index])
	order.sort_custom(func(a, b): return a[0] < b[0] or (a[0] == b[0] and (a[1] < b[1] or (a[1] == b[1] and a[2] < b[2]))))
	for position in order.size():
		if order[position][2] == source:
			return position
	return -1

# A fresh material clock at `time_ms`, frozen, for the next model's material animation.
func set_clock(time_ms: float) -> void:
	var old := root.get_node_or_null("M2MaterialClock")
	if old != null:
		root.remove_child(old)
		old.free()
	var clock: Node = ClassDB.instantiate("WowMaterialClock")
	clock.name = "M2MaterialClock"
	clock.process_mode = Node.PROCESS_MODE_DISABLED
	root.add_child(clock)
	clock.call("advance_time_ms", time_ms)

func texture_images(material: Dictionary) -> Array:
	var images := []
	for slot in material.textures:
		if slot[0] == 0:
			return []
		var loaded: Dictionary = loader.load_blp(DATA + "textures/%d.blp" % slot[0])
		if not loaded.has("image"):
			push_error("Texture %d: %s" % [slot[0], loaded.get("error")])
			return []
		images.append(mip_chain(loaded.image))
	return images

# Mip levels down to 1x1 by 2x2 averaging (the BLP mips are authored the same way).
static func mip_chain(image: Image) -> Array:
	image.convert(Image.FORMAT_RGBA8)
	var levels := [image]
	var level := image
	while level.get_width() > 1 or level.get_height() > 1:
		level = level.duplicate() as Image
		level.resize(maxi(level.get_width() / 2, 1), maxi(level.get_height() / 2, 1), Image.INTERPOLATE_BILINEAR)
		levels.append(level)
	return levels

func run_case(case: Dictionary) -> bool:
	var model_path := DATA + "models/%d.m2" % case.fdid
	var oracle := Oracle.new(read_bytes(model_path), read_bytes(DATA + "models/%d00.skin" % case.fdid))
	var info := oracle.batch(case.batch)
	var time_ms: int = case.get("time", 0)
	var material := oracle.material(info, time_ms)
	print("%s: shader 0x%x textures %d blend %d -> PS %d VS %d, flags 0x%x" % [case.name, info.shader_id, info.texture_count, info.blend_mode, material.pixel, material.vertex, info.render_flags])
	var images := texture_images(material)
	if images.is_empty():
		push_error("%s: batch textures unavailable" % case.name)
		return false
	set_clock(time_ms)
	var result: Dictionary = loader.load_m2(model_path)
	if result.has("error"):
		push_error("%s: %s" % [case.name, result.error])
		return false
	var model: Node3D = result.node
	viewport.add_child(model)
	var animation := model.get_node_or_null("M2Animation")
	if animation != null:
		animation.process_mode = Node.PROCESS_MODE_DISABLED
	var target := model.get_node_or_null("Batch%d" % node_index(oracle, case.batch)) as MeshInstance3D
	if target == null:
		push_error("%s: no node for batch %d (colour alpha/weight 0 at time 0: %s, weight %s)" % [case.name, case.batch, oracle.mesh_color(info, 0), oracle.weight(info, 0, 0)])
		model.free()
		return false
	for child in model.get_children():
		if child is MeshInstance3D and child != target:
			child.visible = false
	var shader_material := target.get_active_material(0) as ShaderMaterial
	shader_material.set_shader_parameter("ambient", SCENE.ambient)
	shader_material.set_shader_parameter("horizon_ambient", SCENE.horizon)
	shader_material.set_shader_parameter("ground_ambient", SCENE.ground)
	shader_material.set_shader_parameter("direct", SCENE.direct)
	shader_material.set_shader_parameter("sun_direction", SCENE.sun_direction)
	await process_frame
	var mesh := target.bake_mesh_from_current_skeleton_pose()
	var arrays := mesh.surface_get_arrays(0)
	if arrays[Mesh.ARRAY_INDEX].size() != info.triangle_count:
		push_error("%s: node %s has %d indices, batch %d has %d" % [case.name, target.name, arrays[Mesh.ARRAY_INDEX].size(), case.batch, info.triangle_count])
		model.free()
		return false
	frame_camera(arrays[Mesh.ARRAY_VERTEX], case)
	for frame in 3:
		await process_frame
		await RenderingServer.frame_post_draw
	var actual := viewport.get_texture().get_image()
	actual.convert(Image.FORMAT_RGBA8)
	var passed := compare(case, oracle, info, material, images, arrays, actual, target.global_transform, info.render_flags & 4 != 0)
	model.free()
	return passed

func frame_camera(vertices: PackedVector3Array, case: Dictionary) -> void:
	var bounds := AABB(vertices[0], Vector3.ZERO)
	for vertex in vertices:
		bounds = bounds.expand(vertex)
	var focus: Vector3 = case.get("focus", Vector3(0.5, 0.5, 0.5))
	var center := bounds.position + bounds.size * focus
	var radius := bounds.size.length() * 0.5 * float(case.get("zoom", 1.0))
	var direction := (case.view as Vector3).normalized()
	var distance := maxf(radius / sin(deg_to_rad(camera.fov * 0.5)), 0.05)
	var up := Vector3.UP if absf(direction.y) < 0.95 else Vector3.FORWARD
	camera.look_at_from_position(center + direction * distance, center, up)

# Ray-triangle (Moller-Trumbore) without bounds checks: [t, b1, b2].
static func intersect(origin: Vector3, direction: Vector3, a: Vector3, b: Vector3, c: Vector3) -> Array:
	var e1 := b - a
	var e2 := c - a
	var p := direction.cross(e2)
	var det := e1.dot(p)
	if absf(det) < 1e-12:
		return []
	var s := origin - a
	var u := s.dot(p) / det
	var q := s.cross(e1)
	var v := direction.dot(q) / det
	return [e2.dot(q) / det, u, v]

class Hit:
	var t: float
	var triangle: int
	var b1: float
	var b2: float

# Every front (or two-sided) triangle crossing each pixel centre, nearest first.
func cache_rays() -> void:
	ray_origins.resize(SIZE * SIZE)
	ray_normals.resize(SIZE * SIZE)
	for y in SIZE:
		for x in SIZE:
			ray_origins[y * SIZE + x] = camera.project_ray_origin(Vector2(x + 0.5, y + 0.5))
			ray_normals[y * SIZE + x] = camera.project_ray_normal(Vector2(x + 0.5, y + 0.5))

func rasterize(arrays: Array, transform: Transform3D, two_sided: bool) -> Array:
	cache_rays()
	var positions: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
	var hits := []
	hits.resize(SIZE * SIZE)
	for i in hits.size():
		hits[i] = []
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
		var normal := (b - a).cross(c - a)
		for y in range(maxi(int(low.y), 0), mini(int(high.y) + 1, SIZE)):
			for x in range(maxi(int(low.x), 0), mini(int(high.x) + 1, SIZE)):
				var origin := ray_origins[y * SIZE + x]
				var direction := ray_normals[y * SIZE + x]
				# Godot draws clockwise-wound triangles as front faces.
				if not two_sided and normal.dot(direction) <= 0.0:
					continue
				var hit := intersect(origin, direction, a, b, c)
				if hit.is_empty() or hit[1] < 0.0 or hit[2] < 0.0 or hit[1] + hit[2] > 1.0 or hit[0] <= 0.0:
					continue
				var record := Hit.new()
				record.t = hit[0]
				record.triangle = triangle
				record.b1 = hit[1]
				record.b2 = hit[2]
				hits[y * SIZE + x].append(record)
	for list in hits:
		list.sort_custom(func(l, r): return l.t < r.t)
	return hits

# Surface attributes where the pixel ray meets `triangle`'s plane.
func surface(arrays: Array, transform: Transform3D, triangle: int, pixel: Vector2) -> Dictionary:
	var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
	var corner := [indices[triangle * 3], indices[triangle * 3 + 1], indices[triangle * 3 + 2]]
	var positions: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
	var uv: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV]
	var uv2: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV2]
	var a := transform * positions[corner[0]]
	var hit := intersect(camera.project_ray_origin(pixel), camera.project_ray_normal(pixel), a, transform * positions[corner[1]], transform * positions[corner[2]])
	var w := [1.0 - hit[1] - hit[2], hit[1], hit[2]]
	var result := {"position": Vector3.ZERO, "normal": Vector3.ZERO, "uv": Vector2.ZERO, "uv2": Vector2.ZERO}
	for k in 3:
		result.position += (transform * positions[corner[k]]) * w[k]
		result.normal += (transform.basis * normals[corner[k]]) * w[k]
		result.uv += uv[corner[k]] * w[k]
		result.uv2 += uv2[corner[k]] * w[k]
	result.normal = result.normal.normalized()
	return result

func view_space(world: Dictionary) -> Array:
	var view := camera.get_camera_transform().affine_inverse()
	return [view * world.position, (view.basis * world.normal).normalized()]

func uvs_at(material: Dictionary, arrays: Array, transform: Transform3D, triangle: int, pixel: Vector2) -> Array:
	var world := surface(arrays, transform, triangle, pixel)
	var view := view_space(world)
	return [Oracle.vertex_uvs(material.vertex, view[0], view[1], world.uv, world.uv2, material.matrices), world]

static func wrap_coord(value: int, size: int, wrap: bool) -> int:
	return posmod(value, size) if wrap else clampi(value, 0, size - 1)

# Bilinear sample of one level in the texture's stored (authored gamma) space.
static func bilinear(image: Image, uv: Vector2, wrap_x: bool, wrap_y: bool) -> Color:
	var w := image.get_width()
	var h := image.get_height()
	var x := uv.x * w - 0.5
	var y := uv.y * h - 0.5
	var x0 := floori(x)
	var y0 := floori(y)
	var fx := x - x0
	var fy := y - y0
	var c00 := image.get_pixel(wrap_coord(x0, w, wrap_x), wrap_coord(y0, h, wrap_y))
	var c10 := image.get_pixel(wrap_coord(x0 + 1, w, wrap_x), wrap_coord(y0, h, wrap_y))
	var c01 := image.get_pixel(wrap_coord(x0, w, wrap_x), wrap_coord(y0 + 1, h, wrap_y))
	var c11 := image.get_pixel(wrap_coord(x0 + 1, w, wrap_x), wrap_coord(y0 + 1, h, wrap_y))
	return c00.lerp(c10, fx).lerp(c01.lerp(c11, fx), fy)

# Trilinear sample at the GPU's level of detail log2(texels per pixel).
static func sample(levels: Array, uv: Vector2, lod: float, wrap_x: bool, wrap_y: bool) -> Color:
	if lod <= 0.0:
		return bilinear(levels[0], uv, wrap_x, wrap_y)
	var top := levels.size() - 1
	var low := mini(floori(lod), top)
	var high := mini(low + 1, top)
	return bilinear(levels[low], uv, wrap_x, wrap_y).lerp(bilinear(levels[high], uv, wrap_x, wrap_y), lod - floorf(lod))

# Texture coordinate per sampled texture, after calcM2FragMaterial's PS 26/28 override.
static func sampled_uvs(pixel: int, uvs: Array) -> Array:
	if pixel == 26 or pixel == 28:
		return [uvs[0], uvs[0], uvs[0], uvs[1]]
	return [uvs[0], uvs[1], uvs[2], uvs[1]]

# Per texture slot: log2 of the texel step to the next pixel (the GPU's mip selection).
func lods(material: Dictionary, images: Array, arrays: Array, transform: Transform3D, triangle: int, pixel: Vector2, here: Array) -> Array:
	var result := []
	var base: Array = sampled_uvs(material.pixel, here)
	var steps := []
	for offset in [Vector2(1, 0), Vector2(0, 1)]:
		steps.append(sampled_uvs(material.pixel, uvs_at(material, arrays, transform, triangle, pixel + offset)[0]))
	for slot in images.size():
		var size := Vector2(images[slot][0].get_width(), images[slot][0].get_height())
		var rho := 0.0
		for next in steps:
			rho = maxf(rho, ((next[slot] - base[slot]) * size).length())
		result.append(log(maxf(rho, 1e-6)) / log(2.0))
	return result

# The pixel's colour per WebWowViewer: [rgb (authored), alpha, discarded].
func shade(material: Dictionary, images: Array, uvs: Array, world: Dictionary, lod: Array) -> Array:
	var coords := sampled_uvs(material.pixel, uvs)
	var texels := [Color.WHITE, Color.WHITE, Color.WHITE, Color.WHITE]
	for slot in images.size():
		texels[slot] = sample(images[slot], coords[slot], lod[slot], material.textures[slot][1], material.textures[slot][2])
	var mesh: Color = material.mesh_color * float(uvs[3])
	var frag := Oracle.fragment(material.pixel, texels[0], texels[1], texels[2], texels[3], Oracle.rgb(mesh), material.weights)
	var discarded := Oracle.discards(material.gx_blend, frag[2], frag[3])
	var specular: Vector3 = frag[1] * Oracle.rgb(mesh)
	var color := Oracle.light(frag[0], specular, world.normal, material.lit, SCENE)
	return [color, Oracle.final_opacity(material.gx_blend, frag[2], mesh.a), discarded]

static func to_linear(v: Vector3) -> Vector3:
	var c := Color(maxf(v.x, 0.0), maxf(v.y, 0.0), maxf(v.z, 0.0)).srgb_to_linear()
	return Vector3(c.r, c.g, c.b)

# Godot's linear-framebuffer blend of the authored colour over `dst` (the GL factors of
# M2BlendingModeToEGxBlendEnum; Mod2x doubles the colour in authored space).
static func blend(gx_blend: int, color: Vector3, alpha: float, dst: Vector3) -> Vector3:
	# The reference writes to a UNORM attachment: colour and alpha clamp to [0, 1].
	color = color.clamp(Vector3.ZERO, Vector3.ONE)
	var src := to_linear(color * 2.0 if gx_blend == 5 else color)
	alpha = clampf(alpha, 0.0, 1.0)
	match gx_blend:
		2: return src * alpha + dst * (1.0 - alpha)
		10: return src + dst
		3: return src * alpha + dst
		4: return src * dst
		5: return src * dst
		13: return src + dst * (1.0 - alpha)
	return src

static func encoded(linear: Vector3) -> Color:
	var c := Color(linear.x, linear.y, linear.z).linear_to_srgb()
	return Color(clampf(c.r, 0.0, 1.0), clampf(c.g, 0.0, 1.0), clampf(c.b, 0.0, 1.0))

const MAX_LOD := 2.0

# WebWowViewer's draw of one batch over the background at a pixel: triangles in index
# order under the batch's depth test/write (render flags 0x8/0x10). Null when a drawn
# fragment is too minified to predict.
func resolve(material: Dictionary, images: Array, arrays: Array, transform: Transform3D, list: Array, pixel: Vector2, depth_test: bool, depth_write: bool) -> Variant:
	var ordered := list.duplicate()
	ordered.sort_custom(func(l, r): return l.triangle < r.triangle)
	var dst := to_linear(Oracle.rgb(BACKGROUND))
	var depth := INF
	for hit in ordered:
		if depth_test and hit.t >= depth:
			continue
		var at := uvs_at(material, arrays, transform, hit.triangle, pixel)
		var lod := lods(material, images, arrays, transform, hit.triangle, pixel, at[0])
		var shaded := shade(material, images, at[0], at[1], lod)
		if shaded[2]:
			continue
		for value in lod:
			if value > MAX_LOD:
				return null
		dst = blend(material.gx_blend, shaded[0], shaded[1], dst)
		if depth_write:
			depth = hit.t
	return encoded(dst)

func compare(case: Dictionary, oracle: Oracle, info: Dictionary, material: Dictionary, images: Array, arrays: Array, actual: Image, transform: Transform3D, two_sided: bool) -> bool:
	var hits := rasterize(arrays, transform, two_sided)
	var expected := actual.duplicate() as Image
	var diff := Image.create(SIZE, SIZE, false, Image.FORMAT_RGBA8)
	diff.fill(Color(0.0, 0.0, 0.0))
	var compared := 0
	var matched := 0
	var total_error := 0.0
	var worst := 0.0
	var skipped := {"edge": 0, "minified": 0}
	var depth_test: bool = info.render_flags & 8 == 0
	# m2Object.cpp createM2Material: depthWrite = !(flags & 0x10), depthCulling = !(flags & 0x8).
	var depth_write: bool = info.render_flags & 0x10 == 0
	for y in range(1, SIZE - 1):
		for x in range(1, SIZE - 1):
			var list: Array = hits[y * SIZE + x]
			if list.is_empty():
				continue
			var silhouette := false
			for n in [Vector2i(1, 0), Vector2i(-1, 0), Vector2i(0, 1), Vector2i(0, -1)]:
				if (hits[(y + n.y) * SIZE + x + n.x] as Array).is_empty():
					silhouette = true
			if silhouette:
				skipped.edge += 1
				continue
			var want: Variant = resolve(material, images, arrays, transform, list, Vector2(x + 0.5, y + 0.5), depth_test, depth_write)
			if want == null:
				skipped.minified += 1
				continue
			var got := actual.get_pixel(x, y)
			var error := maxf(absf(got.r - want.r), maxf(absf(got.g - want.g), absf(got.b - want.b)))
			compared += 1
			total_error += error
			worst = maxf(worst, error)
			if error <= TOLERANCE:
				matched += 1
			expected.set_pixel(x, y, want)
			diff.set_pixel(x, y, Color(minf(absf(got.r - want.r) * 8.0, 1.0), minf(absf(got.g - want.g) * 8.0, 1.0), minf(absf(got.b - want.b) * 8.0, 1.0)))
	if not evidence.is_empty():
		actual.save_png(evidence.path_join(case.name + "_actual.png"))
		expected.save_png(evidence.path_join(case.name + "_expected.png"))
		diff.save_png(evidence.path_join(case.name + "_diff.png"))
	var minimum: int = case.get("min_pixels", 300)
	var ratio := float(matched) / maxf(compared, 1.0)
	var line := "%s: compared %d matched %d (%.1f%%) mean %.1f max %.1f /255; skipped %s" % [case.name, compared, matched, ratio * 100.0, total_error / maxf(compared, 1.0) * 255.0, worst * 255.0, skipped]
	if compared < minimum or ratio < MIN_MATCH:
		push_error("FAIL " + line)
		return false
	print("PASS " + line)
	return true
