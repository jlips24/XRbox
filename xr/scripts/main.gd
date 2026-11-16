extends Node3D
## Scene composition and XR lifecycle. Host previews remain synthetic.
const ScreenSettings = preload("res://scripts/screen_settings.gd")

var settings = ScreenSettings.new()
var runtime: RefCounted
var origin: XROrigin3D
var camera: XRCamera3D
var cinema: MeshInstance3D
var menu: RefCounted
var controller_input: RefCounted
var video: RefCounted
var xr_running: bool = false
var xr_interface: XRInterface
var menu_open: bool = true
var anchor: Transform3D = Transform3D.IDENTITY
var menu_combo_armed: bool = true
var focused: bool = true
var initial_placement_pending: bool = false
var live: RefCounted

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
	_create_screen()
	menu = preload("res://scripts/menu_ui.gd").new(self, settings, _apply_screen, recenter, set_menu_open)
	controller_input = preload("res://scripts/controller_input.gd").new(runtime)
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
	# OpenXR failure is visible and still permits an explicitly labeled desktop preview.
	menu.status_label.text = "XRbox %s · core %s\n%s\n%s\nSynthetic screen only · Xbox media integration pending" % [
		ProjectSettings.get_setting("application/config/version"),
		runtime.call("core_version"), runtime.call("native_target"),
		"OpenXR active" if xr_running else "Desktop preview · no XR presentation verified"]
	recenter()
	initial_placement_pending = xr_running
	set_process(initial_placement_pending)
	set_menu_open(true)
	if runtime.call("streaming_supported") and not args.has("--test"):
		live = load("res://scripts/live_ui.gd").new(runtime, menu, video, settings, _apply_screen, recenter, set_menu_open, func(): get_tree().quit())
		if live.start():
			controller_input.start()
			set_process(true)
		else:
			live = null
			menu.status_label.text = "Android services failed to initialize; check app logs"

func _process(_delta: float) -> void:
	if live != null:
		controller_input.sample(focused)
		live.tick(_delta)
	_place_when_tracked(xr_interface != null and xr_interface.get_tracking_status() == XRInterface.XR_NORMAL_TRACKING)

func _place_when_tracked(tracking_valid: bool) -> void:
	# Startup can precede the first headset pose. Place at eye height once a
	# tracked pose exists; following head movement must not drag the screen.
	if initial_placement_pending and tracking_valid:
		recenter()
		initial_placement_pending = false
		set_process(live != null)

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
	var image := Image.create(640, 360, false, Image.FORMAT_RGB8)
	var colors := [Color.WHITE, Color.YELLOW, Color.CYAN, Color.GREEN, Color.MAGENTA, Color.RED, Color.BLUE, Color.BLACK]
	for x in range(640):
		for y in range(360):
			image.set_pixel(x, y, colors[x / 80] if y < 300 else Color(float(x) / 639.0, float(x) / 639.0, float(x) / 639.0))
	material.albedo_texture = ImageTexture.create_from_image(image)
	cinema.material_override = material
	add_child(cinema)

func recenter() -> void:
	# Keep the screen level: headset pitch/roll must not tilt the cinema plane.
	anchor = Transform3D(Basis(Vector3.UP, camera.global_rotation.y), camera.global_position)
	_apply_screen()

func _apply_screen() -> void:
	cinema.mesh.size = Vector2(settings.width, settings.width * 9.0 / 16.0)
	cinema.transform = anchor * Transform3D(Basis.IDENTITY, Vector3(0, 0, -settings.distance))
	menu.place(anchor)
	menu.metrics_label.text = "Screen %.1f m wide · %.1f m away" % [settings.width, settings.distance]

func set_menu_open(open: bool) -> void:
	menu_open = open
	if runtime != null: runtime.call("set_menu", open)
	menu.set_open(open)

func _input(event: InputEvent) -> void:
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
		set_menu_open(not menu_open)
	elif menu_open:
		menu.navigate(event)

func _xr_focus_lost() -> void:
	focused = false
	if runtime != null:
		runtime.call("set_focused", false)
		if live != null and live.state.get("active", false): runtime.call("disconnect_stream")

func _xr_focus_returned() -> void:
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
