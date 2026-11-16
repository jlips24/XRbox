extends RefCounted
## UI only: device-code display and console selection. No tokens or media bytes.
var runtime: RefCounted
var menu: RefCounted
var video: RefCounted
var settings: RefCounted
var apply_screen: Callable
var recenter: Callable
var set_menu_open: Callable
var quit: Callable
var column: VBoxContainer
var code_label: Label
var diagnostics_label: Label
var page: String = "home"
var state: Dictionary = {}
var remember: bool = true
var key: String = ""
var poll_time: float = 0.0
var preview: bool = false
var input_status: String = "No gamepad detected"
var console_page: int = 0
var code: String = ""
var uri: String = ""

func _init(native_runtime: RefCounted, menu_ui: RefCounted, video_surface: RefCounted, screen_settings: RefCounted, apply: Callable, center: Callable, open_menu: Callable, quit_app: Callable, load_preferences: bool = true) -> void:
	runtime = native_runtime
	menu = menu_ui
	video = video_surface
	settings = screen_settings
	apply_screen = apply
	recenter = center
	set_menu_open = open_menu
	quit = quit_app
	column = menu.column
	var config := ConfigFile.new()
	if load_preferences and config.load("user://preferences.cfg") == OK:
		remember = config.get_value("auth", "remember", true)

func start() -> bool:
	return runtime.call("start_client", remember)

func tick(delta: float) -> void:
	if not preview:
		runtime.call("update_video")
	poll_time += delta
	if poll_time < 0.1:
		return
	poll_time = 0.0
	if not preview:
		state = runtime.call("snapshot")
	if state.is_empty():
		return
	var active: bool = state.get("active", false)
	video.update(active)
	if state.get("gpu_error", false):
		menu.status_label.text = "Video presentation failed. Disconnect and retry."
	else:
		menu.status_label.text = state.get("status", "Ready")
	if diagnostics_label != null:
		diagnostics_label.text = _diagnostics()
	if active and menu.menu_surface.visible != state.get("menu", true):
		set_menu_open.call(state.get("menu", true))
	code = state.get("code", "")
	uri = state.get("uri", "")
	var next_key := str([page, console_page, active, state.get("busy", false), state.get("consoles", []), code, remember])
	if key != next_key:
		key = next_key
		_rebuild()

func _rebuild() -> void:
	menu.clear_actions()
	code_label = null
	diagnostics_label = null
	menu.page_label.text = {"home": "Your console", "display": "Screen", "placement": "Placement", "consoles": "Consoles", "signout": "Sign out", "diagnostics": "Diagnostics", "settings": "Settings", "environment": "Environment", "account": "Account"}.get(page, "Cinema")
	menu.title_label.text = {"home": "Ready to play" if state.get("active", false) else "Welcome home", "display": "Your floating screen", "placement": "Find your sweet spot", "settings": "Make it yours", "environment": "Set the atmosphere", "account": "Your account", "consoles": "Choose your Xbox", "signout": "Before you go", "diagnostics": "Connection details"}.get(page, "XRbox")
	if page == "diagnostics":
		diagnostics_label = menu.add_description(_diagnostics())
		menu.add_button(column, "Back", func(): page = "home")
	elif page == "settings":
		menu.add_section("Make yourself comfortable", "Screen and room adjustments last for this session.")
		menu.add_button(column, "Screen size & placement", func(): page = "display")
		menu.add_button(column, "Environment & passthrough", func(): page = "environment")
		menu.add_button(column, "Account & sign-in", func(): page = "account")
		menu.add_button(column, "Connection diagnostics", func(): page = "diagnostics")
		menu.add_button(column, "Back", func(): page = "home")
	elif page == "display":
		menu.add_screen_controls(settings, apply_screen)
		menu.add_button(column, "Position & orientation", func(): page = "placement")
		menu.add_button(column, "Back to settings", func(): page = "settings")
	elif page == "placement":
		menu.add_placement_controls(settings, apply_screen, recenter)
		menu.add_section("Picture orientation")
		var flip: Button = menu.add_button(column, "Vertical flip: " + ("On" if video.flip_video else "Off"), func(): video.toggle_flip(); key = "")
		flip.tooltip_text = "Use only if the console picture appears upside down"
		menu.add_button(column, "Back to settings", func(): page = "settings")
	elif page == "environment":
		menu.add_section("Your surroundings", "Keep your room visible, or settle into a dark cinema.")
		menu.add_background_controls(settings, apply_screen)
		menu.add_button(column, "Back to settings", func(): page = "settings")
	elif page == "account":
		menu.add_section("Microsoft account", "Remembered sign-in uses secure device storage. Choose Off for session-only sign-in.")
		menu.add_button(column, "Remember sign-in: " + ("On" if remember else "Off"), _toggle_remember)
		menu.add_button(column, "Sign in to Microsoft", func(): runtime.call("sign_in", remember))
		menu.add_button(column, "Refresh consoles", func(): runtime.call("discover", remember))
		menu.add_button(column, "Sign out", func(): page = "signout")
		menu.add_button(column, "Back to settings", func(): page = "settings")
	elif not code.is_empty():
		menu.page_label.text = "Sign in"
		menu.title_label.text = "Connect your account"
		menu.add_description("On your phone or computer, open %s and enter this code. Keep this screen open while you sign in." % (uri if not uri.is_empty() else "the Microsoft verification page"))
		code_label = Label.new()
		code_label.text = code
		code_label.add_theme_font_override("font", load("res://fonts/Hack-Regular.ttf"))
		code_label.add_theme_font_size_override("font_size", 48)
		column.add_child(code_label)
		menu.add_button(column, "Copy sign-in code", func(): DisplayServer.clipboard_set(code))
		menu.add_button(column, "Open Microsoft sign-in", _open_auth)
		menu.add_button(column, "Cancel sign-in", func(): runtime.call("disconnect_stream"))
	elif page == "signout":
		menu.add_description("Sign out of Microsoft and remove remembered credentials from this device?")
		menu.add_button(column, "Confirm sign out", func(): runtime.call("sign_out"); page = "home")
		menu.add_button(column, "Cancel", func(): page = "home")
	elif state.get("busy", false):
		menu.add_button(column, "Cancel connection/sign-in", func(): runtime.call("disconnect_stream"))
	elif state.get("active", false):
		menu.add_section("Now playing", "Your console keeps running while this menu is open.")
		menu.add_button(column, "Resume play", func(): set_menu_open.call(false))
		menu.add_button(column, "Settings", func(): page = "settings")
		menu.add_button(column, "Disconnect", func(): runtime.call("disconnect_stream"))
	elif page == "consoles":
		var consoles: PackedStringArray = state.get("consoles", PackedStringArray())
		var offset: int = console_page * 4
		for index in range(offset, mini(offset + 4, consoles.size())):
			var selected := index
			menu.add_button(column, "Play " + consoles[index], func():
				if runtime.call("connect_console", selected): set_menu_open.call(false))
		if offset + 4 < consoles.size():
			menu.add_button(column, "Next consoles", func(): console_page += 1)
		if console_page > 0:
			menu.add_button(column, "Previous consoles", func(): console_page -= 1)
		menu.add_button(column, "Back", func(): page = "home")
	else:
		menu.add_section("Console Remote Play", "Pick up where you left off, on a screen that fits your space.")
		if not state.get("consoles", PackedStringArray()).is_empty():
			menu.add_button(column, "Choose Xbox console", func(): page = "consoles"; console_page = 0)
		else:
			menu.add_button(column, "Sign in to Microsoft", func(): runtime.call("sign_in", remember))
			menu.add_button(column, "Remember sign-in: " + ("On" if remember else "Off"), _toggle_remember)
		menu.add_button(column, "Settings", func(): page = "settings")
	if page == "home":
		menu.add_button(column, "Quit", func(): quit.call())
	menu.add_navigation(page, func(destination: String): page = destination)
	menu.select_focus(0)

func _toggle_remember() -> void:
	remember = not remember
	var config := ConfigFile.new()
	config.set_value("auth", "remember", remember)
	if config.save("user://preferences.cfg") != OK:
		menu.status_label.text = "Could not save sign-in preference"

func _open_auth() -> void:
	# Only the supported Microsoft device-code browser flow is exposed.
	if uri.begins_with("https://microsoft.com/") or uri.begins_with("https://www.microsoft.com/"):
		OS.shell_open(uri)

func _diagnostics() -> String:
	return "Gamepad: %s\nInput ticks: sampled %d · routed %d · focus %s · menu %s\nStream controller: %s\nDecoded source: %s\nAssembled %d · Decoded %d · Decoder errors %d\nGPU imports %d · Audio %s · Audio drops %d\nRoute: %s\nCodec: %s · status %d\nThese counters do not establish visible playback or end-to-end latency." % [
		input_status, state.get("sampled_input_ticks", 0), state.get("routed_input_ticks", 0), str(state.get("input_focused", false)), str(state.get("menu", true)), state.get("stream_controller", "waiting"), _source_size(), state.get("assembled", 0), state.get("decoded", 0), state.get("decoder_errors", 0),
		state.get("gpu_imports", 0), "playing" if state.get("audio", false) else "waiting", state.get("audio_drops", 0),
		state.get("route", "unknown"), state.get("codec", "unknown"), state.get("codec_status", 0)]

func _source_size() -> String:
	var width: int = state.get("video_width", 0)
	var height: int = state.get("video_height", 0)
	return "%d × %d" % [width, height] if width > 0 and height > 0 else "waiting for video"
