if gui:get_storage_elapsed(node_index) > 1.0 then
	local fps = gui:get_storage_value1(node_index) / gui:get_storage_elapsed(node_index)
	gui:update_text_of_node(node_index, string.format("FPS: %.1f", fps))
	gui:reset_storage_time(node_index)
	gui:set_storage_value1(node_index, 0.0)
end

local current = gui:get_storage_value1(node_index)
gui:set_storage_value1(node_index, current + 1.0)