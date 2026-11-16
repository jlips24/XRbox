extends Node3D
## Scene composition and XR lifecycle. Host previews remain synthetic.
const ScreenSettings = preload("res://scripts/screen_settings.gd")

var settings = ScreenSettings.new()
var runtime: RefCounted
var origin: XROrigin3D
var camera: XRCamera3D
var screen_frame: MeshInstance3D
var cinema: MeshInstance3D
var menu: RefCounted
var controller_input: RefCounted
var video: RefCounted
var xr_running: bool = false
var xr_session_focused: bool = false
var xr_interface: XRInterface
var menu_open: bool = true
var anchor: Transform3D = Transform3D.IDENTITY
var menu_combo_armed: bool = true
var focused: bool = true
var initial_placement_pending: bool = false
var live: RefCounted
var xr_menu: RefCounted
var background: MeshInstance3D

func _ready() -> void:
	_configure_input()
	# ClassDB avoids a parser failure that hides the actionable missing-native-library error.
	if not ClassDB.class_exists("XrboxRuntime"):
		push_error("XRbox native extension missing. Run make xr-build before launching.")
		get_tree().quit(1)
		return
	runtime = ClassDB.instantiate("XrboxRuntime")
	origin = XROrigin3D.new()
	add_child(origin)
	camera = XRCamera3D.new()
	origin.add_child(camera)
	camera.current = true
	_create_background()
	_create_screen()
	menu = preload("res://scripts/menu_ui.gd").new(self, settings, _apply_screen, recenter, set_menu_open)
	controller_input = preload("res://scripts/controller_input.gd").new(runtime)
	xr_menu = preload("res://scripts/xr_menu_input.gd").new(origin, _xr_navigate, _xr_accept, _xr_back, _xr_toggle)
	video = preload("res://scripts/video_surface.gd").new(runtime, cinema)
	var args := OS.get_cmdline_user_args()
	if not args.has("--desktop") and not args.has("--test"):
		xr_interface = XRServer.find_interface("OpenXR")
		if xr_interface != null and xr_interface.initialize():
			xr_running = true
			get_viewport().use_xr = true
			for signal_name in ["session_visible", "session_stopping"]:
				if xr_interface.has_signal(signal_name): xr_interface.connect(signal_name, _xr_focus_lost)
			if xr_interface.has_signal("session_focussed"): xr_interface.connect("session_focussed", _xr_focus_returned)
			if xr_interface.has_signal("pose_recentered"): xr_interface.connect("pose_recentered", _headset_recentered)
	settings.passthrough_available = xr_running and XRInterface.XR_ENV_BLEND_MODE_ALPHA_BLEND in xr_interface.get_supported_environment_blend_modes()
	# OpenXR failure is visible and still permits an explicitly labeled desktop preview.
	menu.status_label.text = "OpenXR active" if xr_running else "Synthetic preview · no account or console access"
	recenter()
	initial_placement_pending = xr_running
	if xr_running:
		cinema.hide()
		screen_frame.hide()
	set_process(true)
	set_menu_open(true)
	if runtime.call("streaming_supported") and not args.has("--test"):
		live = load("res://scripts/live_ui.gd").new(runtime, menu, video, settings, _apply_screen, recenter, set_menu_open, func(): get_tree().quit())
		if live.start():
			controller_input.start()
			set_process(true)
		else:
			live = null
			menu.status_label.text = "Android services failed to initialize; check app logs"

	if args.has("--ui-preview") or (live == null and not args.has("--test")):
		live = load("res://scripts/live_ui.gd").new(runtime, menu, video, settings, _apply_screen, recenter, set_menu_open, func(): get_tree().quit(), false)
		live.preview = true
		live.state = {"status": "Synthetic preview · no account or console access", "consoles": PackedStringArray(["Living room Xbox", "Office Xbox"]) if args.has("--ui-preview") else PackedStringArray()}
		live._rebuild()

func _process(_delta: float) -> void:
	if xr_menu != null:
		xr_menu.tick(_delta, focused and xr_running)
	if live != null:
		controller_input.sample(focused)
		live.input_status = controller_input.status
		live.tick(_delta)
	_place_when_tracked(xr_session_focused and _tracked_head_pose() != null)

func _place_when_tracked(tracking_valid: bool) -> void:
	# Startup can precede the first headset pose. Place at eye height once a
	# tracked pose exists; following head movement must not drag the screen.
	if initial_placement_pending and tracking_valid:
		if recenter():
			initial_placement_pending = false
			cinema.show()
			screen_frame.show()
			menu.set_open(menu_open)

func _configure_input() -> void:
	# Godot 4.7 desktop defaults omit A/B from accept/cancel. Add explicit
	# application-local mappings; keep menu navigation independent of gameplay swaps.
	for binding in [["ui_accept", JOY_BUTTON_A], ["ui_cancel", JOY_BUTTON_B]]:
		var event := InputEventJoypadButton.new()
		event.device = -1
		event.button_index = binding[1]
		if not InputMap.action_has_event(binding[0], event):
			InputMap.action_add_event(binding[0], event)

func _create_screen() -> void:
	cinema = MeshInstance3D.new()
	cinema.mesh = QuadMesh.new()
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	# Render the idle card once. Live playback replaces only the cinema material.
	var idle := SubViewport.new()
	idle.size = Vector2i(1280, 720)
	idle.render_target_update_mode = SubViewport.UPDATE_ONCE
	add_child(idle)
	var panel := PanelContainer.new()
	panel.size = Vector2(1280, 720)
	var style := StyleBoxFlat.new()
	style.bg_color = Color("111b25")
	style.border_color = Color("334a50")
	style.set_border_width_all(2)
	style.set_corner_radius_all(28)
	panel.add_theme_stylebox_override("panel", style)
	idle.add_child(panel)
	var center := CenterContainer.new()
	panel.add_child(center)
	var words := VBoxContainer.new()
	words.add_theme_constant_override("separation", 24)
	center.add_child(words)
	for line in [["X R B O X", 30, Color("b3edc5")], ["A space for your Xbox.", 54, Color("f2f5fa")], ["Your console appears here when you connect.", 26, Color("a2adbd")]]:
		var label := Label.new()
		label.text = line[0]
		label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		label.add_theme_font_size_override("font_size", line[1])
		label.add_theme_color_override("font_color", line[2])
		words.add_child(label)
	material.albedo_texture = idle.get_texture()
	cinema.material_override = material
	add_child(cinema)
	screen_frame = MeshInstance3D.new()
	screen_frame.mesh = QuadMesh.new()
	var frame_material := StandardMaterial3D.new()
	frame_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	frame_material.albedo_color = Color("293640")
	screen_frame.material_override = frame_material
	add_child(screen_frame)

func _headset_recentered() -> void:
	# Wait for the next valid pose rather than using the pre-recenter camera.
	initial_placement_pending = true

func _create_background() -> void:
	background = MeshInstance3D.new()
	var sphere := SphereMesh.new()
	sphere.radius = 50.0
	sphere.height = 100.0
	background.mesh = sphere
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.cull_mode = BaseMaterial3D.CULL_FRONT
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	# The room sphere is transparent and camera-centered. Distance sorting alone
	# can draw it over alpha-blended foregrounds despite its much larger radius.
	material.render_priority = Material.RENDER_PRIORITY_MIN
	material.albedo_color = Color.BLACK
	background.material_override = material
	camera.add_child(background)

func _apply_background() -> void:
	var enabled: bool = settings.passthrough and settings.passthrough_available
	if xr_running and xr_interface != null:
		xr_interface.environment_blend_mode = XRInterface.XR_ENV_BLEND_MODE_ALPHA_BLEND if enabled else XRInterface.XR_ENV_BLEND_MODE_OPAQUE
	get_viewport().transparent_bg = enabled
	background.visible = enabled and settings.darkness > 0.0
	background.material_override.albedo_color = Color(0, 0, 0, settings.darkness)

func _tracked_head_pose() -> XRPose:
	var tracker := XRServer.get_tracker("head") as XRPositionalTracker
	if tracker == null or not tracker.has_pose("default"):
		return null
	var pose := tracker.get_pose("default")
	if pose == null or not pose.has_tracking_data or pose.tracking_confidence == XRPose.XR_TRACKING_CONFIDENCE_NONE:
		return null
	return pose

func recenter() -> bool:
	var head: Transform3D = camera.global_transform
	if xr_running:
		var pose := _tracked_head_pose()
		if not xr_session_focused or pose == null:
			initial_placement_pending = true
			return false
		# Match XRCamera3D's adjusted tracking space, including origin and world scale.
		head = origin.global_transform * pose.get_adjusted_transform()
	return _place_from_head(head)

func _place_from_head(head: Transform3D) -> bool:
	# Project the actual gaze direction onto the floor; Euler yaw can be
	# ambiguous when the headset is tilted. Do not guess while looking vertically.
	var forward := -head.basis.z
	forward.y = 0.0
	if forward.length_squared() < 0.0001:
		return false
	forward = forward.normalized()
	anchor = Transform3D(Basis(forward.cross(Vector3.UP), Vector3.UP, -forward), head.origin)
	_apply_screen()
	return true

func _apply_screen() -> void:
	_apply_background()
	cinema.mesh.size = Vector2(settings.width, settings.width * 9.0 / 16.0)
	cinema.transform = anchor * Transform3D(Basis.IDENTITY, Vector3(settings.horizontal, settings.height, -settings.distance))
	screen_frame.mesh.size = cinema.mesh.size + Vector2(0.035, 0.035)
	screen_frame.transform = cinema.transform
	screen_frame.position += anchor.basis.z * -0.005
	menu.place(anchor)
	menu.refresh_values()
	menu.metrics_label.text = "Screen %.1f m wide · %.1f m away" % [settings.width, settings.distance]

func set_menu_open(open: bool) -> void:
	menu_open = open
	if runtime != null: runtime.call("set_menu", open)
	menu.set_open(open and not initial_placement_pending)

func _input(event: InputEvent) -> void:
	if focused and controller_input != null: controller_input.handle_event(event)
	if live != null:
		controller_input.sample(focused)
		if live.state.get("active", false) and not menu_open:
			if event is InputEventKey and event.is_action_pressed("ui_cancel"): set_menu_open(true)
			return
	if not focused:
		return
	if live == null and event is InputEventJoypadButton:
		var pad := event.device
		var chord := Input.is_joy_button_pressed(pad, JOY_BUTTON_BACK) and Input.is_joy_button_pressed(pad, JOY_BUTTON_START) and Input.is_joy_button_pressed(pad, JOY_BUTTON_LEFT_SHOULDER) and Input.is_joy_button_pressed(pad, JOY_BUTTON_RIGHT_SHOULDER)
		if chord and menu_combo_armed:
			menu_combo_armed = false
			set_menu_open(true)
		if not Input.is_joy_button_pressed(pad, JOY_BUTTON_BACK) and not Input.is_joy_button_pressed(pad, JOY_BUTTON_START):
			menu_combo_armed = true
	if event.is_action_pressed("ui_cancel"):
		_xr_back()
	elif menu_open:
		menu.navigate(event)

func _xr_focus_lost() -> void:
	xr_session_focused = false
	focused = false
	if runtime != null:
		runtime.call("set_focused", false)
		if live != null and live.state.get("active", false): runtime.call("disconnect_stream")

func _xr_focus_returned() -> void:
	xr_session_focused = true
	focused = true
	if runtime != null: runtime.call("set_focused", true)
	set_menu_open(true)

func _notification(what: int) -> void:
	if what == NOTIFICATION_APPLICATION_FOCUS_OUT or what == NOTIFICATION_APPLICATION_PAUSED:
		focused = false
		if runtime != null: runtime.call("set_focused", false)
		if live != null and live.state.get("active", false): runtime.call("disconnect_stream")
		menu_combo_armed = false
	elif what == NOTIFICATION_APPLICATION_FOCUS_IN or what == NOTIFICATION_APPLICATION_RESUMED:
		focused = true
		if runtime != null: runtime.call("set_focused", true)
		if menu != null:
			set_menu_open(true)

func _exit_tree() -> void:
	if controller_input != null: controller_input.dispose()
	if video != null: video.dispose()
	if runtime != null: runtime.call("shutdown")
	if xr_running and xr_interface != null:
		get_viewport().use_xr = false
		xr_interface.uninitialize()
	runtime = null

func _xr_navigate(direction: int) -> void:
	if focused and menu_open:
		menu.move(direction)

func _xr_accept() -> void:
	if not focused:
		return
	if menu_open:
		menu.accept()
	else:
		set_menu_open(true)

func _xr_back() -> void:
	if not focused:
		return
	if live != null and menu_open and live.page != "home":
		live.page = {"display": "settings", "placement": "display", "environment": "settings", "account": "settings", "diagnostics": "settings", "signout": "account"}.get(live.page, "home")
		live.key = ""
	else:
		set_menu_open(not menu_open)

func _xr_toggle() -> void:
	if focused:
		set_menu_open(not menu_open)
