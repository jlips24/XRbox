extends SceneTree
## Optional GPU smoke: no audio output, XR runtime, credentials or Xbox session.
func _initialize() -> void:
	call_deferred("capture")

func capture() -> void:
	var packed: PackedScene = load("res://scenes/main.tscn")
	var scene = packed.instantiate()
	root.add_child(scene)
	var result := OK
	# Each product page uses synthetic state; never initialize a session.
	for page in ["home", "settings", "display", "placement", "environment", "account", "consoles", "screen"]:
		if page == "screen":
			scene.set_menu_open(false)
		else:
			scene.live.page = page
			scene.live._rebuild()
		for frame in range(5):
			await process_frame
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		var filename: String = "godot-preview.png" if page == "home" else "godot-preview-%s.png" % page
		var output := ProjectSettings.globalize_path("res://../.tools/" + filename)
		if image.save_png(output) != OK:
			result = ERR_CANT_CREATE
			push_error("Could not save synthetic preview: " + page)
	if not await check_room_darkness(scene):
		result = ERR_INVALID_DATA
	scene.queue_free()
	await process_frame
	quit(0 if result == OK else 1)

func rendered_image() -> Image:
	for frame in range(5):
		await process_frame
	await RenderingServer.frame_post_draw
	return root.get_texture().get_image()

func check_room_darkness(scene: Node) -> bool:
	# Synthetic room behind the dimmer, while menu and screen remain in front.
	# No passthrough device/runtime is involved in this draw-order regression.
	var room := MeshInstance3D.new()
	var quad := QuadMesh.new()
	quad.size = Vector2(200, 200)
	room.mesh = quad
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_color = Color(0.6, 0.6, 0.6)
	room.material_override = material
	scene.add_child(room)
	room.global_transform = scene.camera.global_transform * Transform3D(Basis.IDENTITY, Vector3(0, 0, -80))
	scene.settings.passthrough_available = true
	scene.settings.passthrough = true
	scene.live.page = "home"
	scene.live._rebuild()
	var success := true
	# Deliberately draw the dimmer late: foreground depth must still protect it.
	scene.background.material_override.render_priority = 2
	for show_menu in [true, false]:
		scene.set_menu_open(show_menu)
		var target: MeshInstance3D = scene.menu.menu_surface if show_menu else scene.cinema
		var center := Vector2i(scene.camera.unproject_position(target.global_position))
		scene.settings.darkness = 0.0
		scene._apply_background()
		var original: Image = await rendered_image()
		for darkness in [0.5, 1.0]:
			scene.settings.darkness = darkness
			scene._apply_background()
			var dimmed: Image = await rendered_image()
			# Include menu pixels outside the screen silhouette: the screen depth
			# must not accidentally protect all samples from a broken dimmer.
			var offsets: Array = [Vector2i.ZERO, Vector2i(-300, 0), Vector2i(0, -200), Vector2i(0, 200)] if show_menu else [Vector2i.ZERO, Vector2i(-80, -40), Vector2i(80, 40)]
			for offset in offsets:
				var point: Vector2i = center + offset
				var a := original.get_pixelv(point)
				var b := dimmed.get_pixelv(point)
				if absf(a.r - b.r) + absf(a.g - b.g) + absf(a.b - b.b) > 0.015:
					push_error("Room darkness changed foreground pixels: menu=%s darkness=%s" % [show_menu, darkness])
					success = false
			if original.get_pixel(100, 40).get_luminance() - dimmed.get_pixel(100, 40).get_luminance() < 0.1:
				push_error("Room darkness must still dim the synthetic room")
				success = false
			var output := ProjectSettings.globalize_path("res://../.tools/room-darkness-%s-%d.png" % ["menu" if show_menu else "screen", roundi(darkness * 100)])
			if dimmed.save_png(output) != OK:
				success = false
	scene.background.material_override.render_priority = Material.RENDER_PRIORITY_MIN
	scene.settings.passthrough = false
	scene.settings.passthrough_available = false
	scene._apply_background()
	room.queue_free()
	print("Room darkness foreground isolation: ", "PASS" if success else "FAIL")
	return success
