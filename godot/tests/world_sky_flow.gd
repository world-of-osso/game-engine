extends "res://tests/capture_world_view.gd"

## The retail exterior sky dome behind the world (WebWowViewerCpp map.cpp skyConus,
## skyConus.frag.slang), in the real client on a private server. Environment:
##   GODOT_TEST_SERVER, VIEW_ACCOUNT, VIEW_CHARACTER, VIEW_SHOTS   as capture_world_view.gd
##   SKY_CASES   "name,minutes,yaw,pitch,distance" joined by ";"
## Each case sets the time of day and camera, captures the frame, and checks every
## sampled upper-screen pixel against the dome colour along that pixel's view ray,
## computed here from the dome's authored LightData stops.

const SKY_SHADER := "res://shaders/sky_dome.gdshader"
# 8-bit rounding and the steep sun-scatter gradient sampled from the
# camera after the captured frame.
const TOLERANCE := 4.0 / 255.0
const SAMPLE_COLUMNS := [0.4, 0.5, 0.6]
const SAMPLE_ROWS := [0.03, 0.1, 0.2]

func run_test() -> void:
	root.size = Vector2i(1280, 720)
	var server := OS.get_environment("GODOT_TEST_SERVER")
	var account := OS.get_environment("VIEW_ACCOUNT")
	character = OS.get_environment("VIEW_CHARACTER")
	shots = OS.get_environment("VIEW_SHOTS")
	var cases := OS.get_environment("SKY_CASES")
	if server == "" or account == "" or character == "" or shots == "" or cases == "":
		fail("GODOT_TEST_SERVER, VIEW_ACCOUNT, VIEW_CHARACTER, VIEW_SHOTS and SKY_CASES are required")
		return
	DirAccess.make_dir_recursive_absolute(shots)
	client = load("res://scenes/client.tscn").instantiate()
	root.add_child(client)
	var error = client.connect_account(server, account, PASSWORD, false)
	if error != "":
		fail("Fixture connection: " + error)
		return
	if not await enter_world():
		return
	var tops := {}
	for entry in cases.split(";", false):
		var fields := entry.split(",")
		if fields.size() != 5:
			fail("Case %s is not name,minutes,yaw,pitch,distance" % entry)
			return
		client.set_world_minutes(float(fields[1]))
		client.set_camera_orbit(float(fields[2]), float(fields[3]), float(fields[4]))
		await wait_real(2.0)
		var material := sky_material()
		if material == null:
			return
		await capture(fields[0] + ".png")
		var image := root.get_texture().get_image()
		if not check_dome(fields[0], image, material):
			return
		tops[fields[0]] = image.get_pixelv(sample_point(image, 0.5, 0.02))
	if tops.size() >= 2 and tops.values()[0].is_equal_approx(tops.values()[1]):
		fail("Sky did not change with the time of day: %s" % tops)
		return
	print("FIXTURE WORLD_SKY_DONE ", tops)
	client.free()
	quit(0)

func sky_material() -> ShaderMaterial:
	var holder := client.get_node_or_null("WorldLighting/Environment") as WorldEnvironment
	if holder == null or holder.environment == null:
		fail("World has no lighting environment")
		return null
	var environment := holder.environment
	if environment.background_mode != Environment.BG_SKY or environment.sky == null:
		fail("World background is %d with sky %s, not the retail sky dome" % [environment.background_mode, environment.sky])
		return null
	var material := environment.sky.sky_material as ShaderMaterial
	if material == null or material.shader == null or material.shader.resource_path != SKY_SHADER:
		fail("World sky material is not %s" % SKY_SHADER)
		return null
	return material

func sample_point(image: Image, column: float, row: float) -> Vector2i:
	return Vector2i(int(image.get_width() * column), int(image.get_height() * row))

func check_dome(name: String, image: Image, material: ShaderMaterial) -> bool:
	var camera := client.get_node("WorldCamera") as Camera3D
	var scale := Vector2(root.size) / Vector2(image.get_size())
	for row in SAMPLE_ROWS:
		for column in SAMPLE_COLUMNS:
			var point := sample_point(image, column, row)
			var direction := camera.project_ray_normal(Vector2(point) * scale)
			var want := expected_color(material, direction)
			var got := image.get_pixelv(point)
			var delta := maxf(absf(got.r - want.r), maxf(absf(got.g - want.g), absf(got.b - want.b)))
			print("FIXTURE SKY %s pixel=%s elevation=%.2f got=%s want=%s" % [name, point, rad_to_deg(asin(direction.y)), got, want])
			if delta > TOLERANCE:
				fail("%s: sky pixel %s is %s, the dome along %s is %s" % [name, point, got, direction, want])
				return false
	return true

## sky_dome.gdshader's colour along `direction`, in authored (displayed) space.
func expected_color(material: ShaderMaterial, direction: Vector3) -> Color:
	var stops: PackedVector3Array = material.get_shader_parameter("sky_stops")
	var points: PackedVector2Array = material.get_shader_parameter("dome_points")
	var s := direction.y
	var c := Vector2(direction.x, direction.z).length()
	var elevation := atan2(s, c)
	var color := authored(stops[6])
	for index in 6:
		var upper := points[index]
		var lower := points[index + 1]
		if elevation > atan2(lower.y, lower.x) or index == 5:
			var d := lower - upper
			var denominator := d.x * s - d.y * c
			var t := 0.0 if absf(denominator) <= 1.1920929e-7 else clampf((upper.y * c - upper.x * s) / denominator, 0.0, 1.0)
			color = authored(stops[index]).lerp(authored(stops[index + 1]), t)
			break
	var sun_angle: float = material.get_shader_parameter("fog_sun_angle")
	var sun_direction: Vector3 = material.get_shader_parameter("fog_sun_direction")
	var n_dot_sun := clampf(direction.dot(sun_direction) - sun_angle, 0.0, 1.0)
	if n_dot_sun > 0.0:
		var scatter := pow(n_dot_sun / (1.0 - sun_angle), 3.0)
		var percentage: float = material.get_shader_parameter("fog_sun_percentage")
		var sun_color: Vector3 = material.get_shader_parameter("fog_sun_color")
		color = color.lerp(authored(sun_color), clampf(scatter * percentage, 0.0, 1.0))
	return color

func authored(linear: Vector3) -> Color:
	return Color(linear.x, linear.y, linear.z).linear_to_srgb()
