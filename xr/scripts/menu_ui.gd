extends RefCounted
## Shared controller-first menu. Header and build footer never scroll with actions.
const INK := Color("e8eef7")
const MUTED := Color("96a7bc")
const ACCENT := Color("9eea76")
var menu_surface: MeshInstance3D
var menu_viewport: SubViewport
var menu_panel: Control
var status_label: Label
var metrics_label: Label
var page_label: Label
var footer: Label
var scroll: ScrollContainer
var column: VBoxContainer
var buttons: Array[Button] = []
var focus_index: int = -1

func _style(color: Color, border: Color = Color.TRANSPARENT, width: int = 0) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = color
	box.border_color = border
	box.set_border_width_all(width)
	box.set_corner_radius_all(14)
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
	menu_viewport.size = Vector2i(1024, 1024)
	menu_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	owner.add_child(menu_viewport)
	menu_panel = PanelContainer.new()
	menu_panel.size = Vector2(1024, 1024)
	menu_panel.add_theme_stylebox_override("panel", _style(Color("101923"), Color("263649"), 2))
	menu_viewport.add_child(menu_panel)
	var margin := MarginContainer.new()
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 40)
	menu_panel.add_child(margin)
	var layout := VBoxContainer.new()
	layout.add_theme_constant_override("separation", 16)
	margin.add_child(layout)
	layout.add_child(_label("CONSOLE REMOTE PLAY", 20, ACCENT))
	var header := HBoxContainer.new()
	layout.add_child(header)
	var brand := _label("XRbox", 48, INK)
	brand.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(brand)
	page_label = _label("Cinema", 28, MUTED)
	page_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	header.add_child(page_label)
	status_label = _label("Your console. A screen of your own.", 24, INK)
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
	layout.add_child(scroll)
	column = VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", 12)
	scroll.add_child(column)
	add_button(column, "View screen", func(): set_menu_open.call(false))
	add_button(column, "Screen wider", func(): settings.adjust_width(0.2); apply_screen.call())
	add_button(column, "Screen narrower", func(): settings.adjust_width(-0.2); apply_screen.call())
	add_button(column, "Screen farther", func(): settings.adjust_distance(0.2); apply_screen.call())
	add_button(column, "Screen nearer", func(): settings.adjust_distance(-0.2); apply_screen.call())
	add_button(column, "Recenter", recenter)
	add_button(column, "Reset screen", func(): settings.reset(); recenter.call())
	add_button(column, "Quit", func(): owner.get_tree().quit())
	layout.add_child(_label("Stick / D-pad   Move       A / X   Select       B / Y   Back", 20, MUTED))
	footer = _label("XRbox %s  ·  Select + Start + LB + RB / stick click: menu" % ProjectSettings.get_setting("application/config/version"), 18, MUTED)
	layout.add_child(footer)
	menu_surface = MeshInstance3D.new()
	var quad := QuadMesh.new()
	quad.size = Vector2(1.5, 1.5)
	menu_surface.mesh = quad
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.albedo_texture = menu_viewport.get_texture()
	menu_surface.material_override = material
	owner.add_child(menu_surface)

func add_button(parent: Node, text: String, action: Callable) -> void:
	var button := Button.new()
	button.text = text
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.clip_text = true
	button.custom_minimum_size.y = 76
	button.add_theme_font_size_override("font_size", 28)
	button.add_theme_color_override("font_color", INK)
	button.add_theme_color_override("font_focus_color", Color("101923"))
	button.add_theme_stylebox_override("normal", _style(Color("1a2838")))
	button.add_theme_stylebox_override("hover", _style(Color("24384b")))
	button.add_theme_stylebox_override("pressed", _style(Color("76bc57")))
	button.add_theme_stylebox_override("focus", _style(ACCENT, Color("d6ffc0"), 3))
	button.pressed.connect(action)
	parent.add_child(button)
	buttons.append(button)

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
	for child in column.get_children():
		column.remove_child(child)
		child.queue_free()
	buttons.clear()
	focus_index = -1

func set_open(open: bool) -> void:
	menu_surface.visible = open
	menu_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS if open else SubViewport.UPDATE_DISABLED
	if open:
		select_focus(0)

func place(anchor: Transform3D) -> void:
	menu_surface.transform = anchor * Transform3D(Basis.IDENTITY, Vector3(0, 0, -1.4))

func navigate(event: InputEvent) -> void:
	if event.is_action_pressed("ui_down"):
		move(1)
	elif event.is_action_pressed("ui_up"):
		move(-1)
	elif event.is_action_pressed("ui_accept"):
		accept()

func _reveal(button: Button) -> void:
	if is_instance_valid(button) and scroll.is_ancestor_of(button):
		scroll.ensure_control_visible(button)
