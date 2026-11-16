extends RefCounted
var runtime: RefCounted
var pad_id: int = -1
var status: String = "No gamepad detected"

func _init(native_runtime: RefCounted) -> void:
	runtime = native_runtime

func start() -> void:
	Input.joy_connection_changed.connect(_joy_changed)

func dispose() -> void:
	if Input.joy_connection_changed.is_connected(_joy_changed):
		Input.joy_connection_changed.disconnect(_joy_changed)
	runtime = null

func _joy_changed(device: int, connected: bool) -> void:
	if runtime != null and device == pad_id and not connected:
		pad_id = -1
		status = "No gamepad detected / input unfocused"
		runtime.call("pad_sample", false, PackedInt32Array(), Vector2.ZERO, Vector2.ZERO, Vector2.ZERO)

func handle_event(event: InputEvent) -> void:
	# Android may enumerate multiple devices. Follow the physical pad being used,
	# rather than permanently sampling the first enumerated device.
	if (event is InputEventJoypadButton and event.pressed and event.button_index != JOY_BUTTON_GUIDE) or (event is InputEventJoypadMotion and absf(event.axis_value) > 0.5):
		if event.device != pad_id:
			runtime.call("pad_sample", false, PackedInt32Array(), Vector2.ZERO, Vector2.ZERO, Vector2.ZERO)
			pad_id = event.device

func sample(focused: bool) -> void:
	var pads := Input.get_connected_joypads()
	if pads.is_empty() or not focused:
		pad_id = -1
		status = "No gamepad detected / input unfocused"
		runtime.call("pad_sample", false, PackedInt32Array(), Vector2.ZERO, Vector2.ZERO, Vector2.ZERO)
		return
	var pad: int = pad_id if pads.has(pad_id) else pads[0]
	if pad != pad_id:
		runtime.call("pad_sample", false, PackedInt32Array(), Vector2.ZERO, Vector2.ZERO, Vector2.ZERO)
		pad_id = pad
	# Native bridge maps Godot button IDs to upstream protocol bits.
	var pressed := PackedInt32Array()
	for button in range(JOY_BUTTON_MAX):
		if Input.is_joy_button_pressed(pad, button): pressed.append(button)

	status = "%s · device %d · %d buttons held" % [Input.get_joy_name(pad), pad, pressed.size()]
	runtime.call("pad_sample", true, pressed,
		Vector2(Input.get_joy_axis(pad, JOY_AXIS_LEFT_X), -Input.get_joy_axis(pad, JOY_AXIS_LEFT_Y)),
		Vector2(Input.get_joy_axis(pad, JOY_AXIS_RIGHT_X), -Input.get_joy_axis(pad, JOY_AXIS_RIGHT_Y)),
		Vector2(Input.get_joy_axis(pad, JOY_AXIS_TRIGGER_LEFT), Input.get_joy_axis(pad, JOY_AXIS_TRIGGER_RIGHT)))

