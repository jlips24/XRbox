extends RefCounted
## Session-only prototype controls. Does not modify desktop settings or credentials.

var passthrough_available: bool = false
var passthrough: bool = false
var darkness: float = 0.0

func adjust_darkness(delta: float) -> void:
	darkness = clampf(darkness + delta, 0.0, 1.0)

var width: float = 2.4
var distance: float = 2.0

func adjust_width(delta: float) -> void:
	width = clampf(width + delta, 1.0, 5.0)

func adjust_distance(delta: float) -> void:
	distance = clampf(distance + delta, 1.0, 5.0)

func reset() -> void:
	width = 2.4
	distance = 2.0
