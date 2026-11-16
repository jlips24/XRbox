extends SceneTree

var failures: int = 0

func verify(condition: bool, message: String) -> void:
	if not condition:
		failures += 1
		push_error(message)

func _initialize() -> void:
	call_deferred("run_tests")

func press(action: StringName, scene: Node) -> void:
	var event := InputEventAction.new()
	event.action = action
	event.pressed = true
	scene._input(event)

func run_tests() -> void:
	var packed: PackedScene = load("res://scenes/main.tscn")
	var scene = packed.instantiate()
	root.add_child(scene)
	await process_frame
	_test_navigation(scene)
	_test_meta_input(scene)
	_test_placement(scene)
	_test_video_surface(scene)
	_test_focus(scene)
	await _test_live_pages(scene)
	await _test_shutdown(scene, packed)

func _test_navigation(scene: Node) -> void:
	verify(scene.runtime != null, "Native extension must load")
	verify(scene.runtime.call("core_version") == "0.1.10", "Native core version mismatch")
	verify(not String(scene.runtime.call("native_target")).is_empty(), "Target diagnostic missing")
	verify(scene.menu.menu_panel.get_combined_minimum_size().y <= scene.menu.menu_viewport.size.y, "Menu must fit in the XR surface")
	verify(not scene.xr_running, "Headless checks must not initialize OpenXR")
	verify(scene.menu_open and scene.menu.focus_index == 0, "Initial menu focus")
	press("ui_down", scene)
	verify(scene.menu.focus_index == 1, "Controller/keyboard down must advance focus")
	press("ui_accept", scene)
	verify(is_equal_approx(scene.settings.width, 2.6), "Accept must activate focused setting")
	press("ui_cancel", scene)
	verify(not scene.menu_open and not scene.menu.menu_surface.visible, "Back must hide menu")
	verify(scene.menu.menu_viewport.render_target_update_mode == SubViewport.UPDATE_DISABLED, "Hidden menu must not redraw")
	press("ui_down", scene)
	verify(scene.menu.focus_index == 1, "Hidden menu must not consume navigation")
	press("ui_cancel", scene)
	verify(scene.menu_open and scene.menu.focus_index == 0, "Menu reopening restores safe focus")
	press("ui_up", scene)
	verify(scene.menu.focus_index == scene.menu.buttons.size() - 1, "Focus wraps predictably")
	scene.menu.select_focus(0)
	var down := InputEventJoypadButton.new()
	down.button_index = JOY_BUTTON_DPAD_DOWN
	down.pressed = true
	scene._input(down)
	verify(scene.menu.focus_index == 1, "Physical D-pad mapping")
	var accept := InputEventJoypadButton.new()
	accept.button_index = JOY_BUTTON_A
	accept.pressed = true
	var old_width: float = scene.settings.width
	scene._input(accept)
	verify(scene.settings.width > old_width, "Physical A mapping")
	var back := InputEventJoypadButton.new()
	back.button_index = JOY_BUTTON_B
	back.pressed = true
	scene._input(back)
	verify(not scene.menu_open, "Physical B mapping")
	var escape := InputEventKey.new()
	escape.keycode = KEY_ESCAPE
	escape.pressed = true
	scene._input(escape)
	verify(scene.menu_open, "Keyboard Escape mapping")
	var guide := InputEventJoypadButton.new()
	guide.button_index = JOY_BUTTON_GUIDE
	guide.pressed = true
	scene._input(guide)
	verify(scene.menu_open and scene.menu.focus_index == 0, "Physical Guide stays reserved to host")

func _test_placement(scene: Node) -> void:
	scene.settings.adjust_width(100.0)
	scene.settings.adjust_distance(-100.0)
	verify(scene.settings.width == 5.0 and scene.settings.distance == 1.0, "Geometry bounds")
	scene.settings.reset()
	scene.camera.position = Vector3(1, 1.5, 0)
	scene.camera.rotation = Vector3(0.3, 0.2, 0.1)
	scene.recenter()
	verify(is_equal_approx(scene.cinema.position.y, 1.5), "Recenter follows eye height")
	verify(is_zero_approx(scene.cinema.rotation.x) and is_zero_approx(scene.cinema.rotation.z), "Screen stays level")
	verify(is_equal_approx(scene.cinema.mesh.size.y, 1.35), "Screen preserves 16:9")
	scene.initial_placement_pending = true
	var previous_anchor: Transform3D = scene.anchor
	scene.camera.position = Vector3(0, 1.7, 0)
	scene._place_when_tracked(false)
	verify(scene.initial_placement_pending and scene.anchor == previous_anchor, "Do not place from an untracked startup pose")
	scene._place_when_tracked(true)
	verify(not scene.initial_placement_pending and is_equal_approx(scene.cinema.position.y, 1.7), "Initial tracked pose sets eye-height placement")
	scene.camera.position.y = 1.9
	scene._place_when_tracked(true)
	verify(is_equal_approx(scene.cinema.position.y, 1.7), "Subsequent head motion must keep screen world-locked")

func _test_focus(scene: Node) -> void:
	scene._notification(Node.NOTIFICATION_APPLICATION_FOCUS_OUT)
	var before: int = scene.menu.focus_index
	press("ui_down", scene)
	verify(scene.menu.focus_index == before, "Unfocused input must be ignored")
	scene._notification(Node.NOTIFICATION_APPLICATION_FOCUS_IN)
	verify(scene.menu_open and scene.menu.focus_index == 0, "Focus recovery returns to menu")
	verify(not scene.runtime.call("start_client", false), "Host test must not start Android services or read credentials")
	verify(not scene.runtime.call("connect_console", -1), "Invalid selection is rejected without a session")

func _test_live_pages(scene: Node) -> void:
	var live_ui = load("res://scripts/live_ui.gd").new(scene.runtime, scene.menu, scene.video, scene.settings, scene._apply_screen, scene.recenter, scene.set_menu_open, func(): quit(), false)
	live_ui.state = {"consoles": PackedStringArray(["Test 1", "Test 2", "Test 3", "Test 4", "Test 5"])}
	scene.menu.status_label.text = "XRbox 0.1.10-xr.2 · " + "Network operation failed; check console and retry. ".repeat(3) + "\nAssembled 100 · Decoded 90 · GPU imports 90\nAudio playing · route unknown\nCodec test · status 1"
	for page in ["home", "consoles", "display", "signout", "diagnostics"]:
		live_ui.page = page
		live_ui._rebuild()
		await process_frame
		verify(scene.menu.buttons.size() > 0 and scene.menu.focus_index == 0, "Live page receives controller focus")
		verify(scene.menu.menu_panel.get_combined_minimum_size().y <= scene.menu.menu_viewport.size.y, "Live menu must fit its XR surface")
		verify(scene.menu.menu_panel.get_combined_minimum_size().x <= scene.menu.menu_viewport.size.x, "Long diagnostics must wrap inside menu width")
	live_ui.page = "home"
	live_ui._rebuild()
	await process_frame
	scene.menu.select_focus(scene.menu.buttons.size() - 1)
	await process_frame
	var footer_position: Vector2 = scene.menu.footer.position
	scene.menu.scroll.scroll_vertical = 17
	var idle_scroll: int = scene.menu.scroll.scroll_vertical
	scene.menu.select_focus(scene.menu.focus_index)
	await process_frame
	verify(scene.menu.scroll.scroll_vertical == idle_scroll, "Idle focus must not recenter a scrolled page")
	verify(scene.menu.footer.position == footer_position, "Action scrolling must not move the version footer")
	live_ui.page = "consoles"
	live_ui.console_page = 1
	live_ui._rebuild()
	verify(scene.menu.buttons[0].text == "Play Test 5", "Console pagination retains discovered selection order")
	live_ui.page = "home"
	live_ui.code = "0O1I-TEST"
	live_ui._rebuild()
	verify(live_ui.code_label.text == "0O1I-TEST", "Device code displayed without modification")
	verify(live_ui.code_label.get_theme_font("font") != null, "Device code has a dedicated monospaced font")
	scene.live = live_ui
	live_ui.state = {"active": true}
	scene.set_menu_open(false)
	press("ui_cancel", scene)
	verify(not scene.menu_open, "Gameplay B must stay in gameplay instead of opening local UI")
	scene.live = null

func _test_shutdown(scene: Node, packed: PackedScene) -> void:
	scene.runtime.call("shutdown")
	scene.runtime.call("shutdown")
	verify(scene.runtime.call("snapshot").is_empty(), "Bridge shutdown is idempotent")
	scene.queue_free()
	await process_frame
	# A second scene validates extension object teardown/recreation without auth or devices.
	var second = packed.instantiate()
	root.add_child(second)
	await process_frame
	verify(second.runtime != null, "Native object recreation")
	second.queue_free()
	await process_frame
	print("XRbox Godot smoke: %s" % ("PASS" if failures == 0 else "FAIL (%d)" % failures))
	quit(0 if failures == 0 else 1)

class FakeVideoRuntime extends RefCounted:
	var texture: Texture2D
	var clears: int = 0
	func _init() -> void:
		texture = ImageTexture.create_from_image(Image.create(2, 2, false, Image.FORMAT_RGB8))
	func video_texture() -> Texture2D:
		return texture
	func clear_video() -> void:
		clears += 1

func _test_video_surface(scene: Node) -> void:
	var native := FakeVideoRuntime.new()
	var original: Material = scene.cinema.material_override
	var surface = load("res://scripts/video_surface.gd").new(native, scene.cinema)
	surface.update(true)
	verify(surface.video_assigned and scene.cinema.material_override is ShaderMaterial, "Active video uses its shader material")
	var assigned: Material = scene.cinema.material_override
	surface.update(true)
	verify(scene.cinema.material_override == assigned, "Repeated active snapshots retain the material")
	surface.toggle_flip()
	verify(scene.cinema.material_override.get_shader_parameter("flip"), "Video flip updates the assigned material")
	surface.update(false)
	verify(scene.cinema.material_override == original and native.clears == 1, "Disconnect restores the synthetic material and clears native video once")
	surface.dispose()
	verify(native.clears == 1, "Repeated inactive cleanup is idempotent")
	surface.update(true)
	surface.dispose()
	verify(scene.cinema.material_override == original and native.clears == 2, "Scene teardown releases an active video material")

func _test_meta_input(scene: Node) -> void:
	var moves: Array[int] = []
	var actions: Array[String] = []
	var input = load("res://scripts/xr_menu_input.gd").new(scene.origin,
		func(value: int): moves.append(value), func(): actions.append("select"),
		func(): actions.append("back"), func(): actions.append("menu"))
	input.sample(0.0, 0.0, true, false, false, true)
	verify(actions.is_empty(), "Held Meta select on startup must not activate anything")
	input.sample(0.0, 0.0, false, false, false, true)
	input.sample(0.0, -0.8, false, false, false, true)
	input.sample(0.2, -0.8, false, false, false, true)
	verify(moves == [1], "Meta stick moves once before repeat delay")
	input.sample(0.21, -0.8, false, false, false, true)
	verify(moves == [1, 1], "Meta stick repeats after delay")
	input.sample(0.0, 0.0, true, false, false, true)
	input.sample(0.0, 0.0, true, false, false, true)
	verify(actions == ["select"], "A/X activates exactly once per press")
	input.sample(0.0, 0.0, false, true, true, true)
	verify(actions == ["select", "menu"], "Menu click wins simultaneous menu/back")
	input.sample(0.0, 0.0, false, false, false, false)
	input.sample(0.0, 0.0, false, true, false, true)
	verify(actions.size() == 2, "Focus recovery waits for Meta buttons to be released")
	input.sample(0.0, 0.0, false, false, false, true)
	input.sample(0.0, 0.0, false, true, false, true)
	verify(actions.back() == "back", "B/Y returns after neutral rearming")
	var map: OpenXRActionMap = load("res://openxr_action_map.tres")
	verify(map.get_action_sets().size() == 1, "XR menu uses one explicit action set")
	for profile in map.get_interaction_profiles():
		for binding in profile.bindings:
			verify(not binding.binding_path.contains("/menu/") and not binding.binding_path.contains("/system/"), "System-reserved buttons are never bound")
	verify(not scene.menu.scroll.is_ancestor_of(scene.menu.footer), "Version footer stays outside scrolling actions")
