extends RefCounted
## Menu surface, controls and focus; callers supply actions explicitly.
var menu_surface: MeshInstance3D
var menu_viewport: SubViewport
var menu_panel: Control
var status_label: Label
var metrics_label: Label
var column: VBoxContainer
var buttons: Array[Button] = []
var focus_index: int = 0
func _init(owner: Node, settings: RefCounted, apply_screen: Callable, recenter: Callable, set_menu_open: Callable) -> void:
	menu_viewport = SubViewport.new()
	menu_viewport.size = Vector2i(1024, 1024)
	menu_viewport.transparent_bg = false
	menu_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	owner.add_child(menu_viewport)
	menu_panel = PanelContainer.new()
	menu_panel.size = Vector2(1024, 1024)
	menu_viewport.add_child(menu_panel)
	var margin := MarginContainer.new()
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 40)
	menu_panel.add_child(margin)
	column = VBoxContainer.new()
	column.add_theme_constant_override("separation", 12)
	margin.add_child(column)
	var heading := Label.new()
	heading.text = "XRbox · XR foundation"
	heading.add_theme_font_size_override("font_size", 34)
	column.add_child(heading)
	status_label = Label.new()
	status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status_label.add_theme_font_size_override("font_size", 22)
	column.add_child(status_label)
	metrics_label = Label.new()
	metrics_label.add_theme_font_size_override("font_size", 24)
	column.add_child(metrics_label)
	add_button(column, "View test screen / Back", func(): set_menu_open.call(false))
	add_button(column, "Screen wider", func(): settings.adjust_width(0.2); apply_screen.call())
	add_button(column, "Screen narrower", func(): settings.adjust_width(-0.2); apply_screen.call())
	add_button(column, "Screen farther", func(): settings.adjust_distance(0.2); apply_screen.call())
	add_button(column, "Screen nearer", func(): settings.adjust_distance(-0.2); apply_screen.call())
	add_button(column, "Recenter screen and menu", recenter)
	add_button(column, "Reset screen", func(): settings.reset(); recenter.call())
	add_button(column, "Quit", func(): owner.get_tree().quit())
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
	owner.add_child(menu_surface)

func add_button(parent: Node, text: String, action: Callable) -> void:
	var button := Button.new()
	button.text = text
	button.clip_text = true
	button.custom_minimum_size.y = 60
	button.add_theme_font_size_override("font_size", 26)
	button.pressed.connect(action)
	parent.add_child(button)
	buttons.append(button)

func select_focus(index: int) -> void:
	focus_index = wrapi(index, 0, buttons.size())
	buttons[focus_index].grab_focus()

func clear_actions() -> void:
	for child in column.get_children():
		if child == status_label or child == metrics_label:
			continue
		column.remove_child(child)
		child.queue_free()
	buttons.clear()

func set_open(open: bool) -> void:
	menu_surface.visible = open
	menu_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS if open else SubViewport.UPDATE_DISABLED
	if open:
		select_focus(0)

func place(anchor: Transform3D) -> void:
	menu_surface.transform = anchor * Transform3D(Basis.IDENTITY, Vector3(0, 0, -1.4))

func navigate(event: InputEvent) -> void:
	if event.is_action_pressed("ui_down"):
		select_focus(focus_index + 1)
	elif event.is_action_pressed("ui_up"):
		select_focus(focus_index - 1)
	elif event.is_action_pressed("ui_accept"):
		buttons[focus_index].pressed.emit()
