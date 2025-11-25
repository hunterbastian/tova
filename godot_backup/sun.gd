extends DirectionalLight3D

var normal_energy := 2.2
var overcast_energy := 0.8
var is_overcast := false

func toggle_overcast() -> void:
	is_overcast = !is_overcast
	light_energy = overcast_energy if is_overcast else normal_energy

