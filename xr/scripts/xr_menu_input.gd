extends RefCounted
## OpenXR menu input is separate from the physical Xbox gamepad sampler.
const PRESS_THRESHOLD := 0.65
const RELEASE_THRESHOLD := 0.35
const INITIAL_REPEAT := 0.4
const REPEAT_INTERVAL := 0.12
var controllers: Array[XRController3D] = []
var navigate: Callable
var select: Callable
var back: Callable
var toggle: Callable
var direction: int = 0
var repeat_in: float = 0.0
var armed: bool = false
var previous_select: bool = false
var previous_back: bool = false
var previous_toggle: bool = false

func _init(origin: XROrigin3D, move: Callable, accept: Callable, cancel: Callable, open_menu: Callable) -> void:
	navigate = move
	select = accept
	back = cancel
	toggle = open_menu
	for hand in ["left_hand", "right_hand"]:
		var controller := XRController3D.new()
		controller.tracker = hand
		controller.pose = "menu_pose"
		origin.add_child(controller)
		controllers.append(controller)

func tick(delta: float, focused: bool) -> void:
	var vertical: float = 0.0
	var active: bool = false
	var selected: bool = false
	var backed: bool = false
	var toggled: bool = false
	for controller in controllers:
		if not controller.get_is_active():
			continue
		active = true
		var axis: float = controller.get_vector2("menu_move").y
		if absf(axis) > absf(vertical):
			vertical = axis
		selected = selected or controller.is_button_pressed("menu_select")
		backed = backed or controller.is_button_pressed("menu_back")
		toggled = toggled or controller.is_button_pressed("menu_toggle")
	sample(delta, vertical, selected, backed, toggled, focused and active)

func sample(delta: float, vertical: float, selected: bool, backed: bool, toggled: bool, focused: bool) -> void:
	step(delta, vertical, selected or backed or toggled, focused)
	if armed:
		# Exactly one action per sample; menu has priority over select/back.
		if toggled and not previous_toggle:
			toggle.call()
		elif backed and not previous_back:
			back.call()
		elif selected and not previous_select:
			select.call()
	previous_select = selected
	previous_back = backed
	previous_toggle = toggled

func step(delta: float, vertical: float, held: bool, focused: bool) -> void:
	# Require neutral after focus recovery/disconnect so held controls never activate UI.
	if not focused:
		reset()
		return
	if not armed:
		armed = not held and absf(vertical) < RELEASE_THRESHOLD
		return
	if absf(vertical) < RELEASE_THRESHOLD:
		direction = 0
		repeat_in = 0.0
		return
	if absf(vertical) < PRESS_THRESHOLD:
		return
	var next: int = -1 if vertical > 0.0 else 1
	if direction != next:
		direction = next
		repeat_in = INITIAL_REPEAT
		navigate.call(direction)
	else:
		repeat_in -= delta
		if repeat_in <= 0.0:
			repeat_in = REPEAT_INTERVAL
			navigate.call(direction)

func reset() -> void:
	previous_select = false
	previous_back = false
	previous_toggle = false
	armed = false
	direction = 0
	repeat_in = 0.0
