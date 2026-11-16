extends RefCounted
var runtime: RefCounted
var cinema: MeshInstance3D
var original_material: Material
var video_assigned: bool = false
var flip_video: bool = false

func _init(native_runtime: RefCounted, screen: MeshInstance3D) -> void:
	runtime = native_runtime
	cinema = screen
	original_material = cinema.material_override

func update(active: bool) -> void:
	if active and not video_assigned:
		var texture: Texture2D = runtime.call("video_texture")
		if texture != null:
			var material := ShaderMaterial.new()
			material.shader = preload("res://shaders/video.gdshader")
			material.set_shader_parameter("video", texture)
			material.set_shader_parameter("flip", flip_video)
			cinema.material_override = material
			video_assigned = true
	if not active and video_assigned:
		cinema.material_override = original_material
		runtime.call("clear_video")
		video_assigned = false

func toggle_flip() -> void:
	flip_video = not flip_video
	if video_assigned:
		cinema.material_override.set_shader_parameter("flip", flip_video)

func dispose() -> void:
	if video_assigned:
		cinema.material_override = original_material
		runtime.call("clear_video")
	video_assigned = false
