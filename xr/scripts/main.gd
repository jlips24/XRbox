extends Node3D
## Linux-first XR bring-up scene. Synthetic pixels only, never Xbox media.
const ScreenSettings = preload("res://scripts/screen_settings.gd")

var settings = ScreenSettings.new()
var runtime: RefCounted
var origin: XROrigin3D
var camera: XRCamera3D
var cinema: MeshInstance3D
var menu_surface: MeshInstance3D
var menu_viewport: SubViewport
var menu_panel: Control
var status_label: Label
var metrics_label: Label
var buttons: Array[Button] = []
var focus_index: int = 0
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
		push_error("XRBox native extension missing. Run make xr-build before launching.")
		get_tree().quit(1)
		return
	runtime = ClassDB.instantiate("XrboxRuntime")
	origin = XROrigin3D.new()
	add_child(origin)
	camera = XRCamera3D.new()
	origin.add_child(camera)
	camera.current = true
	_create_screen()
	_create_menu()
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
	status_label.text = "XRBox %s · core %s\n%s\n%s\nSynthetic screen only · Xbox media integration pending" % [
		ProjectSettings.get_setting("application/config/version"),
		runtime.call("core_version"), runtime.call("native_target"),
		"OpenXR active" if xr_running else "Desktop preview · no XR presentation verified"]
	recenter()
	initial_placement_pending = xr_running
	set_process(initial_placement_pending)
	set_menu_open(true)
	if runtime.call("streaming_supported") and not args.has("--test"):
		live = load("res://scripts/live_ui.gd").new(self)
		if live.start():
			set_process(true)
		else:
			live = null
			status_label.text = "Android services failed to initialize; check app logs"

func _process(_delta: float) -> void:
	if live != null: live.tick(_delta)
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

func _create_menu() -> void:
	menu_viewport = SubViewport.new()
	menu_viewport.size = Vector2i(1024, 1024)
	menu_viewport.transparent_bg = false
	menu_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	add_child(menu_viewport)
	menu_panel = PanelContainer.new()
	menu_panel.size = Vector2(1024, 1024)
	menu_viewport.add_child(menu_panel)
	var margin := MarginContainer.new()
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 40)
	menu_panel.add_child(margin)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 12)
	margin.add_child(column)
	var heading := Label.new()
	heading.text = "XRBox · XR foundation"
	heading.add_theme_font_size_override("font_size", 34)
	column.add_child(heading)
	status_label = Label.new()
	status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status_label.add_theme_font_size_override("font_size", 22)
	column.add_child(status_label)
	metrics_label = Label.new()
	metrics_label.add_theme_font_size_override("font_size", 24)
	column.add_child(metrics_label)
	_add_button(column, "View test screen / Back", func(): set_menu_open(false))
	_add_button(column, "Screen wider", func(): settings.adjust_width(0.2); _apply_screen())
	_add_button(column, "Screen narrower", func(): settings.adjust_width(-0.2); _apply_screen())
	_add_button(column, "Screen farther", func(): settings.adjust_distance(0.2); _apply_screen())
	_add_button(column, "Screen nearer", func(): settings.adjust_distance(-0.2); _apply_screen())
	_add_button(column, "Recenter screen and menu", recenter)
	_add_button(column, "Reset screen", func(): settings.reset(); recenter())
	_add_button(column, "Quit", func(): get_tree().quit())
	var help := Label.new()
	help.text = "D-pad / arrows: navigate · A / Enter: select\nB / Escape: back · Select + Start + LB + RB: menu\nSession-only screen settings · no sign-in or network access"
	help.add_theme_font_size_override("font_size", 20)
	column.add_child(help)
	menu_surface = MeshInstance3D.new()
	var quad := QuadMesh.new()
	quad.size = Vector2(1.5, 1.5)
	menu_surface.mesh = quad
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_texture = menu_viewport.get_texture()
	menu_surface.material_override = material
	add_child(menu_surface)

func _add_button(parent: Node, text: String, action: Callable) -> void:
	var button := Button.new()
	button.text = text
	button.clip_text = true
	button.custom_minimum_size.y = 60
	button.add_theme_font_size_override("font_size", 26)
	button.pressed.connect(action)
	parent.add_child(button)
	buttons.append(button)

func recenter() -> void:
	# Keep the screen level: headset pitch/roll must not tilt the cinema plane.
	anchor = Transform3D(Basis(Vector3.UP, camera.global_rotation.y), camera.global_position)
	_apply_screen()

func _apply_screen() -> void:
	cinema.mesh.size = Vector2(settings.width, settings.width * 9.0 / 16.0)
	cinema.transform = anchor * Transform3D(Basis.IDENTITY, Vector3(0, 0, -settings.distance))
	menu_surface.transform = anchor * Transform3D(Basis.IDENTITY, Vector3(0, 0, -1.4))
	metrics_label.text = "Screen %.1f m wide · %.1f m away" % [settings.width, settings.distance]

func set_menu_open(open: bool) -> void:
	menu_open = open
	if runtime != null: runtime.call("set_menu", open)
	menu_surface.visible = open
	# Freeze the menu texture while hidden; video/XR submissions remain independent.
	menu_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS if open else SubViewport.UPDATE_DISABLED
	if open:
		_select_focus(0)

func _select_focus(index: int) -> void:
	focus_index = wrapi(index, 0, buttons.size())
	buttons[focus_index].grab_focus()

func _input(event: InputEvent) -> void:
	if live != null:
		live._sample_pad()
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
		if event.is_action_pressed("ui_down"):
			_select_focus(focus_index + 1)
		elif event.is_action_pressed("ui_up"):
			_select_focus(focus_index - 1)
		elif event.is_action_pressed("ui_accept"):
			buttons[focus_index].pressed.emit()

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
		if menu_surface != null:
			set_menu_open(true)

func _exit_tree() -> void:
	if live != null: live.dispose()
	if runtime != null: runtime.call("shutdown")
	if xr_running and xr_interface != null:
		get_viewport().use_xr = false
		xr_interface.uninitialize()
	runtime = null
