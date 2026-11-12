extends SceneTree
## Optional GPU smoke: no audio output, XR runtime, credentials or Xbox session.
func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var packed: PackedScene = load("res://scenes/main.tscn")
	var scene = packed.instantiate()
	root.add_child(scene)
	for frame in range(5):
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var output := ProjectSettings.globalize_path("res://../.tools/godot-preview.png")
	var result := image.save_png(output)
	if result != OK:
		push_error("Could not save synthetic preview")
	scene.queue_free()
	await process_frame
	quit(0 if result == OK else 1)
