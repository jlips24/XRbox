extends RefCounted
## UI only: device-code display and console selection. No tokens or media bytes.
var scene: Node
var column: VBoxContainer
var code_label: Label
var page: String = "home"
var state: Dictionary = {}
var remember: bool = true
var key: String = ""
var poll_time: float = 0.0
var pad_id: int = -1
var video_assigned: bool = false
var original_material: Material
var flip_video: bool = false
var console_page: int = 0
var code: String = ""
var uri: String = ""

func _init(owner: Node, load_preferences: bool = true) -> void:
	scene = owner
	column = scene.status_label.get_parent()
	original_material = scene.cinema.material_override
	var config := ConfigFile.new()
	if load_preferences and config.load("user://preferences.cfg") == OK:
		remember = config.get_value("auth", "remember", true)

func start() -> bool:
	var started: bool = scene.runtime.call("start_client", remember)
	if started: Input.joy_connection_changed.connect(_joy_changed)
	return started

func dispose() -> void:
	if Input.joy_connection_changed.is_connected(_joy_changed): Input.joy_connection_changed.disconnect(_joy_changed)
	scene = null

func _joy_changed(device: int, connected: bool) -> void:
	if scene != null and device == pad_id and not connected:
		pad_id = -1
		scene.runtime.call("pad_sample", false, PackedInt32Array(), Vector2.ZERO, Vector2.ZERO, Vector2.ZERO)

func tick(delta: float) -> void:
	_sample_pad()
	scene.runtime.call("update_video")
	poll_time += delta
	if poll_time < 0.1:
		return
	poll_time = 0.0
	state = scene.runtime.call("snapshot")
	if state.is_empty():
		return
	var active: bool = state.get("active", false)
	if active and not video_assigned:
		var texture: Texture2D = scene.runtime.call("video_texture")
		if texture != null:
			var shader := Shader.new()
			shader.code = "shader_type spatial; render_mode unshaded, cull_disabled; uniform samplerExternalOES video : source_color; uniform bool flip = false; void fragment(){vec2 uv=UV; if(flip){uv.y=1.0-uv.y;} ALBEDO=texture(video,uv).rgb;}"
			var material := ShaderMaterial.new()
			material.shader = shader
			material.set_shader_parameter("video", texture)
			material.set_shader_parameter("flip", flip_video)
			scene.cinema.material_override = material
			video_assigned = true
	if not active and video_assigned:
		scene.cinema.material_override = original_material
		scene.runtime.call("clear_video")
		video_assigned = false
	if state.get("gpu_error", false):
		scene.status_label.text = "Android GPU image import failed. Disconnect and retry."
	else:
		scene.status_label.text = "XRBox %s · %s\nAssembled %d · Decoded %d · Errors %d · GPU imports %d\nAudio %s · drops %d · route %s\nCodec %s · status %d" % [
			ProjectSettings.get_setting("application/config/version"), state.get("status", ""), state.get("assembled", 0), state.get("decoded", 0), state.get("decoder_errors", 0),
			state.get("gpu_imports", 0), "playing" if state.get("audio", false) else "waiting", state.get("audio_drops", 0), state.get("route", "unknown"), state.get("codec", "unknown"), state.get("codec_status", 0)]
	if active and scene.menu_open != state.get("menu", true):
		scene.set_menu_open(state.get("menu", true))
	code = state.get("code", "")
	uri = state.get("uri", "")
	var next_key := str([page, console_page, active, state.get("busy", false), state.get("consoles", []), code, remember])
	if key != next_key:
		key = next_key
		_rebuild()

func _sample_pad() -> void:
	var pads := Input.get_connected_joypads()
	if pads.is_empty() or not scene.focused:
		pad_id = -1
		scene.runtime.call("pad_sample", false, PackedInt32Array(), Vector2.ZERO, Vector2.ZERO, Vector2.ZERO)
		return
	var pad: int = pads[0]
	if pad != pad_id:
		scene.runtime.call("pad_sample", false, PackedInt32Array(), Vector2.ZERO, Vector2.ZERO, Vector2.ZERO)
		pad_id = pad
	# Native bridge maps Godot button IDs to upstream protocol bits.
	var pressed := PackedInt32Array()
	for button in range(JOY_BUTTON_MAX):
		if Input.is_joy_button_pressed(pad, button): pressed.append(button)

	scene.runtime.call("pad_sample", true, pressed,
		Vector2(Input.get_joy_axis(pad, JOY_AXIS_LEFT_X), -Input.get_joy_axis(pad, JOY_AXIS_LEFT_Y)),
		Vector2(Input.get_joy_axis(pad, JOY_AXIS_RIGHT_X), -Input.get_joy_axis(pad, JOY_AXIS_RIGHT_Y)),
		Vector2(Input.get_joy_axis(pad, JOY_AXIS_TRIGGER_LEFT), Input.get_joy_axis(pad, JOY_AXIS_TRIGGER_RIGHT)))

func _rebuild() -> void:
	for child in column.get_children():
		if child == scene.status_label or child == scene.metrics_label:
			continue
		column.remove_child(child)
		child.queue_free()
	scene.buttons.clear()
	code_label = null
	if page == "display":
		scene._add_button(column, "Screen wider", func(): scene.settings.adjust_width(0.2); scene._apply_screen())
		scene._add_button(column, "Screen narrower", func(): scene.settings.adjust_width(-0.2); scene._apply_screen())
		scene._add_button(column, "Screen farther", func(): scene.settings.adjust_distance(0.2); scene._apply_screen())
		scene._add_button(column, "Screen nearer", func(): scene.settings.adjust_distance(-0.2); scene._apply_screen())
		scene._add_button(column, "Recenter", scene.recenter)
		scene._add_button(column, "Flip video vertically", func():
			flip_video = not flip_video
			if video_assigned: scene.cinema.material_override.set_shader_parameter("flip", flip_video))
		scene._add_button(column, "Back", func(): page = "home")
	elif not code.is_empty():
		code_label = Label.new()
		code_label.text = code
		code_label.add_theme_font_override("font", load("res://fonts/Hack-Regular.ttf"))
		code_label.add_theme_font_size_override("font_size", 48)
		column.add_child(code_label)
		scene._add_button(column, "Copy sign-in code", func(): DisplayServer.clipboard_set(code))
		scene._add_button(column, "Open Microsoft sign-in", _open_auth)
		scene._add_button(column, "Cancel sign-in", func(): scene.runtime.call("disconnect_stream"))
	elif page == "signout":
		scene._add_button(column, "Confirm sign out", func(): scene.runtime.call("sign_out"); page = "home")
		scene._add_button(column, "Cancel", func(): page = "home")
	elif state.get("busy", false):
		scene._add_button(column, "Cancel connection/sign-in", func(): scene.runtime.call("disconnect_stream"))
	elif state.get("active", false):
		scene._add_button(column, "Resume", func(): scene.set_menu_open(false))
		scene._add_button(column, "Screen settings", func(): page = "display")
		scene._add_button(column, "Disconnect", func(): scene.runtime.call("disconnect_stream"))
	elif page == "consoles":
		var consoles: PackedStringArray = state.get("consoles", PackedStringArray())
		var offset: int = console_page * 4
		for index in range(offset, mini(offset + 4, consoles.size())):
			var selected := index
			scene._add_button(column, "Play " + consoles[index], func():
				if scene.runtime.call("connect_console", selected): scene.set_menu_open(false))
		if offset + 4 < consoles.size():
			scene._add_button(column, "Next consoles", func(): console_page += 1)
		if console_page > 0:
			scene._add_button(column, "Previous consoles", func(): console_page -= 1)
		scene._add_button(column, "Back", func(): page = "home")
	else:
		if not state.get("consoles", PackedStringArray()).is_empty():
			scene._add_button(column, "Choose Xbox console", func(): page = "consoles"; console_page = 0)
		scene._add_button(column, "Sign in to Microsoft", func(): scene.runtime.call("sign_in", remember))
		scene._add_button(column, "Refresh consoles", func(): scene.runtime.call("discover", remember))
		scene._add_button(column, "Remember sign-in: " + ("On" if remember else "Off"), _toggle_remember)
		scene._add_button(column, "Screen settings", func(): page = "display")
		scene._add_button(column, "Sign out", func(): page = "signout")
	scene._add_button(column, "Quit", func(): scene.get_tree().quit())
	scene._select_focus(0)

func _toggle_remember() -> void:
	remember = not remember
	var config := ConfigFile.new()
	config.set_value("auth", "remember", remember)
	if config.save("user://preferences.cfg") != OK:
		scene.status_label.text = "Could not save sign-in preference"

func _open_auth() -> void:
	# Only the supported Microsoft device-code browser flow is exposed.
	if uri.begins_with("https://microsoft.com/") or uri.begins_with("https://www.microsoft.com/"):
		OS.shell_open(uri)
