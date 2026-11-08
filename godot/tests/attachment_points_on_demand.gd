extends SceneTree
# A creature's M2 attachment points are made on first use (rust/src/assets/attachments.rs):
# a Stormwind guard (creature display 3167) holds only the points its items use. A point
# asked for while the guard animates in the tree is posed at once, where its bone's current
# pose puts it; asking again returns the same node; an attachment the model lacks is null.
const DISPLAY := 3167
const ITEMS := [
	{"slot": "MainHand", "item_id": 1899, "inventory_type": 13},
	{"slot": "OffHand", "item_id": 143, "inventory_type": 14},
	{"slot": "Ranged", "item_id": 2551, "inventory_type": 26},
]
## Attachment 2 (HandLeft): no guard item uses it.
const UNUSED := 2
## The guard model has no attachment 200.
const ABSENT := 200
const TOLERANCE := 0.0001

func _initialize() -> void:
	call_deferred("run")

func fail(reason: String) -> void:
	push_error("ATTACHMENT_POINTS_FAIL " + reason)
	quit(1)

func run() -> void:
	var loader = ClassDB.instantiate("WowAssetLoader")
	var built: Dictionary = loader.load_world_creature_display(DISPLAY, ITEMS, 0)
	if built.has("error"):
		fail(str(built.error))
		return
	var visual: Node3D = built.node
	var model: Node3D = visual.get_node("NpcModel")
	var skeleton: Skeleton3D = model.get_node("Skeleton3D")
	var held := skeleton.find_children("*", "BoneAttachment3D", false, false).size()
	root.add_child(visual)
	for frame in 10:
		await process_frame
	if model.has_node("Skeleton3D/AttachmentBone%d" % UNUSED):
		fail("the unused attachment %d exists before it is asked for" % UNUSED)
		return
	var point: Node3D = loader.attachment_point(model, UNUSED)
	if point == null:
		fail("attachment %d was not made" % UNUSED)
		return
	var bone: BoneAttachment3D = point.get_parent()
	var expected := skeleton.global_transform * skeleton.get_bone_global_pose(bone.bone_idx) \
		* point.transform
	var offset := point.global_transform.origin.distance_to(expected.origin)
	var again: Node3D = loader.attachment_point(model, UNUSED)
	var absent = loader.attachment_point(model, ABSENT)
	print("ATTACHMENT_POINTS held=%d offset=%.6f same=%s absent=%s"
		% [held, offset, again == point, absent])
	if held == 0 or held > 10:
		fail("the guard holds %d attachment points" % held)
		return
	if offset > TOLERANCE:
		fail("the new point is %.6f from its bone's current pose" % offset)
		return
	if again != point or absent != null:
		fail("asking again returned another node, or the absent attachment exists")
		return
	print("ATTACHMENT_POINTS_PASS")
	quit(0)
