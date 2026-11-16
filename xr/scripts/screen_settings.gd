extends RefCounted
## Session-only spatial controls. No credential or runtime-service dependencies.

var passthrough_available: bool = false
var passthrough: bool = false
var darkness: float = 0.0

func adjust_darkness(delta: float) -> void:
	darkness = clampf(darkness + delta, 0.0, 1.0)

var width: float = 2.4
var distance: float = 2.0
var height: float = 0.0
var horizontal: float = 0.0

func adjust_height(delta: float) -> void:
	height = clampf(height + delta, -1.5, 1.5)

func adjust_horizontal(delta: float) -> void:
	horizontal = clampf(horizontal + delta, -2.0, 2.0)

func adjust_width(delta: float) -> void:
	width = clampf(width + delta, 1.0, 5.0)

func adjust_distance(delta: float) -> void:
	distance = clampf(distance + delta, 1.0, 5.0)

func reset() -> void:
	width = 2.4
	distance = 2.0
	height = 0.0
	horizontal = 0.0
