extends SceneTree

const WORKERS := 4
const MATERIALS_PER_WORKER := 10000
const ROUNDS := 100

var shader: Shader
var threads: Array[Thread] = []
var round_number := 0
var frames := 0

func _initialize() -> void:
	shader = Shader.new()
	shader.code = "shader_type spatial; uniform float value; void fragment() { ALBEDO = vec3(value); }"
	_start_round()

func _start_round() -> void:
	for i in WORKERS:
		var thread := Thread.new()
		threads.append(thread)
		thread.start(_create_materials.bind(i))

func _create_materials(worker: int) -> void:
	for i in MATERIALS_PER_WORKER:
		var material := ShaderMaterial.new()
		material.shader = shader
		material.set_shader_parameter("value", float(i + worker) / MATERIALS_PER_WORKER)
		material.get_rid()

func _process(_delta: float) -> bool:
	frames += 1
	for thread in threads:
		if thread.is_alive():
			return false
	for thread in threads:
		thread.wait_to_finish()
	threads.clear()
	round_number += 1
	if round_number == ROUNDS:
		print("PASS: %d ShaderMaterials across %d frames" % [ROUNDS * WORKERS * MATERIALS_PER_WORKER, frames])
		quit()
	else:
		_start_round()
	return false
