extends SceneTree

# Native physics/rendering fixture using the real Barrens M2 and the same swept
# contact entry point used by player-controlled mount flight.
const TREE_MODEL := "res://../data/models/201394.m2"
const TREE_LAYER := 1 << 4
const RADIUS := 1.25
var tree: Node3D
var other: Node3D
var controller: Node3D
var shots := ""

func _initialize() -> void:
	Engine.max_fps = 60
	call_deferred("run_test")

func run_test() -> void:
	root.size = Vector2i(960, 640)
	shots = OS.get_environment("ELASTIC_TREE_SHOTS")
	if shots != "":
		DirAccess.make_dir_recursive_absolute(shots)
	var loader := WowAssetLoader.new()
	tree = load_tree(loader)
	other = load_tree(loader)
	if tree == null or other == null:
		return
	other.position = Vector3(90, 5, 0)
	other.rotation.y = PI / 3
	other.scale = Vector3.ONE * 0.5
	root.add_child(tree)
	root.add_child(other)
	controller = tree.get_node("ElasticTree")
	var camera := Camera3D.new()
	root.add_child(camera)
	camera.position = Vector3(65, 35, 70)
	camera.look_at(Vector3(0, 18, 0))
	camera.current = true
	camera.far = 200
	await frames(3)
	await snapshot("rest")
	if not check_trunk():
		return
	var foliage_from := Vector3(20, 32, -15)
	var foliage_to := Vector3(20, 32, 15)
	if move_probe(tree, foliage_from, foliage_to).distance_to(foliage_to) > 0.001:
		fail("Foliage blocked airborne contact")
		return
	var other_controller := other.get_node("ElasticTree")
	var local_from := Vector3(-10, 10, 0)
	var local_to := Vector3(10, 10, 0)
	var transformed := move_probe(other, other.to_global(local_from), other.to_global(local_to), RADIUS * 0.5)
	if absf(other.to_local(transformed).x + 4.26) > 0.03:
		fail("Placed tree did not respect rotation/scale: %s" % other.to_local(transformed))
		return
	var thin := impact_branch(0)
	var thick := impact_branch(1)
	if thin < 0 or thick < 0 or thin <= thick:
		fail("Thin limb must yield more: remaining travel thin=%s thick=%s" % [thin, thick])
		return
	await frames(6)
	var state: Array = controller.branch_state()
	var thin_angle: float = state[0].rotation.length()
	var thick_angle: float = state[1].rotation.length()
	if thin_angle < 0.01 or thin_angle <= thick_angle:
		fail("Thin branch did not bend visibly more: %s / %s" % [thin_angle, thick_angle])
		return
	for branch in other_controller.branch_state():
		if branch.rotation.length() > 0.00001:
			fail("Contact changed another placement")
			return
	await snapshot("impact")
	print("TRACE thin angle=", thin_angle, " thick angle=", thick_angle)
	# Recontact a moving limb, without resetting its current pose.
	var before: Vector3 = controller.branch_state()[0].rotation
	impact_branch(0)
	var after: Vector3 = controller.branch_state()[0].rotation
	if before.distance_to(after) > 0.00001:
		fail("Repeated contact snapped the branch pose")
		return
	await frames(360)
	for branch in controller.branch_state():
		if branch.rotation.length() > 0.001:
			fail("Branch did not recover: " + str(branch))
			return
	if not check_trunk():
		return
	await snapshot("recovered")
	var space := tree.get_world_3d().direct_space_state
	tree.queue_free()
	other.queue_free()
	await frames(3)
	var ray := PhysicsRayQueryParameters3D.create(Vector3(-10, 10, 0), Vector3(10, 10, 0), TREE_LAYER)
	if not space.intersect_ray(ray).is_empty():
		fail("Unloading the tree retained collision")
		return
	print("ELASTIC_TREE_CONTACT PASS: trunk/glancing/high-speed/foliage/thin-thick/placement/recontact/recovery/unload")
	quit(0)

func load_tree(loader: Object) -> Node3D:
	var loaded: Dictionary = loader.load_m2(TREE_MODEL)
	if loaded.has("error"):
		fail("Real tree load: " + str(loaded.error))
		return null
	return loaded.node

func move_probe(node: Node3D, from: Vector3, to: Vector3, radius := RADIUS) -> Vector3:
	return WowElasticTree.move_airborne(node, from, to, radius, 0.2)

func check_trunk() -> bool:
	var head_on := move_probe(tree, Vector3(-100, 10, 0), Vector3(100, 10, 0))
	if absf(head_on.x + 4.26) > 0.03:
		fail("Fast impact penetrated fixed trunk: " + str(head_on))
		return false
	var glancing := move_probe(tree, Vector3(-10, 10, 3.8), Vector3(10, 10, 3.8))
	if glancing.x < 2 or glancing.z < 4.5:
		fail("Glancing trunk impact stopped tangential travel: " + str(glancing))
		return false
	print("TRACE fixed trunk head-on=", head_on, " glancing=", glancing)
	return true

func impact_branch(index: int) -> float:
	var branch: Dictionary = controller.branch_state()[index]
	var axis: Vector3 = (branch.tip - branch.pivot).normalized()
	var side := axis.cross(Vector3.UP).normalized()
	var center: Vector3 = branch.pivot.lerp(branch.tip, 0.8)
	var from := center - side * 8
	var to := center + side * 8
	var moved := move_probe(tree, from, to)
	var travel := (moved - from).dot(side)
	if travel >= 15.99 or travel <= 8:
		fail("Elastic limb did not yield with bounded resistance: branch=%d travel=%s" % [index, travel])
		return -1
	return travel

func frames(count: int) -> void:
	for _frame in count:
		await physics_frame
		await process_frame

func snapshot(label: String) -> void:
	if shots == "":
		return
	await RenderingServer.frame_post_draw
	var path := shots.path_join("elastic-tree-" + label + ".png")
	var error := root.get_texture().get_image().save_png(path)
	if error != OK:
		fail("Screenshot failed: %s (%s)" % [path, error])
		return
	print("TRACE screenshot ", path)

func fail(message: String) -> void:
	push_error(message)
	for node in [tree, other]:
		if is_instance_valid(node):
			node.queue_free()
	quit(1)
