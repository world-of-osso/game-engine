extends RefCounted
## Shared fail-closed contract. Count only cases whose checks and awaited steps completed.

var tree: SceneTree
var expected: int
var done := 0
var failed := false
var finished := false

func _init(owner: SceneTree, count: int) -> void:
	tree = owner
	expected = count
	watchdog()

func watchdog() -> void:
	await tree.create_timer(300.0).timeout
	if not finished:
		fail("completion timeout: %d/%d" % [done, expected])

func fail(message: String) -> bool:
	if not failed:
		print("FAIL completion: ", message)
	failed = true
	tree.quit(1)
	return false

func require(condition: bool, message: String = "behavior check failed") -> bool:
	if failed:
		return false
	if not condition:
		return fail(message)
	return true

func step(result: Variant, label: String) -> bool:
	return require(is_same(result, true), "incomplete step: " + label)

func case_done() -> bool:
	if not require(not finished and done < expected, "unexpected extra case"):
		return false
	done += 1
	return true

func finish(result: Variant, exit_tree: bool = true) -> void:
	if not step(result, "top-level fixture") or not require(done == expected, "case count %d/%d" % [done, expected]):
		return
	finished = true
	print("ALL_CASES_DONE %d/%d" % [done, expected])
	if exit_tree:
		tree.quit(0)
