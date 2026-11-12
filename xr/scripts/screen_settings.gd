extends RefCounted
## Session-only prototype controls. Does not modify desktop settings or credentials.

var width: float = 2.4
var distance: float = 2.0

func adjust_width(delta: float) -> void:
	width = clampf(width + delta, 1.0, 5.0)

func adjust_distance(delta: float) -> void:
	distance = clampf(distance + delta, 1.0, 5.0)

func reset() -> void:
	width = 2.4
	distance = 2.0
