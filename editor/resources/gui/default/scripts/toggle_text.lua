local current_text = gui:get_node_text(node_index)
if current_text == "On" then
	gui:update_text_of_node(node_index, "Off")
else
	gui:update_text_of_node(node_index, "On")
end