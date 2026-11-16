extends RefCounted
## Shared controller-first menu. Header and build footer never scroll with actions.
const INK := Color("f2f5fa")
const MUTED := Color("a2adbd")
const ACCENT := Color("b3edc5")
var menu_surface: MeshInstance3D
var menu_viewport: SubViewport
var menu_panel: Control
var status_label: Label
var metrics_label: Label
var page_label: Label
var title_label: Label
var footer: Label
var scroll: ScrollContainer
var navigation: VBoxContainer
var column: VBoxContainer
var buttons: Array[Button] = []
var focus_index: int = -1
var value_refreshers: Array[Callable] = []

func _style(color: Color, border: Color = Color.TRANSPARENT, width: int = 0) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = color
	box.border_color = border
	box.set_border_width_all(width)
	box.set_corner_radius_all(22)
	box.content_margin_left = 24
	box.content_margin_right = 24
	box.content_margin_top = 16
	box.content_margin_bottom = 16
	return box

func _label(text: String, size: int, color: Color) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", size)
	label.add_theme_color_override("font_color", color)
	return label

func _init(owner: Node, settings: RefCounted, apply_screen: Callable, recenter: Callable, set_menu_open: Callable) -> void:
	menu_viewport = SubViewport.new()
	menu_viewport.size = Vector2i(1536, 1024)
	menu_viewport.transparent_bg = true
	menu_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	owner.add_child(menu_viewport)
	menu_panel = PanelContainer.new()
	menu_panel.size = Vector2(1536, 1024)
	menu_panel.add_theme_stylebox_override("panel", _style(Color("171c25"), Color("3b4454"), 2))
	menu_viewport.add_child(menu_panel)
	var margin := MarginContainer.new()
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 36)
	menu_panel.add_child(margin)
	var layout := VBoxContainer.new()
	layout.add_theme_constant_override("separation", 12)
	margin.add_child(layout)
	layout.add_child(_label("X R B O X    /    YOUR PERSONAL PLAY SPACE", 18, ACCENT))
	var header := HBoxContainer.new()
	layout.add_child(header)
	title_label = _label("Welcome home", 44, INK)
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title_label)
	page_label = _label("Overview", 22, MUTED)
	page_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	header.add_child(page_label)
	status_label = _label("Your Xbox. Your space. Settle in and play.", 24, INK)
	status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status_label.max_lines_visible = 3
	layout.add_child(status_label)
	metrics_label = _label("", 20, MUTED)
	layout.add_child(metrics_label)
	scroll = ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	# Focus changes reveal controls explicitly. Hover/idle must never recenter a page.
	scroll.follow_focus = false
	var body := HBoxContainer.new()
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body.add_theme_constant_override("separation", 40)
	layout.add_child(body)
	navigation = VBoxContainer.new()
	navigation.custom_minimum_size.x = 300
	navigation.add_theme_constant_override("separation", 12)
	body.add_child(navigation)
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	body.add_child(scroll)
	column = VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", 12)
	scroll.add_child(column)
	add_button(column, "View screen", func(): set_menu_open.call(false))
	add_screen_controls(settings, apply_screen)
	add_button(column, "Quit", func(): owner.get_tree().quit())
	layout.add_child(_label("↑ ↓   Navigate        A / X   Select        B / Y   Back", 20, MUTED))
	footer = _label("XRbox %s  ·  Select + Start + LB + RB / stick click: menu" % ProjectSettings.get_setting("application/config/version"), 18, MUTED)
	layout.add_child(footer)
	menu_surface = MeshInstance3D.new()
	var quad := QuadMesh.new()
	quad.size = Vector2(2.1, 1.4)
	menu_surface.mesh = quad
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_texture = menu_viewport.get_texture()
	# Draw solid panel pixels in the opaque pass; discard only the rounded
	# transparent corners. The room dimmer cannot blend over the panel.
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
	material.alpha_scissor_threshold = 0.5
	material.render_priority = 1
	material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR
	menu_surface.material_override = material
	owner.add_child(menu_surface)

func add_background_controls(settings: RefCounted, apply: Callable) -> void:
	if not settings.passthrough_available:
		add_description("Passthrough unavailable in this runtime")
		return
	var toggle := add_button(column, "", func(): settings.passthrough = not settings.passthrough; apply.call())
	var refresh := func(): toggle.text = "Show my room: " + ("On" if settings.passthrough else "Off")
	refresh.call()
	value_refreshers.append(refresh)
	add_adjustment("Room darkness", func(): return "%d%%" % roundi(settings.darkness * 100),
		func(): settings.adjust_darkness(-0.1); apply.call(), func(): settings.adjust_darkness(0.1); apply.call())

func add_button(parent: Node, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.clip_text = true
	button.custom_minimum_size.y = 72
	button.add_theme_font_size_override("font_size", 26)
	button.add_theme_color_override("font_color", INK)
	button.add_theme_color_override("font_focus_color", Color("101923"))
	button.add_theme_stylebox_override("normal", _style(Color("242c38")))
	button.add_theme_stylebox_override("hover", _style(Color("334254")))
	button.add_theme_stylebox_override("pressed", _style(Color("537b63")))
	button.add_theme_stylebox_override("focus", _style(ACCENT, Color("e0ffea"), 3))
	button.pressed.connect(action)
	parent.add_child(button)
	buttons.append(button)
	return button

func add_description(text: String) -> Label:
	var label := _label(text, 24, MUTED)
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(label)
	return label

func select_focus(index: int) -> void:
	if buttons.is_empty():
		return
	var next: int = wrapi(index, 0, buttons.size())
	if next == focus_index and buttons[next].has_focus():
		return
	focus_index = next
	buttons[focus_index].grab_focus()
	_reveal.call_deferred(buttons[focus_index])

func move(direction: int) -> void:
	select_focus(focus_index + direction)

func accept() -> void:
	if focus_index >= 0 and focus_index < buttons.size():
		buttons[focus_index].pressed.emit()

func clear_actions() -> void:
	for child in navigation.get_children():
		navigation.remove_child(child)
		child.queue_free()
	for child in column.get_children():
		column.remove_child(child)
		child.queue_free()
	buttons.clear()
	value_refreshers.clear()
	scroll.scroll_vertical = 0
	focus_index = -1

func set_open(open: bool) -> void:
	menu_surface.visible = open
	menu_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS if open else SubViewport.UPDATE_DISABLED
	if open:
		select_focus(0)

func place(anchor: Transform3D) -> void:
	menu_surface.transform = anchor * Transform3D(Basis.IDENTITY, Vector3(0, -0.08, -1.4))

func navigate(event: InputEvent) -> void:
	if event.is_action_pressed("ui_down"):
		move(1)
	elif event.is_action_pressed("ui_right"):
		move(1)
	elif event.is_action_pressed("ui_left"):
		move(-1)
	elif event.is_action_pressed("ui_up"):
		move(-1)
	elif event.is_action_pressed("ui_accept"):
		accept()

func _reveal(button: Button) -> void:
	if is_instance_valid(button) and button.has_focus() and scroll.is_ancestor_of(button):
		scroll.ensure_control_visible(button)

func add_section(title: String, description: String = "") -> void:
	var heading := _label(title.to_upper(), 18, ACCENT)
	heading.custom_minimum_size.y = 36
	heading.vertical_alignment = VERTICAL_ALIGNMENT_BOTTOM
	column.add_child(heading)
	if not description.is_empty():
		add_description(description)

func add_adjustment(title: String, value: Callable, decrease: Callable, increase: Callable) -> void:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 12)
	column.add_child(row)
	var label := _label("", 24, INK)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	row.add_child(label)
	var refresh := func(): label.text = title + "   " + str(value.call())
	refresh.call()
	value_refreshers.append(refresh)
	var less := add_button(row, "−", func(): decrease.call(); refresh.call())
	var more := add_button(row, "+", func(): increase.call(); refresh.call())
	less.custom_minimum_size.x = 88
	more.custom_minimum_size.x = 88
	less.tooltip_text = "Decrease " + title.to_lower()
	more.tooltip_text = "Increase " + title.to_lower()

func add_screen_controls(settings: RefCounted, apply: Callable) -> void:
	add_section("Screen size", "A floating 16:9 screen, anchored in your space. Changes apply immediately.")
	add_adjustment("Width", func(): return "%.1f m" % settings.width,
		func(): settings.adjust_width(-0.2); apply.call(), func(): settings.adjust_width(0.2); apply.call())
	add_adjustment("Distance", func(): return "%.1f m" % settings.distance,
		func(): settings.adjust_distance(-0.2); apply.call(), func(): settings.adjust_distance(0.2); apply.call())
	add_section("Quick sizes")
	var presets := HBoxContainer.new()
	presets.add_theme_constant_override("separation", 12)
	column.add_child(presets)
	for preset in [["Personal", 1.6, 1.8], ["Lounge", 2.4, 2.0], ["Cinema", 4.0, 3.0]]:
		var choice: Array = preset
		var button := add_button(presets, choice[0], func(): settings.width = choice[1]; settings.distance = choice[2]; apply.call())
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL

func add_placement_controls(settings: RefCounted, apply: Callable, center: Callable) -> void:
	add_section("Placement", "Move your screen to a comfortable position, or center it in front of you.")
	add_adjustment("Height", func(): return "%+.1f m" % settings.height,
		func(): settings.adjust_height(-0.1); apply.call(), func(): settings.adjust_height(0.1); apply.call())
	add_adjustment("Horizontal", func(): return "%+.1f m" % settings.horizontal,
		func(): settings.adjust_horizontal(-0.1); apply.call(), func(): settings.adjust_horizontal(0.1); apply.call())
	add_button(column, "Center screen in front of me", func(): settings.height = 0.0; settings.horizontal = 0.0; center.call())
	add_button(column, "Restore comfortable defaults", func(): settings.reset(); center.call())

func refresh_values() -> void:
	for refresh in value_refreshers:
		refresh.call()

func add_navigation(current: String, go: Callable) -> void:
	var label := _label("YOUR SPACE", 18, MUTED)
	label.custom_minimum_size.y = 44
	navigation.add_child(label)
	for destination in [["home", "Play"], ["display", "Screen"], ["environment", "Environment"], ["account", "Account"], ["diagnostics", "Help & details"]]:
		var target: String = destination[0]
		var button := add_button(navigation, destination[1], func(): go.call(target))
		button.custom_minimum_size.y = 64
		button.add_theme_font_size_override("font_size", 24)
		if current == target or (current == "placement" and target == "display"):
			button.add_theme_stylebox_override("normal", _style(Color("35483e"), Color("668b74"), 1))
	var spacer := Control.new()
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	navigation.add_child(spacer)
	var note := _label("CONSOLE REMOTE PLAY\nOne screen. All your games.", 18, MUTED)
	navigation.add_child(note)
