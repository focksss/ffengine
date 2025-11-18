if gui:get_storage_elapsed(node_index) > 0.1 then
	local x, y, z = controller:get_camera_position()
	gui:update_text_of_node(node_index, string.format("Cam pos: X: %.2f, Y: %.2f, Z: %.2f", x, y, z))
	gui:reset_storage_time(node_index)
end